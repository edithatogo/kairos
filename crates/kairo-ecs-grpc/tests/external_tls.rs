// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "external-time-warp")]
use kairo_ecs_grpc::{
    external_exchange, serve_external_tls, ExternalDispatcher, ExternalTlsIdentity,
    ExternalTlsPeer, MAX_EXTERNAL_PACKET,
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    path::PathBuf,
    process::Command,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use tokio::sync::oneshot;
use tonic::transport::{Certificate, ClientTlsConfig, Endpoint};

static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "cap48-tls-{}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        Self(path)
    }
    fn openssl(&self, args: &[&str]) {
        let output = Command::new("openssl")
            .args(args)
            .current_dir(&self.0)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "openssl failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    fn ca(&self, name: &str) {
        std::fs::write(self.0.join("ca.conf"), "[req]\ndistinguished_name=dn\nx509_extensions=v3_ca\n[dn]\nCN=fixture\n[v3_ca]\nbasicConstraints=critical,CA:TRUE\nkeyUsage=critical,keyCertSign,cRLSign\nsubjectKeyIdentifier=hash\n").unwrap();
        self.openssl(&[
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-nodes",
            "-subj",
            &format!("/CN={name}"),
            "-keyout",
            &format!("{name}.key"),
            "-out",
            &format!("{name}.pem"),
            "-days",
            "1",
            "-config",
            "ca.conf",
        ]);
    }
    fn leaf(&self, name: &str, ca: &str) {
        self.leaf_domain(name, ca, name);
    }
    fn leaf_domain(&self, name: &str, ca: &str, domain: &str) {
        self.openssl(&[
            "req",
            "-new",
            "-newkey",
            "rsa:2048",
            "-nodes",
            "-subj",
            &format!("/CN={name}"),
            "-keyout",
            &format!("{name}.key"),
            "-out",
            &format!("{name}.csr"),
        ]);
        self.openssl(&[
            "pkcs8",
            "-topk8",
            "-nocrypt",
            "-in",
            &format!("{name}.key"),
            "-out",
            &format!("{name}.pk8"),
        ]);
        std::fs::write(self.0.join("leaf.ext"),format!("basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature\nextendedKeyUsage=serverAuth,clientAuth\nsubjectAltName=DNS:{domain}\n")).unwrap();
        self.openssl(&[
            "x509",
            "-req",
            "-in",
            &format!("{name}.csr"),
            "-CA",
            &format!("{ca}.pem"),
            "-CAkey",
            &format!("{ca}.key"),
            "-CAcreateserial",
            "-out",
            &format!("{name}.pem"),
            "-days",
            "1",
            "-extfile",
            "leaf.ext",
        ]);
        self.openssl(&[
            "x509",
            "-in",
            &format!("{name}.pem"),
            "-outform",
            "DER",
            "-out",
            &format!("{name}.der"),
        ]);
    }
    fn read(&self, name: &str) -> Vec<u8> {
        std::fs::read(self.0.join(name)).unwrap()
    }
    fn identity(&self, name: &str, ca: &str) -> ExternalTlsIdentity {
        ExternalTlsIdentity {
            certificate_pem: self.read(&format!("{name}.pem")),
            private_key_pem: self.read(&format!("{name}.pk8")),
            client_ca_pem: self.read(&format!("{ca}.pem")),
        }
    }
    fn hash(&self, name: &str) -> [u8; 32] {
        Sha256::digest(self.read(&format!("{name}.der"))).into()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
struct Running {
    peer: ExternalTlsPeer,
    stop: Option<oneshot::Sender<()>>,
    task: Option<tokio::task::JoinHandle<Result<(), String>>>,
}
impl Running {
    async fn start(f: &Fixture, allowed: BTreeSet<[u8; 32]>, dispatch: ExternalDispatcher) -> Self {
        Self::start_leaf(f, allowed, dispatch, "node-b").await
    }
    async fn start_leaf(
        f: &Fixture,
        allowed: BTreeSet<[u8; 32]>,
        dispatch: ExternalDispatcher,
        leaf: &str,
    ) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = oneshot::channel();
        let identity = f.identity(leaf, "ca-a");
        let task = tokio::spawn(serve_external_tls(
            listener, identity, allowed, dispatch, rx,
        ));
        Self {
            peer: ExternalTlsPeer {
                endpoint: format!("https://{addr}"),
                domain: "node-b".into(),
                ca_pem: f.read("ca-b.pem"),
                certificate_hash: f.hash("node-b"),
            },
            stop: Some(tx),
            task: Some(task),
        }
    }
    async fn close(mut self) {
        self.stop.take().unwrap().send(()).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(3), self.task.take().unwrap())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }
}
impl Drop for Running {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
    }
}
fn fixture() -> Fixture {
    let f = Fixture::new();
    f.ca("ca-a");
    f.ca("ca-b");
    f.ca("ca-c");
    f.leaf("node-a", "ca-a");
    f.leaf("node-b", "ca-b");
    f.leaf("node-c", "ca-c");
    f.leaf("node-unpinned", "ca-a");
    f
}

#[tokio::test]
async fn mutual_tls_binds_actual_leaf_and_preserves_exact_records() {
    let f = fixture();
    let expected = f.hash("node-a");
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&calls);
    let server = Running::start(
        &f,
        BTreeSet::from([expected]),
        Arc::new(move |peer, packet| {
            assert_eq!(peer, expected);
            observed.fetch_add(1, Ordering::SeqCst);
            Ok(vec![packet, b"retained-retirement".to_vec()])
        }),
    )
    .await;
    let payload = b"exact-signed-canonical-record".to_vec();
    let reply = external_exchange(&f.identity("node-a", "ca-b"), &server.peer, payload.clone())
        .await
        .unwrap();
    assert_eq!(reply, vec![payload, b"retained-retirement".to_vec()]);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    server.close().await;
}
#[tokio::test]
async fn tls_and_manifest_negatives_never_reach_accounting_dispatch() {
    let f = fixture();
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&calls);
    let server = Running::start(
        &f,
        BTreeSet::from([f.hash("node-a")]),
        Arc::new(move |_, p| {
            observed.fetch_add(1, Ordering::SeqCst);
            Ok(vec![p])
        }),
    )
    .await;
    let good = f.identity("node-a", "ca-b");
    for bad in [
        f.identity("node-c", "ca-b"),
        f.identity("node-unpinned", "ca-b"),
    ] {
        assert!(external_exchange(&bad, &server.peer, b"record".to_vec())
            .await
            .is_err());
    }
    let mut wrong_ca = server.peer.clone();
    wrong_ca.ca_pem = f.read("ca-c.pem");
    assert!(external_exchange(&good, &wrong_ca, b"record".to_vec())
        .await
        .is_err());
    let mut wrong_name = server.peer.clone();
    wrong_name.domain = "node-c".into();
    assert!(external_exchange(&good, &wrong_name, b"record".to_vec())
        .await
        .is_err());
    // Native TLS probe intentionally omits the client identity, bypassing the
    // public helper's preflight solely to exercise the real server handshake.
    let no_client = Endpoint::from_shared(server.peer.endpoint.clone())
        .unwrap()
        .tls_config(
            ClientTlsConfig::new()
                .domain_name("node-b")
                .ca_certificate(Certificate::from_pem(f.read("ca-b.pem"))),
        )
        .unwrap()
        .connect()
        .await;
    if let Ok(channel) = no_client {
        use tonic::codegen::http;
        let mut grpc = tonic::client::Grpc::new(channel);
        grpc.ready().await.unwrap();
        let request = tonic::Request::new(ProbePacket {
            packet: b"record".to_vec(),
        });
        let result: Result<tonic::Response<ProbeReply>, _> = grpc
            .unary(
                request,
                http::uri::PathAndQuery::from_static("/kairo.ecs.external.v1.Participant/Exchange"),
                probe_codec::Codec::default(),
            )
            .await;
        assert!(result.is_err());
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    server.close().await;
}
#[tokio::test]
async fn bounded_records_reject_oversize_and_shutdown_is_terminal() {
    let f = fixture();
    let server = Running::start(
        &f,
        BTreeSet::from([f.hash("node-a")]),
        Arc::new(|_, _| Ok(vec![vec![1; MAX_EXTERNAL_PACKET]; 5])),
    )
    .await;
    let client = f.identity("node-a", "ca-b");
    assert!(
        external_exchange(&client, &server.peer, vec![0; MAX_EXTERNAL_PACKET + 1])
            .await
            .is_err()
    );
    assert!(external_exchange(&client, &server.peer, b"record".to_vec())
        .await
        .is_err());
    let peer = server.peer.clone();
    server.close().await;
    assert!(external_exchange(&client, &peer, b"record".to_vec())
        .await
        .is_err());
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct ProbePacket {
    #[prost(bytes = "vec", tag = "1")]
    packet: Vec<u8>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct ProbeReply {
    #[prost(bytes = "vec", repeated, tag = "1")]
    packets: Vec<Vec<u8>>,
}
mod probe_codec {
    use std::marker::PhantomData;
    use tonic::codec::{DecodeBuf, Decoder, EncodeBuf, Encoder};
    pub struct Codec(PhantomData<()>);
    impl Default for Codec {
        fn default() -> Self {
            Self(PhantomData)
        }
    }
    pub struct Enc;
    pub struct Dec;
    impl tonic::codec::Codec for Codec {
        type Encode = super::ProbePacket;
        type Decode = super::ProbeReply;
        type Encoder = Enc;
        type Decoder = Dec;
        fn encoder(&mut self) -> Enc {
            Enc
        }
        fn decoder(&mut self) -> Dec {
            Dec
        }
    }
    impl Encoder for Enc {
        type Item = super::ProbePacket;
        type Error = tonic::Status;
        fn encode(&mut self, v: Self::Item, b: &mut EncodeBuf<'_>) -> Result<(), Self::Error> {
            prost::Message::encode(&v, b).map_err(|_| tonic::Status::internal("encode"))
        }
    }
    impl Decoder for Dec {
        type Item = super::ProbeReply;
        type Error = tonic::Status;
        fn decode(&mut self, b: &mut DecodeBuf<'_>) -> Result<Option<Self::Item>, Self::Error> {
            prost::Message::decode(b)
                .map(Some)
                .map_err(|_| tonic::Status::invalid_argument("decode"))
        }
    }
}

#[tokio::test]
async fn same_ca_and_dns_alternate_server_leaf_is_rejected_before_dispatch() {
    let f = fixture();
    f.leaf_domain("node-b-alternate", "ca-b", "node-b");
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&calls);
    let server = Running::start_leaf(
        &f,
        BTreeSet::from([f.hash("node-a")]),
        Arc::new(move |_, p| {
            observed.fetch_add(1, Ordering::SeqCst);
            Ok(vec![p])
        }),
        "node-b-alternate",
    )
    .await;
    assert_ne!(f.hash("node-b-alternate"), server.peer.certificate_hash);
    assert!(external_exchange(
        &f.identity("node-a", "ca-b"),
        &server.peer,
        b"record".to_vec()
    )
    .await
    .is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    // A positive control proves the alternate certificate still passes the same
    // CA, SAN and mutual-authentication checks when explicitly provisioned.
    let mut alternate_peer = server.peer.clone();
    alternate_peer.certificate_hash = f.hash("node-b-alternate");
    assert_eq!(
        external_exchange(
            &f.identity("node-a", "ca-b"),
            &alternate_peer,
            b"record".to_vec()
        )
        .await
        .unwrap(),
        vec![b"record".to_vec()]
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    server.close().await;
}
