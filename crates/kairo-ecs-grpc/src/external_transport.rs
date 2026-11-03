// SPDX-License-Identifier: Apache-2.0
//! Bounded mutual-TLS transport for opaque external-accounting records.
//!
//! A successful RPC only transports bytes. The fixed accounting driver must
//! authenticate every record and bind its issuer to the supplied TLS leaf hash.
//! The dispatcher is responsible for serializing its engine/store transactions.
use std::collections::BTreeSet;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;

use sha2::{Digest, Sha256};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::codec::{Codec, DecodeBuf, Decoder, EncodeBuf, Encoder};
use tonic::codegen::{http, Body, BoxFuture, Service, StdError};
use tonic::transport::{Certificate, ClientTlsConfig, Endpoint, Identity, Server, ServerTlsConfig};
use tonic::{Request, Response, Status};

/// Maximum size of one canonical signed accounting record.
pub const MAX_EXTERNAL_PACKET: usize = 128 * 1024;
/// Maximum number of returned records in one RPC.
pub const MAX_EXTERNAL_RECORDS: usize = 64;
/// Maximum aggregate payload returned by one RPC.
pub const MAX_EXTERNAL_RESPONSE_BYTES: usize = 4 * MAX_EXTERNAL_PACKET;
const MAX_PEM: usize = 64 * 1024;
const RPC_PATH: &str = "/kairo.ecs.external.v1.Participant/Exchange";
const RPC_TIMEOUT: Duration = Duration::from_secs(10);

/// Private participant TLS identity and the explicitly provisioned client CAs.
///
/// This type deliberately has no Debug implementation that could print a key.
#[derive(Clone)]
pub struct ExternalTlsIdentity {
    pub certificate_pem: Vec<u8>,
    pub private_key_pem: Vec<u8>,
    pub client_ca_pem: Vec<u8>,
}
impl ExternalTlsIdentity {
    fn validate(&self) -> Result<(), String> {
        for bytes in [
            &self.certificate_pem,
            &self.private_key_pem,
            &self.client_ca_pem,
        ] {
            if bytes.is_empty() || bytes.len() > MAX_PEM {
                return Err("missing or oversized TLS identity/trust material".into());
            }
        }
        Ok(())
    }
    fn identity(&self) -> Identity {
        Identity::from_pem(&self.certificate_pem, &self.private_key_pem)
    }
}

/// One provisioned peer. Its CA and DNS identity are mandatory, with no public
/// root fallback. Each participant must have its own constrained private CA.
#[derive(Clone, Debug)]
pub struct ExternalTlsPeer {
    pub endpoint: String,
    pub domain: String,
    pub ca_pem: Vec<u8>,
}
impl ExternalTlsPeer {
    fn validate(&self) -> Result<(), Status> {
        if !self.endpoint.starts_with("https://")
            || self.endpoint.len() > 2048
            || self.domain.is_empty()
            || self.domain.len() > 253
            || self.ca_pem.is_empty()
            || self.ca_pem.len() > MAX_PEM
        {
            return Err(Status::invalid_argument("invalid TLS peer configuration"));
        }
        Ok(())
    }
}

/// The authenticated client leaf hash and exact bytes enter the fixed engine
/// dispatcher. Returned bytes carry no transport-created accounting authority.
pub type ExternalDispatcher =
    Arc<dyn Fn([u8; 32], Vec<u8>) -> Result<Vec<Vec<u8>>, String> + Send + Sync>;

#[derive(Clone, PartialEq, prost::Message)]
struct PacketRequest {
    #[prost(bytes = "vec", tag = "1")]
    packet: Vec<u8>,
}
#[derive(Clone, PartialEq, prost::Message)]
struct PacketResponse {
    #[prost(bytes = "vec", repeated, tag = "1")]
    packets: Vec<Vec<u8>>,
}

fn validate_packets(packets: &[Vec<u8>]) -> Result<(), Status> {
    if packets.len() > MAX_EXTERNAL_RECORDS {
        return Err(Status::resource_exhausted(
            "too many accounting response records",
        ));
    }
    let mut total = 0usize;
    for packet in packets {
        if packet.is_empty() || packet.len() > MAX_EXTERNAL_PACKET {
            return Err(Status::resource_exhausted(
                "accounting response record size",
            ));
        }
        total = total
            .checked_add(packet.len())
            .ok_or_else(|| Status::resource_exhausted("response overflow"))?;
        if total > MAX_EXTERNAL_RESPONSE_BYTES {
            return Err(Status::resource_exhausted(
                "accounting response aggregate size",
            ));
        }
    }
    Ok(())
}

struct PacketCodec<E, D>(PhantomData<(E, D)>);
impl<E, D> Default for PacketCodec<E, D> {
    fn default() -> Self {
        Self(PhantomData)
    }
}
struct PacketEncoder<E>(PhantomData<E>);
struct PacketDecoder<D>(PhantomData<D>);
impl<E: prost::Message + 'static, D: prost::Message + Default + 'static> Codec
    for PacketCodec<E, D>
{
    type Encode = E;
    type Decode = D;
    type Encoder = PacketEncoder<E>;
    type Decoder = PacketDecoder<D>;
    fn encoder(&mut self) -> Self::Encoder {
        PacketEncoder(PhantomData)
    }
    fn decoder(&mut self) -> Self::Decoder {
        PacketDecoder(PhantomData)
    }
}
impl<E: prost::Message> Encoder for PacketEncoder<E> {
    type Item = E;
    type Error = Status;
    fn encode(&mut self, item: E, out: &mut EncodeBuf<'_>) -> Result<(), Status> {
        item.encode(out)
            .map_err(|_| Status::internal("protobuf encoding failed"))
    }
}
impl<D: prost::Message + Default> Decoder for PacketDecoder<D> {
    type Item = D;
    type Error = Status;
    fn decode(&mut self, input: &mut DecodeBuf<'_>) -> Result<Option<D>, Status> {
        D::decode(input)
            .map(Some)
            .map_err(|_| Status::invalid_argument("invalid accounting protobuf frame"))
    }
}

#[derive(Clone)]
struct ParticipantService {
    allowed: Arc<BTreeSet<[u8; 32]>>,
    dispatch: ExternalDispatcher,
}
impl tonic::server::UnaryService<PacketRequest> for ParticipantService {
    type Response = PacketResponse;
    type Future = BoxFuture<Response<PacketResponse>, Status>;
    fn call(&mut self, request: Request<PacketRequest>) -> Self::Future {
        let service = self.clone();
        Box::pin(async move {
            let certificates = request
                .peer_certs()
                .ok_or_else(|| Status::unauthenticated("client certificate required"))?;
            let leaf = certificates
                .first()
                .ok_or_else(|| Status::unauthenticated("client certificate required"))?;
            let hash: [u8; 32] = Sha256::digest(leaf.as_ref()).into();
            if !service.allowed.contains(&hash) {
                return Err(Status::unauthenticated(
                    "client certificate is outside participant manifest",
                ));
            }
            let packet = request.into_inner().packet;
            if packet.is_empty() || packet.len() > MAX_EXTERNAL_PACKET {
                return Err(Status::resource_exhausted("accounting request record size"));
            }
            let packets = (service.dispatch)(hash, packet)
                .map_err(|_| Status::failed_precondition("accounting driver rejected record"))?;
            validate_packets(&packets)?;
            Ok(Response::new(PacketResponse { packets }))
        })
    }
}
#[derive(Clone)]
struct ParticipantServer(ParticipantService);
impl tonic::server::NamedService for ParticipantServer {
    const NAME: &'static str = "kairo.ecs.external.v1.Participant";
}
impl<B> Service<http::Request<B>> for ParticipantServer
where
    B: Body + Send + 'static,
    B::Error: Into<StdError> + Send + 'static,
{
    type Response = http::Response<tonic::body::Body>;
    type Error = std::convert::Infallible;
    type Future = BoxFuture<Self::Response, Self::Error>;
    fn poll_ready(
        &mut self,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }
    fn call(&mut self, request: http::Request<B>) -> Self::Future {
        let service = self.0.clone();
        Box::pin(async move {
            if request.uri().path() != RPC_PATH {
                let mut response = http::Response::new(tonic::body::Body::empty());
                response
                    .headers_mut()
                    .insert("grpc-status", http::HeaderValue::from_static("12"));
                response.headers_mut().insert(
                    "content-type",
                    http::HeaderValue::from_static("application/grpc"),
                );
                return Ok(response);
            }
            let mut grpc =
                tonic::server::Grpc::new(PacketCodec::<PacketResponse, PacketRequest>::default())
                    .max_decoding_message_size(MAX_EXTERNAL_PACKET + 16)
                    .max_encoding_message_size(MAX_EXTERNAL_RESPONSE_BYTES + 1024);
            Ok(grpc.unary(service, request).await)
        })
    }
}

/// Runs a bounded mutually authenticated endpoint until the owned shutdown
/// receiver resolves. Client authentication and exact leaf pinning are required.
/// No compression, automatic retry or public-root configuration is enabled.
pub async fn serve_external_tls(
    listener: TcpListener,
    identity: ExternalTlsIdentity,
    allowed_certificates: BTreeSet<[u8; 32]>,
    dispatcher: ExternalDispatcher,
    shutdown: oneshot::Receiver<()>,
) -> Result<(), String> {
    identity.validate()?;
    if allowed_certificates.is_empty() || allowed_certificates.len() > 64 {
        return Err("invalid participant TLS certificate set".into());
    }
    let tls = ServerTlsConfig::new()
        .identity(identity.identity())
        .client_ca_root(Certificate::from_pem(&identity.client_ca_pem))
        .client_auth_optional(false)
        .timeout(Duration::from_secs(5));
    Server::builder()
        .tls_config(tls)
        .map_err(|e| e.to_string())?
        .concurrency_limit_per_connection(1)
        .max_concurrent_streams(Some(1))
        .timeout(RPC_TIMEOUT)
        .add_service(ParticipantServer(ParticipantService {
            allowed: Arc::new(allowed_certificates),
            dispatch: dispatcher,
        }))
        .serve_with_incoming_shutdown(TcpListenerStream::new(listener), async {
            let _ = shutdown.await;
        })
        .await
        .map_err(|e| e.to_string())
}

/// Sends one exact opaque signed record without retry. Authentication and
/// application rejection remain errors; received bytes still need fixed-driver
/// verification. A disconnect cannot be interpreted as transaction failure.
pub async fn external_exchange(
    identity: &ExternalTlsIdentity,
    peer: &ExternalTlsPeer,
    packet: Vec<u8>,
) -> Result<Vec<Vec<u8>>, Status> {
    identity.validate().map_err(Status::invalid_argument)?;
    peer.validate()?;
    if packet.is_empty() || packet.len() > MAX_EXTERNAL_PACKET {
        return Err(Status::resource_exhausted("accounting request record size"));
    }
    let tls = ClientTlsConfig::new()
        .domain_name(&peer.domain)
        .ca_certificate(Certificate::from_pem(&peer.ca_pem))
        .identity(identity.identity())
        .timeout(Duration::from_secs(5));
    let endpoint = Endpoint::from_shared(peer.endpoint.clone())
        .map_err(|_| Status::invalid_argument("invalid endpoint URI"))?
        .tls_config(tls)
        .map_err(|_| Status::invalid_argument("invalid TLS configuration"))?
        .connect_timeout(Duration::from_secs(5))
        .timeout(RPC_TIMEOUT);
    let channel = endpoint
        .connect()
        .await
        .map_err(|_| Status::unavailable("peer connection or TLS authentication failed"))?;
    let mut grpc = tonic::client::Grpc::new(channel)
        .max_encoding_message_size(MAX_EXTERNAL_PACKET + 16)
        .max_decoding_message_size(MAX_EXTERNAL_RESPONSE_BYTES + 1024);
    grpc.ready()
        .await
        .map_err(|_| Status::unavailable("peer service unavailable"))?;
    let mut request = Request::new(PacketRequest { packet });
    request.set_timeout(RPC_TIMEOUT);
    let response: Response<PacketResponse> = grpc
        .unary(
            request,
            http::uri::PathAndQuery::from_static(RPC_PATH),
            PacketCodec::<PacketRequest, PacketResponse>::default(),
        )
        .await?;
    let packets = response.into_inner().packets;
    validate_packets(&packets)?;
    Ok(packets)
}
