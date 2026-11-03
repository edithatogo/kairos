// SPDX-License-Identifier: Apache-2.0
//! Synthetic CAP48 qualification worker. Local stdin controls owned fixtures;
//! peer traffic uses the fixed authenticated runtime and real mutual TLS.
#[cfg(unix)]
mod native {
    use ed25519_dalek::SigningKey;
    use kairo_ecs_grpc::{
        external_exchange, serve_external_tls, ExternalDispatcher, ExternalTlsIdentity,
        ExternalTlsPeer,
    };
    use kairo_ecs_pdes::external::protocol::{digest, Hash};
    use kairo_ecs_pdes::external::{
        ExternalConfiguration, ExternalOptimisticRuntime, PortableModelCodec,
    };
    use kairo_ecs_pdes::{
        LpId, OptimisticLimits, OptimisticProcess, OptimisticRuntime, OptimisticStateError,
        PartitionPlan, RemoteEvent, Tick,
    };
    use serde::{Deserialize, Serialize};
    use serde_json::{json, Value};
    use std::{
        collections::{BTreeMap, BTreeSet},
        io::{BufRead, Write},
        path::Path,
        sync::{Arc, Mutex},
    };
    #[derive(Clone, Serialize, Deserialize)]
    struct Model {
        value: i64,
        rng: u64,
        generation: u64,
        trace: Vec<(u128, Vec<u8>)>,
    }
    impl Model {
        fn new() -> Self {
            Self {
                value: 0,
                rng: 48,
                generation: 1,
                trace: vec![],
            }
        }
    }
    impl OptimisticProcess for Model {
        type Snapshot = Self;
        fn snapshot(&self) -> Self {
            self.clone()
        }
        fn restore(&mut self, s: &Self) -> Result<(), OptimisticStateError> {
            *self = s.clone();
            Ok(())
        }
        fn on_event(&mut self, e: &RemoteEvent) -> Vec<RemoteEvent> {
            self.generation += 1;
            self.rng = self.rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            self.trace.push((e.tick.ticks(), e.event_payload.clone()));
            match e.event_payload.first().copied().unwrap_or(0) {
                1 => vec![RemoteEvent {
                    source_lp: e.dest_lp,
                    dest_lp: LpId(1),
                    tick: Tick::from_ticks(if self.value >= 100 { 25 } else { 30 }),
                    event_payload: vec![3, self.value as u8],
                }],
                2 => {
                    self.value += 100;
                    vec![]
                }
                3 => {
                    self.value += i64::from(e.event_payload[1]);
                    vec![RemoteEvent {
                        source_lp: e.dest_lp,
                        dest_lp: LpId(0),
                        tick: Tick::from_ticks(e.tick.ticks() + 5),
                        event_payload: vec![4],
                    }]
                }
                4 => {
                    self.value += 10;
                    vec![]
                }
                _ => vec![],
            }
        }
    }
    struct Codec;
    impl PortableModelCodec<Model> for Codec {
        fn identity(&self) -> Hash {
            digest(b"cap48-model-v1-complete-rng-generations-trace")
        }
        fn encode_process(&self, p: &Model) -> Result<Vec<u8>, String> {
            serde_json::to_vec(p).map_err(|e| e.to_string())
        }
        fn decode_process(&self, b: &[u8]) -> Result<Model, String> {
            serde_json::from_slice(b).map_err(|e| e.to_string())
        }
        fn encode_snapshot(&self, p: &Model) -> Result<Vec<u8>, String> {
            self.encode_process(p)
        }
        fn decode_snapshot(&self, b: &[u8]) -> Result<Model, String> {
            self.decode_process(b)
        }
    }
    type Runtime = ExternalOptimisticRuntime<Model, Codec>;
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Settings {
        configuration: ExternalConfiguration,
        participant: u32,
        store: String,
        listen: String,
        certificate: String,
        private_key: String,
        client_ca: String,
        peer_endpoint: String,
        peer_domain: String,
        peer_ca: String,
        peer_certificate_hash: Hash,
    }
    fn field<T: serde::de::DeserializeOwned>(v: &Value, k: &str) -> Result<T, String> {
        serde_json::from_value(v.get(k).cloned().ok_or_else(|| format!("missing {k}"))?)
            .map_err(|e| e.to_string())
    }
    fn number(v: &Value, k: &str) -> Result<u128, String> {
        if let Some(n) = v.get(k).and_then(Value::as_u64) {
            return Ok(u128::from(n));
        }
        let s: String = field(v, k)?;
        u128::from_str_radix(&s, 16).map_err(|e| e.to_string())
    }
    fn serial() -> Result<Value, String> {
        let partition = PartitionPlan::from_entities(
            2,
            kairo_ecs_types::SimDuration::from_ticks(1),
            vec![
                kairo_ecs_types::EntityId::new(1, 0),
                kairo_ecs_types::EntityId::new(2, 0),
            ],
        )
        .map_err(|e| format!("{e:?}"))?;
        let mut serial = OptimisticRuntime::new(
            partition,
            BTreeMap::from([(LpId(0), vec![LpId(1)]), (LpId(1), vec![LpId(0)])]),
            BTreeMap::from([(LpId(0), Model::new()), (LpId(1), Model::new())]),
            OptimisticLimits::default(),
        )
        .map_err(|e| format!("{e:?}"))?;
        for (seq, tick, op) in [(0, 20, 1), (1, 10, 2)] {
            serial
                .schedule_initial(
                    seq,
                    RemoteEvent {
                        source_lp: LpId(0),
                        dest_lp: LpId(0),
                        tick: Tick::from_ticks(tick),
                        event_payload: vec![op],
                    },
                )
                .map_err(|e| format!("{e:?}"))?;
        }
        let progress = serial
            .run_until_with_budget(Tick::from_ticks(100), 64)
            .map_err(|e| format!("{e:?}"))?;
        let messages = progress
            .published_messages
            .iter()
            .map(|m| {
                let mut logical = m.logical_id();
                let mut ancestry = Vec::new();
                while let Some((parent, ordinal)) = logical.output_parts() {
                    ancestry.push(kairo_ecs_pdes::external::protocol::Frame {
                        tick: parent.tick().ticks(),
                        source: parent.source_lp().0,
                        ordinal,
                    });
                    logical = parent.logical_id();
                }
                ancestry.reverse();
                let (root_source, root_sequence) = logical.root_parts().unwrap();
                kairo_ecs_pdes::external::protocol::Message {
                    source: m.event().source_lp.0,
                    destination: m.event().dest_lp.0,
                    tick: m.event().tick.ticks(),
                    root_source: root_source.0,
                    root_sequence,
                    ancestry,
                    incarnation: m.incarnation(),
                    anti: false,
                    payload: m.event().event_payload.clone(),
                }
            })
            .collect::<Vec<_>>();
        Ok(
            json!({"messages":messages,"models":[Codec.encode_process(serial.process_at(LpId(0)).unwrap())?,Codec.encode_process(serial.process_at(LpId(1)).unwrap())?]}),
        )
    }
    async fn command(
        v: Value,
        runtime: &Arc<Mutex<Runtime>>,
        identity: &ExternalTlsIdentity,
        peer: &ExternalTlsPeer,
    ) -> Result<Value, String> {
        let op = v.get("op").and_then(Value::as_str).ok_or("op")?;
        if op == "send" {
            let packet: Vec<u8> = field(&v, "packet")?;
            let replies = external_exchange(identity, peer, packet)
                .await
                .map_err(|e| format!("{e:?}"))?;
            if v.get("accept").and_then(Value::as_bool).unwrap_or(true) {
                for p in &replies {
                    runtime
                        .lock()
                        .map_err(|_| "poisoned")?
                        .receive_packet(p)
                        .map_err(|e| format!("{e:?}"))?;
                }
            }
            return Ok(json!({"replies":replies}));
        }
        if op == "sign_fixture" {
            let record: kairo_ecs_pdes::external::protocol::Record = field(&v, "record")?;
            let seed = [u8::try_from(record.issuer + 1).map_err(|e| e.to_string())?; 32];
            let packet =
                kairo_ecs_pdes::external::protocol::sign(&record, &SigningKey::from_bytes(&seed))?;
            return Ok(json!({"packet":packet}));
        }
        if op == "serial" {
            return serial();
        }
        let mut r = runtime.lock().map_err(|_| "poisoned")?;
        let result = match op {
            "snapshot" => {
                json!({"checkpoint":r.checkpoint().map_err(|e|format!("{e:?}"))?,"model":r.model_bytes(v.get("lp").and_then(Value::as_u64).unwrap_or(0) as u32),"generation":r.generation(),"gvt":format!("{:032x}",r.gvt()),"active":r.is_active(),"revision":r.revision().map_err(|e|format!("{e:?}"))?,"counters":r.counters()})
            }
            "pending" => json!({"packets":r.pending_packets().map_err(|e|format!("{e:?}"))?}),
            "propose" => json!({"packet":r.activation_proposal().map_err(|e|format!("{e:?}"))?}),
            "join" => {
                json!({"packet":r.join_activation(&field::<Vec<u8>>(&v,"packet")?).map_err(|e|format!("{e:?}"))?})
            }
            "activate" => {
                r.activate(
                    &field::<Vec<u8>>(&v, "packet")?,
                    &field::<Vec<Vec<u8>>>(&v, "joins")?,
                )
                .map_err(|e| format!("{e:?}"))?;
                json!({})
            }
            "activation" => json!({"decision":r.retained_activation()}),
            "root" => {
                r.schedule_initial(
                    field(&v, "sequence")?,
                    RemoteEvent {
                        source_lp: LpId(field(&v, "source")?),
                        dest_lp: LpId(field(&v, "destination")?),
                        tick: Tick::from_ticks(number(&v, "tick")?),
                        event_payload: field(&v, "payload")?,
                    },
                )
                .map_err(|e| format!("{e:?}"))?;
                json!({})
            }
            "receive" => {
                json!({"replies":r.receive_packet(&field::<Vec<u8>>(&v,"packet")?).map_err(|e|format!("{e:?}"))?})
            }
            "recover" => {
                json!({"replies":r.recover_delivery(&field::<Vec<u8>>(&v,"packet")?).map_err(|e|format!("{e:?}"))?})
            }
            "run" => {
                json!({"used":r.run_until_with_budget(number(&v,"horizon")?,field(&v,"budget")?).map_err(|e|format!("{e:?}"))?})
            }
            "close" => {
                r.close_initial_inputs().map_err(|e| format!("{e:?}"))?;
                json!({})
            }
            "pause" => json!({"packet":r.pause_cut().map_err(|e|format!("{e:?}"))?}),
            "prepare" => {
                json!({"packet":r.prepare_cut(number(&v,"limit")?).map_err(|e|format!("{e:?}"))?})
            }
            "cut" => {
                json!({"packet":r.commit_cut(field(&v,"prepares")?).map_err(|e|format!("{e:?}"))?})
            }
            "recover_cut" => {
                r.recover_cut(&field::<Vec<u8>>(&v, "packet")?)
                    .map_err(|e| format!("{e:?}"))?;
                json!({})
            }
            "cut_decision" => json!({"packet":r.retained_cut_decision()}),
            "fault" => {
                r.qualify_pause_next_commit(field(&v, "after")?);
                json!({})
            }
            _ => return Err("unknown local qualification command".into()),
        };
        Ok(result)
    }
    pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let path = std::env::args().nth(1).ok_or("settings path required")?;
        if path == "--describe" {
            println!(
                "{}",
                json!({"model":Codec.identity(),"public_keys":[SigningKey::from_bytes(&[1;32]).verifying_key().to_bytes().to_vec(),SigningKey::from_bytes(&[2;32]).verifying_key().to_bytes().to_vec()]})
            );
            return Ok(());
        }
        if path == "--serial" {
            println!("{}", serial()?);
            return Ok(());
        }
        let raw = std::fs::read(&path)?;
        if raw.len() > 65536 {
            return Err("settings bound".into());
        }
        let settings: Settings = serde_json::from_slice(&raw)?;
        // Public deterministic application signing keys are synthetic test vectors.
        let seed = [u8::try_from(settings.participant + 1)?; 32];
        let owned = settings
            .configuration
            .members
            .iter()
            .find(|m| m.participant == settings.participant)
            .ok_or("member")?
            .lps
            .clone();
        let runtime = Arc::new(Mutex::new(Runtime::open(
            Path::new(&settings.store),
            settings.configuration.clone(),
            settings.participant,
            SigningKey::from_bytes(&seed),
            Codec,
            owned.into_iter().map(|lp| (lp, Model::new())).collect(),
        )?));
        let identity = ExternalTlsIdentity {
            certificate_pem: std::fs::read(&settings.certificate)?,
            private_key_pem: std::fs::read(&settings.private_key)?,
            client_ca_pem: std::fs::read(&settings.client_ca)?,
        };
        let peer = ExternalTlsPeer {
            endpoint: settings.peer_endpoint,
            domain: settings.peer_domain,
            ca_pem: std::fs::read(&settings.peer_ca)?,
            certificate_hash: settings.peer_certificate_hash,
        };
        let listener = tokio::net::TcpListener::bind(&settings.listen).await?;
        let address = listener.local_addr()?;
        let allowed = settings
            .configuration
            .members
            .iter()
            .map(|m| m.certificate_hash)
            .collect::<BTreeSet<_>>();
        let driver = Arc::clone(&runtime);
        let dispatcher: ExternalDispatcher = Arc::new(move |leaf, packet| {
            driver
                .lock()
                .map_err(|_| "serialized driver poisoned".to_string())?
                .receive_from(leaf, &packet)
                .map_err(|e| e.to_string())
        });
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let server_identity = identity.clone();
        let server = tokio::spawn(async move {
            serve_external_tls(listener, server_identity, allowed, dispatcher, shutdown_rx).await
        });
        println!(
            "{}",
            json!({"ready":true,"pid":std::process::id(),"listen":address.to_string(),"generation":runtime.lock().unwrap().generation()})
        );
        std::io::stdout().flush()?;
        for line in std::io::stdin().lock().lines() {
            let line = line?;
            if line.len() > 1024 * 1024 {
                return Err("command bound".into());
            }
            let value: Value = serde_json::from_str(&line)?;
            if value.get("op").and_then(Value::as_str) == Some("stop") {
                break;
            }
            let result = command(value, &runtime, &identity, &peer).await;
            match result {
                Ok(result) => println!("{}", json!({"ok":true,"result":result})),
                Err(error) => println!("{}", json!({"ok":false,"error":error})),
            };
            std::io::stdout().flush()?;
        }
        let _ = shutdown_tx.send(());
        server.await??;
        Ok(())
    }
}
#[cfg(unix)]
#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    native::run().await
}
#[cfg(not(unix))]
fn main() {
    eprintln!("CAP48 participant storage qualification currently requires Unix file locking");
    std::process::exit(2);
}
