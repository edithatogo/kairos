// SPDX-License-Identifier: Apache-2.0
//! Synthetic entity migration through the authenticated, durable external engine.
//!
//! Optional `sync_listen`, `sync_peers: [{participant, endpoint}]` and
//! `sync_timeout_ms` settings expose a separate trusted-allocation Tonic worker.
//! `sync_snapshot_send` takes `destination` and canonical lowercase
//! `wire_snapshot` hex; `sync_snapshot_drain` returns `{version, wire_snapshot}`.
//! `sync_telemetry_make` takes batch/run/source IDs, 32-digit lowercase hex
//! `tick_start`/`tick_end`, and EventLog-v2 `rows`; it returns a `batch` with real
//! IPC bytes. `sync_telemetry_send` takes `destination` and `batches`;
//! `sync_telemetry_drain` returns `batches`; `sync_telemetry_merge` takes `batches`
//! and `expected_sources`, returning ordered rows, aligned row sources, missing
//! sources and canonical IPC. All synchronization commands retain classified
//! Tonic failures. Queue ACKs do not imply durable storage or model application.
#[cfg(all(unix, feature = "external-fault-injection"))]
#[path = "../../../docs/distributed/track49-fixtures/model.rs"]
mod model;
#[cfg(all(unix, feature = "external-fault-injection"))]
mod worker {
    use super::model;
    use kairo_ecs_grpc::{
        external_exchange, serve_external_tls, ExternalDispatcher, ExternalTlsIdentity,
        ExternalTlsPeer,
    };
    use kairo_ecs_pdes::external::protocol::Hash;
    use kairo_ecs_pdes::external::ExternalConfiguration;
    use serde::Deserialize;
    use serde_json::{json, Value};
    use std::collections::{BTreeMap, BTreeSet};
    use std::io::{BufRead, Write};
    use std::path::Path;
    use std::sync::{Arc, Mutex};
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
        peers: Vec<Peer>,
        #[serde(default)]
        sync_listen: Option<String>,
        #[serde(default)]
        sync_peers: Vec<super::sync_control::Peer>,
        #[serde(default)]
        sync_timeout_ms: Option<u64>,
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Peer {
        participant: u32,
        endpoint: String,
        domain: String,
        ca: String,
        certificate_hash: Hash,
    }
    pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let arg = std::env::args().nth(1).ok_or("settings path required")?;
        if arg == "--describe" {
            println!("{}", model::describe(2));
            return Ok(());
        }
        if arg == "--serial" {
            println!("{}", model::serial(2)?);
            return Ok(());
        }
        let raw = std::fs::read(&arg)?;
        if raw.len() > 65536 {
            return Err("settings bound".into());
        }
        let settings: Settings = serde_json::from_slice(&raw)?;
        // This owned executable selects its provider before constructing TLS.
        rustls::crypto::ring::default_provider()
            .install_default()
            .map_err(|_| "unexpected prior TLS provider")?;
        let sync = match settings.sync_listen {
            Some(listen) => Some(
                super::sync_control::Control::start(
                    settings.participant,
                    listen,
                    settings.sync_peers,
                    settings.sync_timeout_ms.unwrap_or(1000),
                )
                .await?,
            ),
            None if settings.sync_peers.is_empty() => None,
            None => return Err("sync_peers require sync_listen".into()),
        };
        let identity = ExternalTlsIdentity {
            certificate_pem: std::fs::read(&settings.certificate)?,
            private_key_pem: std::fs::read(&settings.private_key)?,
            client_ca_pem: std::fs::read(&settings.client_ca)?,
        };
        let mut peers = BTreeMap::new();
        for p in settings.peers {
            if peers
                .insert(
                    p.participant,
                    ExternalTlsPeer {
                        endpoint: p.endpoint,
                        domain: p.domain,
                        ca_pem: std::fs::read(p.ca)?,
                        certificate_hash: p.certificate_hash,
                    },
                )
                .is_some()
            {
                return Err("duplicate peer".into());
            }
        }
        let allowed = settings
            .configuration
            .members
            .iter()
            .map(|m| m.certificate_hash)
            .collect::<BTreeSet<_>>();
        let runtime = Arc::new(Mutex::new(model::open(
            Path::new(&settings.store),
            settings.configuration.clone(),
            settings.participant,
        )?));
        let listener = tokio::net::TcpListener::bind(&settings.listen).await?;
        let address = listener.local_addr()?;
        let driver = Arc::clone(&runtime);
        let dispatcher: ExternalDispatcher = Arc::new(move |leaf, packet| {
            driver
                .lock()
                .map_err(|_| "driver poisoned".to_string())?
                .receive_from(leaf, &packet)
                .map_err(|e| e.to_string())
        });
        let (tx, rx) = tokio::sync::oneshot::channel();
        let server_id = identity.clone();
        let server = tokio::spawn(async move {
            serve_external_tls(listener, server_id, allowed, dispatcher, rx).await
        });
        println!(
            "{}",
            json!({"ready":true,"pid":std::process::id(),"listen":address,"sync_listen":sync.as_ref().map(|s|s.address),"generation":runtime.lock().unwrap().generation()})
        );
        std::io::stdout().flush()?;
        for line in std::io::stdin().lock().lines() {
            let raw = line?;
            if raw.len() > 1024 * 1024 {
                return Err("owned command bound".into());
            }
            let v: Value = serde_json::from_str(&raw)?;
            let op = v.get("op").and_then(Value::as_str).ok_or("op")?;
            if op == "stop" {
                break;
            }
            if op.starts_with("sync_") {
                let response = match &sync {
                    Some(control) => control.call(v).await?,
                    None => {
                        json!({"ok":false,"error":{"kind":"Permanent","code":"FailedPrecondition","message":"sync_listen required","details":[],"metadata":"{}"}})
                    }
                };
                println!("{response}");
                std::io::stdout().flush()?;
                continue;
            }
            let result: Result<Value, String> = if op == "send" {
                let destination: u32 = model::field(&v, "destination")?;
                let packet: Vec<u8> = model::field(&v, "packet")?;
                match external_exchange(&identity, peers.get(&destination).ok_or("peer")?, packet)
                    .await
                {
                    Err(e) => Err(format!("{e:?}")),
                    Ok(replies) => {
                        let mut result = Ok(json!({"replies":replies}));
                        if v.get("accept").and_then(Value::as_bool).unwrap_or(true) {
                            for packet in &replies {
                                if let Err(e) = runtime.lock().unwrap().receive_from(
                                    settings.configuration.members[destination as usize]
                                        .certificate_hash,
                                    packet,
                                ) {
                                    result = Err(e.to_string());
                                }
                            }
                        }
                        result
                    }
                }
            } else {
                let mut guard = runtime.lock().map_err(|_| "driver poisoned")?;
                model::local(
                    &v,
                    &mut guard,
                    settings.participant,
                    settings.configuration.members.len() as u32,
                )
            };
            let response = match result {
                Ok(result) => json!({"ok":true,"result":result}),
                Err(error) => json!({"ok":false,"error":error}),
            };
            println!("{response}");
            std::io::stdout().flush()?;
        }
        let _ = tx.send(());
        server.await??;
        if let Some(control) = sync {
            control.stop().await?;
        }
        Ok(())
    }
}
#[cfg(all(unix, feature = "external-fault-injection"))]
#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    worker::run().await
}
#[cfg(not(all(unix, feature = "external-fault-injection")))]
fn main() {
    eprintln!("Track49 qualification requires Unix and external-fault-injection");
    std::process::exit(2);
}

// This qualification control surface is plaintext Tonic on an explicitly selected
// trusted allocation. Its RAM queues do not authenticate or apply model state.
// The separate external mTLS engine above remains the model authority.
#[cfg(all(unix, feature = "external-fault-injection"))]
mod sync_control {
    use super::model;
    use kairo_ecs_grpc::{
        decode_migration_snapshot, encode_migration_snapshot, GrpcCallFailure, GrpcPeer,
        GrpcTransportConfig, TonicTransport,
    };
    use kairo_ecs_pdes::LpId;
    use serde::Deserialize;
    use serde_json::{json, Value};
    use std::net::SocketAddr;
    use std::sync::mpsc;
    use std::time::Duration;
    use tokio::sync::oneshot;

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Peer {
        participant: u32,
        endpoint: String,
    }
    type Request = (Value, oneshot::Sender<Value>);
    pub struct Control {
        tx: mpsc::Sender<Request>,
        task: tokio::task::JoinHandle<()>,
        pub address: SocketAddr,
    }
    impl Control {
        pub async fn start(
            participant: u32,
            listen: String,
            peers: Vec<Peer>,
            timeout_ms: u64,
        ) -> Result<Self, String> {
            if timeout_ms == 0 || timeout_ms > 60_000 || peers.len() > 128 {
                return Err("sync timeout/peer bound".into());
            }
            let listen: SocketAddr = listen.parse().map_err(|e| format!("{e}"))?;
            if !listen.ip().is_loopback() {
                return Err("sync fixture listener must use loopback".into());
            }
            let mut seen = std::collections::BTreeSet::new();
            let peers = peers
                .into_iter()
                .map(|p| {
                    if p.participant == participant || !seen.insert(p.participant) {
                        return Err("duplicate/local sync peer".to_string());
                    }
                    Ok(GrpcPeer {
                        lp_id: LpId(p.participant),
                        endpoint: p.endpoint,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            let (tx, rx) = mpsc::channel::<Request>();
            let (ready_tx, ready_rx) = oneshot::channel();
            // Bind, every synchronous RPC and runtime destruction all belong to
            // this blocking owner; no nested Tokio runtime or async Drop occurs.
            let task = tokio::task::spawn_blocking(move || {
                let transport = match TonicTransport::bind(
                    LpId(participant),
                    listen,
                    peers,
                    GrpcTransportConfig {
                        request_timeout: Duration::from_millis(timeout_ms),
                        ..Default::default()
                    },
                ) {
                    Ok(t) => t,
                    Err(e) => {
                        let _ = ready_tx.send(Err(e.to_string()));
                        return;
                    }
                };
                let _ = ready_tx.send(Ok(transport.bound_addr()));
                while let Ok((v, response)) = rx.recv() {
                    let result = command(&transport, &v);
                    let value = match result {
                        Ok(result) => json!({"ok":true,"result":result}),
                        Err(e) => json!({"ok":false,"error":{
                            "kind":format!("{:?}",e.kind),"code":format!("{:?}",e.status.code()),
                            "message":e.status.message(),"details":e.status.details(),
                            "metadata":format!("{:?}",e.status.metadata())
                        }}),
                    };
                    let _ = response.send(value);
                }
            });
            let address = ready_rx.await.map_err(|e| e.to_string())??;
            Ok(Self { tx, task, address })
        }
        pub async fn call(&self, value: Value) -> Result<Value, String> {
            let (tx, rx) = oneshot::channel();
            self.tx.send((value, tx)).map_err(|e| e.to_string())?;
            rx.await.map_err(|e| e.to_string())
        }
        pub async fn stop(self) -> Result<(), String> {
            drop(self.tx);
            self.task.await.map_err(|e| e.to_string())
        }
    }
    fn invalid(message: impl Into<String>) -> GrpcCallFailure {
        tonic::Status::invalid_argument(message.into()).into()
    }
    fn field<T: serde::de::DeserializeOwned>(v: &Value, name: &str) -> Result<T, GrpcCallFailure> {
        model::field(v, name).map_err(invalid)
    }
    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }
    fn unhex(s: &str) -> Result<Vec<u8>, GrpcCallFailure> {
        if s.len() > 256 * 1024
            || !s.len().is_multiple_of(2)
            || !s
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(invalid("bounded lowercase snapshot hex required"));
        }
        s.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|p| {
                u8::from_str_radix(
                    std::str::from_utf8(p).map_err(|e| invalid(e.to_string()))?,
                    16,
                )
                .map_err(|e| invalid(e.to_string()))
            })
            .collect()
    }
    fn command(t: &TonicTransport, v: &Value) -> Result<Value, GrpcCallFailure> {
        match v.get("op").and_then(Value::as_str).unwrap_or("") {
            "sync_snapshot_send" => {
                let snapshot =
                    decode_migration_snapshot(&unhex(&field::<String>(v, "wire_snapshot")?)?)?;
                let ack = t.send_migration_snapshot(LpId(field(v, "destination")?), &snapshot)?;
                Ok(json!({"applied":ack.applied,"message":ack.message}))
            }
            "sync_snapshot_drain" => {
                let snapshots = t.drain_migration_snapshots().into_iter().map(|r| {
                    Ok(json!({"version":r.version,"wire_snapshot":hex(&encode_migration_snapshot(&r.snapshot)?)}))
                }).collect::<Result<Vec<_>,GrpcCallFailure>>()?;
                Ok(json!({"snapshots":snapshots}))
            }
            _ => telemetry_command(t, v),
        }
    }
    #[cfg(not(feature = "telemetry-ipc"))]
    fn telemetry_command(_: &TonicTransport, _: &Value) -> Result<Value, GrpcCallFailure> {
        Err(tonic::Status::failed_precondition("telemetry-ipc feature required").into())
    }
    #[cfg(feature = "telemetry-ipc")]
    fn telemetry_command(t: &TonicTransport, v: &Value) -> Result<Value, GrpcCallFailure> {
        ipc::command(t, v)
    }
    #[cfg(feature = "telemetry-ipc")]
    mod ipc {
        use super::*;
        use kairo_ecs_arrow::polars_interop::{records_to_dataframe, write_ipc_stream};
        use kairo_ecs_arrow::{EventLogRecord, EventStatus, SCHEMA_VERSION_V2};
        use kairo_ecs_grpc::{merge_telemetry_batches, VersionedTelemetryBatch};
        use kairo_ecs_pdes::Tick;
        use kairo_ecs_types::{EntityId, EventId};
        use serde::Serialize;
        #[derive(Deserialize, Serialize)]
        #[serde(deny_unknown_fields)]
        struct Batch {
            version: u32,
            batch_id: String,
            run_id: String,
            source: u32,
            tick_start: String,
            tick_end: String,
            arrow_ipc_payload: Vec<u8>,
        }
        impl Batch {
            fn into_sdk(self) -> Result<VersionedTelemetryBatch, GrpcCallFailure> {
                Ok(VersionedTelemetryBatch {
                    version: self.version,
                    batch_id: self.batch_id,
                    run_id: self.run_id,
                    source_lp: LpId(self.source),
                    tick_start: Tick::from_ticks(
                        model::parse_tick(&self.tick_start).map_err(invalid)?,
                    ),
                    tick_end: Tick::from_ticks(model::parse_tick(&self.tick_end).map_err(invalid)?),
                    arrow_ipc_payload: self.arrow_ipc_payload,
                })
            }
            fn from_sdk(b: VersionedTelemetryBatch) -> Self {
                Self {
                    version: b.version,
                    batch_id: b.batch_id,
                    run_id: b.run_id,
                    source: b.source_lp.0,
                    tick_start: model::tick(b.tick_start.ticks()),
                    tick_end: model::tick(b.tick_end.ticks()),
                    arrow_ipc_payload: b.arrow_ipc_payload,
                }
            }
        }
        #[derive(Deserialize, Serialize)]
        #[serde(deny_unknown_fields)]
        struct Row {
            event_index: u64,
            event_generation: u32,
            entity_index: Option<u64>,
            entity_generation: Option<u32>,
            tick: String,
            priority: i32,
            sequence: u64,
            event_kind: String,
            status: String,
            payload_ref: Option<String>,
        }
        impl Row {
            fn into_record(self, run_id: &str) -> Result<EventLogRecord, GrpcCallFailure> {
                let entity_id = match (self.entity_index, self.entity_generation) {
                    (Some(i), Some(g)) => Some(EntityId::new(i, g)),
                    (None, None) => None,
                    _ => return Err(invalid("entity index/generation must occur together")),
                };
                Ok(EventLogRecord {
                    schema_version: SCHEMA_VERSION_V2,
                    run_id: run_id.into(),
                    event_id: EventId::new(self.event_index, self.event_generation),
                    entity_id,
                    time_ticks: model::parse_tick(&self.tick).map_err(invalid)?,
                    time_scale: "ticks".into(),
                    priority: self.priority,
                    sequence: self.sequence,
                    event_kind: self.event_kind,
                    status: EventStatus::try_from(self.status.as_str())
                        .map_err(|e| invalid(e.to_string()))?,
                    payload_ref: self.payload_ref,
                })
            }
        }
        fn batches(v: &Value) -> Result<Vec<VersionedTelemetryBatch>, GrpcCallFailure> {
            field::<Vec<Batch>>(v, "batches")?
                .into_iter()
                .map(Batch::into_sdk)
                .collect()
        }
        pub(super) fn command(t: &TonicTransport, v: &Value) -> Result<Value, GrpcCallFailure> {
            match v.get("op").and_then(Value::as_str).unwrap_or("") {
                "sync_telemetry_make" => {
                    let run: String = field(v, "run_id")?;
                    let rows: Vec<Row> = field(v, "rows")?;
                    if rows.len() > 4096 {
                        return Err(invalid("telemetry row bound"));
                    }
                    let records = rows
                        .into_iter()
                        .map(|r| r.into_record(&run))
                        .collect::<Result<Vec<_>, _>>()?;
                    let mut frame =
                        records_to_dataframe(&records).map_err(|e| invalid(e.to_string()))?;
                    let mut ipc = Vec::new();
                    write_ipc_stream(&mut frame, &mut ipc).map_err(|e| invalid(e.to_string()))?;
                    let batch = Batch {
                        version: 2,
                        batch_id: field(v, "batch_id")?,
                        run_id: run,
                        source: field(v, "source")?,
                        tick_start: field(v, "tick_start")?,
                        tick_end: field(v, "tick_end")?,
                        arrow_ipc_payload: ipc,
                    };
                    // Reuse the SDK validator without publishing or mutating a queue.
                    let sdk = batch.into_sdk()?;
                    merge_telemetry_batches(
                        std::slice::from_ref(&sdk),
                        &[sdk.source_lp].into_iter().collect(),
                    )?;
                    Ok(json!({"batch":Batch::from_sdk(sdk)}))
                }
                "sync_telemetry_send" => {
                    let ack =
                        t.send_versioned_telemetry(LpId(field(v, "destination")?), &batches(v)?)?;
                    Ok(json!({"accepted_batches":ack.accepted_batches,"message":ack.message}))
                }
                "sync_telemetry_drain" => Ok(
                    json!({"batches":t.drain_versioned_telemetry().into_iter().map(Batch::from_sdk).collect::<Vec<_>>()}),
                ),
                "sync_telemetry_merge" => {
                    let expected: Vec<u32> = field(v, "expected_sources")?;
                    let expected = expected.into_iter().map(LpId).collect();
                    let merged = merge_telemetry_batches(&batches(v)?, &expected)?;
                    let rows = merged.records.iter().map(|r|json!({
                        "run_id":r.run_id,"event_index":r.event_id.index,"event_generation":r.event_id.generation,
                        "entity_index":r.entity_id.map(|e|e.index),"entity_generation":r.entity_id.map(|e|e.generation),
                        "tick":model::tick(r.time_ticks),"priority":r.priority,"sequence":r.sequence,
                        "event_kind":r.event_kind,"status":r.status.as_str(),"payload_ref":r.payload_ref
                    })).collect::<Vec<_>>();
                    Ok(
                        json!({"rows":rows,"row_sources":merged.row_sources.iter().map(|s|s.0).collect::<Vec<_>>(),
                        "missing_sources":merged.missing_sources.iter().map(|s|s.0).collect::<Vec<_>>(),"arrow_ipc_payload":merged.arrow_ipc_payload}),
                    )
                }
                _ => Err(invalid("unknown sync command")),
            }
        }
    }
}
