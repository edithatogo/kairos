#![forbid(unsafe_code)]

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use kairo_ecs_pdes::{
    LpId, NullMessage, OptimisticMessage, PdesMessage, PdesTransport, RemoteEvent, Tick,
    TransportError,
};
use kairo_ecs_types::EntityId;
use tonic::transport::{Channel, Endpoint, Server};
use tonic::{Request, Response, Status};

use crate::envelope::{
    KairoOptimisticEnvelopeV1, WireComponentBlob, WireGvtDecision, WireGvtProposal, WireHeartbeat,
    WireHeartbeatAck, WireMigrationAck, WireMigrationRequest, WireMigrationSnapshotRequest,
    WireNullMessage, WireNullMessageAck, WireOptimisticAck, WireOptimisticEnvelope,
    WirePendingEvent, WireRemoteEvent, WireTelemetryAck, WireTelemetryBatch, WireTelemetryBatchV2,
    ENVELOPE_SCHEMA_VERSION_PROTOBUF_V2,
};
use crate::service::simulation_transport_client::SimulationTransportClient;
use crate::service::simulation_transport_server::{SimulationTransport, SimulationTransportServer};
use crate::socket::parse_endpoint_addr;
use crate::{
    GrpcComponentBlob, GrpcEntityMigrationSnapshot, GrpcMigrationRequest, GrpcPeer,
    GrpcPendingEventMetadata, GrpcTelemetryBatch, GrpcTransportConfig, GrpcVersionedComponentBlob,
    WorkerHeartbeat, GRPC_PROTOCOL_ID,
};

/// Maximum message size accepted over gRPC (128 KiB).
pub const MAX_GRPC_MESSAGE_BYTES: usize = 128 * 1024;

fn parse_wire_tick(bytes: &[u8]) -> Result<Tick, Status> {
    if bytes.len() != 16 {
        return Err(Status::invalid_argument(format!(
            "invalid tick byte length: expected 16, got {}",
            bytes.len()
        )));
    }
    let mut arr = [0u8; 16];
    arr.copy_from_slice(bytes);
    Ok(Tick::from_ticks(u128::from_be_bytes(arr)))
}

/// Internal service implementing the standard Tonic SimulationTransport trait.
#[derive(Clone)]
struct SimulationTransportServiceImpl {
    inboxes: Arc<Mutex<BTreeMap<LpId, VecDeque<PdesMessage>>>>,
    optimistic_inboxes: Arc<Mutex<BTreeMap<LpId, VecDeque<OptimisticMessage>>>>,
    migrations: Arc<Mutex<MigrationState>>,
    telemetry_batches: Arc<Mutex<TelemetryState>>,
    local_lp: LpId,
    heartbeats: Arc<Mutex<VecDeque<WorkerHeartbeat>>>,
    gvt_proposals: Arc<Mutex<HashMap<(u64, LpId), Tick>>>,
    decode_failure_count: Arc<AtomicUsize>,
}

#[tonic::async_trait]
impl SimulationTransport for SimulationTransportServiceImpl {
    async fn exchange_events(
        &self,
        request: Request<WireRemoteEvent>,
    ) -> Result<Response<WireRemoteEvent>, Status> {
        let wire = request.into_inner();
        let dest = LpId(wire.dest_lp);
        let source = LpId(wire.source_lp);
        let tick = parse_wire_tick(&wire.tick)?;
        let pdes_msg = PdesMessage::Event(RemoteEvent {
            source_lp: source,
            dest_lp: dest,
            tick,
            event_payload: wire.event_payload.clone(),
        });

        let mut inboxes = self.inboxes.lock().unwrap();
        if let Some(queue) = inboxes.get_mut(&dest) {
            queue.push_back(pdes_msg);
            Ok(Response::new(wire))
        } else {
            Err(Status::invalid_argument(format!(
                "unknown destination logical process {dest:?}"
            )))
        }
    }

    async fn send_null_message(
        &self,
        request: Request<WireNullMessage>,
    ) -> Result<Response<WireNullMessageAck>, Status> {
        let wire = request.into_inner();
        let dest = LpId(wire.dest_lp);
        let source = LpId(wire.source_lp);
        let safe_time = parse_wire_tick(&wire.safe_time)?;
        let pdes_msg = PdesMessage::Null(NullMessage {
            source_lp: source,
            dest_lp: dest,
            safe_time,
        });

        let mut inboxes = self.inboxes.lock().unwrap();
        if let Some(queue) = inboxes.get_mut(&dest) {
            queue.push_back(pdes_msg);
            Ok(Response::new(WireNullMessageAck { accepted: true }))
        } else {
            Err(Status::invalid_argument(format!(
                "unknown destination logical process {dest:?}"
            )))
        }
    }

    async fn send_optimistic_envelope(
        &self,
        request: Request<WireOptimisticEnvelope>,
    ) -> Result<Response<WireOptimisticAck>, Status> {
        let wire = request.into_inner();

        // Wire protocol validation
        if wire.protocol_id != GRPC_PROTOCOL_ID {
            self.decode_failure_count.fetch_add(1, Ordering::Relaxed);
            return Err(Status::invalid_argument(format!(
                "protocol ID mismatch: expected {}, got {}",
                GRPC_PROTOCOL_ID, wire.protocol_id
            )));
        }
        if wire.envelope_schema_version != ENVELOPE_SCHEMA_VERSION_PROTOBUF_V2 {
            self.decode_failure_count.fetch_add(1, Ordering::Relaxed);
            return Err(Status::invalid_argument(format!(
                "unsupported envelope schema version: {}",
                wire.envelope_schema_version
            )));
        }

        match KairoOptimisticEnvelopeV1::from_wire(wire) {
            Ok(envelope) => match envelope.to_optimistic_message(GRPC_PROTOCOL_ID) {
                Ok(opt_msg) => {
                    let dest = opt_msg.event().dest_lp;
                    let mut opt_inboxes = self.optimistic_inboxes.lock().unwrap();
                    if let Some(queue) = opt_inboxes.get_mut(&dest) {
                        queue.push_back(opt_msg);
                        Ok(Response::new(WireOptimisticAck {
                            source_lp: envelope.source_lp,
                            dest_lp: envelope.dest_lp,
                            accepted: true,
                            message: "accepted".into(),
                        }))
                    } else {
                        Err(Status::invalid_argument(format!(
                            "unknown destination LP {dest:?}"
                        )))
                    }
                }
                Err(e) => {
                    self.decode_failure_count.fetch_add(1, Ordering::Relaxed);
                    Err(Status::invalid_argument(format!(
                        "optimistic message conversion failed: {e:?}"
                    )))
                }
            },
            Err(e) => {
                self.decode_failure_count.fetch_add(1, Ordering::Relaxed);
                Err(Status::invalid_argument(format!(
                    "wire envelope decode error: {e:?}"
                )))
            }
        }
    }

    async fn migrate_entity(
        &self,
        request: Request<WireMigrationRequest>,
    ) -> Result<Response<WireMigrationAck>, Status> {
        self.accept_snapshot(1, snapshot_from_wire(request.into_inner())?)
    }

    async fn migrate_snapshot(
        &self,
        request: Request<WireMigrationSnapshotRequest>,
    ) -> Result<Response<WireMigrationAck>, Status> {
        let w = request.into_inner();
        if w.version != 2 {
            return Err(Status::invalid_argument(
                "unsupported migration snapshot version",
            ));
        }
        self.accept_snapshot(
            2,
            snapshot_from_wire(
                w.snapshot
                    .ok_or_else(|| Status::invalid_argument("missing migration snapshot"))?,
            )?,
        )
    }
    async fn stream_telemetry(
        &self,
        request: Request<tonic::Streaming<WireTelemetryBatch>>,
    ) -> Result<Response<WireMigrationAck>, Status> {
        let mut input = request.into_inner();
        let mut batches = Vec::new();
        let mut bytes = 0usize;
        while let Some(w) = input.message().await? {
            bytes = bytes
                .checked_add(w.arrow_ipc_payload.len())
                .ok_or_else(|| Status::resource_exhausted("telemetry length overflow"))?;
            if batches.len() >= MAX_SYNC_ITEMS || bytes > MAX_SYNC_BYTES {
                return Err(Status::resource_exhausted("telemetry stream bound"));
            }
            if !self
                .inboxes
                .lock()
                .unwrap()
                .contains_key(&LpId(w.source_lp))
            {
                return Err(Status::invalid_argument("unknown telemetry source"));
            }
            batches.push(legacy_telemetry(w)?);
        }
        let accepted = self.accept_telemetry(batches)?;
        Ok(Response::new(WireMigrationAck {
            migration_id: String::new(),
            applied: false,
            message: format!(
                "{accepted} validated in-memory telemetry batches; not durable/model-applied"
            ),
        }))
    }

    async fn stream_telemetry_v2(
        &self,
        request: Request<tonic::Streaming<WireTelemetryBatchV2>>,
    ) -> Result<Response<WireTelemetryAck>, Status> {
        let mut input = request.into_inner();
        let mut batches = Vec::new();
        let mut bytes = 0usize;
        while let Some(w) = input.message().await? {
            bytes = bytes
                .checked_add(w.arrow_ipc_payload.len())
                .ok_or_else(|| Status::resource_exhausted("telemetry length overflow"))?;
            if batches.len() >= MAX_SYNC_ITEMS || bytes > MAX_SYNC_BYTES {
                return Err(Status::resource_exhausted("telemetry stream bound"));
            }
            if !self
                .inboxes
                .lock()
                .unwrap()
                .contains_key(&LpId(w.source_lp))
            {
                return Err(Status::invalid_argument("unknown telemetry source"));
            }
            batches.push(telemetry_from_wire(w)?);
        }
        let accepted = self.accept_telemetry(batches)?;
        Ok(Response::new(WireTelemetryAck {
            accepted_batches: accepted,
            message: "validated in-memory transport queue; not durable/model-applied".into(),
        }))
    }

    async fn gvt_sync(
        &self,
        request: Request<WireGvtProposal>,
    ) -> Result<Response<WireGvtDecision>, Status> {
        let wire = request.into_inner();
        let min_tick = parse_wire_tick(&wire.local_min_tick)?;
        let mut proposals = self.gvt_proposals.lock().unwrap();
        proposals.insert((wire.round, LpId(wire.source_lp)), min_tick);
        Ok(Response::new(WireGvtDecision {
            gvt_tick: wire.local_min_tick,
            round: wire.round,
        }))
    }

    async fn heartbeat(
        &self,
        request: Request<WireHeartbeat>,
    ) -> Result<Response<WireHeartbeatAck>, Status> {
        let wire = request.into_inner();
        self.heartbeats.lock().unwrap().push_back(WorkerHeartbeat {
            lp_id: LpId(wire.source_lp),
            elapsed_since_last_seen: Duration::from_millis(wire.elapsed_since_last_seen_ms),
        });
        Ok(Response::new(WireHeartbeatAck {
            source_lp: wire.source_lp,
            accepted: true,
            message: "heartbeat accepted".into(),
        }))
    }
}

impl SimulationTransportServiceImpl {
    fn accept_telemetry(&self, batches: Vec<VersionedTelemetryBatch>) -> Result<u32, Status> {
        let mut state = self.telemetry_batches.lock().unwrap();
        let mut added = BTreeMap::new();
        for b in batches {
            let key = (b.source_lp, b.batch_id.clone());
            let previous = added.get(&key).or_else(|| state.accepted.get(&key));
            if let Some(old) = previous {
                if old != &b {
                    return Err(Status::already_exists(
                        "conflicting telemetry batch identity",
                    ));
                }
            } else {
                added.insert(key, b);
            }
        }
        if state.accepted.len() + added.len() > MAX_RETAINED_SYNC {
            return Err(Status::resource_exhausted("telemetry retention bound"));
        }
        let accepted = added.len() as u32;
        for (key, b) in added {
            state.queue.push_back(b.clone());
            state.accepted.insert(key, b);
        }
        Ok(accepted)
    }
    fn accept_snapshot(
        &self,
        version: u32,
        snapshot: GrpcEntityMigrationSnapshot,
    ) -> Result<Response<WireMigrationAck>, Status> {
        validate_snapshot(&snapshot)?;
        if snapshot.dest_lp != self.local_lp
            || !self
                .inboxes
                .lock()
                .unwrap()
                .contains_key(&snapshot.source_lp)
        {
            return Err(Status::invalid_argument("migration routing mismatch"));
        }
        let id = snapshot.migration_id.clone();
        let key = (snapshot.source_lp, id.clone());
        let value = ReceivedMigrationSnapshot { version, snapshot };
        let mut state = self.migrations.lock().unwrap();
        if let Some(old) = state.accepted.get(&key) {
            if old != &value {
                return Err(Status::already_exists("conflicting migration identity"));
            }
        } else {
            if state.accepted.len() >= MAX_RETAINED_SYNC {
                return Err(Status::resource_exhausted("migration retention bound"));
            }
            state.queue.push_back(value.clone());
            state.accepted.insert(key, value);
        }
        Ok(Response::new(WireMigrationAck {
            migration_id: id,
            applied: false,
            message: "validated in-memory transport queue; not durable/model-applied".into(),
        }))
    }
}

/// Standards-compliant gRPC transport backed by Tonic HTTP/2 and Prost envelopes.
pub struct TonicTransport {
    local_lp: LpId,
    bound_addr: SocketAddr,
    peers: HashMap<LpId, SocketAddr>,
    config: GrpcTransportConfig,
    inboxes: Arc<Mutex<BTreeMap<LpId, VecDeque<PdesMessage>>>>,
    optimistic_inboxes: Arc<Mutex<BTreeMap<LpId, VecDeque<OptimisticMessage>>>>,
    migrations: Arc<Mutex<MigrationState>>,
    telemetry_batches: Arc<Mutex<TelemetryState>>,
    heartbeats: Arc<Mutex<VecDeque<WorkerHeartbeat>>>,
    gvt_proposals: Arc<Mutex<HashMap<(u64, LpId), Tick>>>,
    gvt_round: u64,
    decode_failure_count: Arc<AtomicUsize>,
    client_runtime: tokio::runtime::Runtime,
    clients: Mutex<HashMap<LpId, SimulationTransportClient<Channel>>>,
    shutdown_tx: Option<tokio::sync::watch::Sender<bool>>,
    server_handle: Option<JoinHandle<()>>,
}

impl TonicTransport {
    /// Binds a real Tonic gRPC HTTP/2 server and prepares peer client channels.
    pub fn bind(
        local_lp: LpId,
        bind_addr: SocketAddr,
        peers: Vec<GrpcPeer>,
        config: GrpcTransportConfig,
    ) -> Result<Self, std::io::Error> {
        // Pre-bind a standard TCP listener to confirm port availability and obtain assigned ephemeral port
        let std_listener = std::net::TcpListener::bind(bind_addr)?;
        std_listener.set_nonblocking(true)?;
        let bound_addr = std_listener.local_addr()?;

        let mut peer_map = HashMap::new();
        let mut inboxes_init = BTreeMap::new();
        let mut opt_inboxes_init = BTreeMap::new();
        inboxes_init.insert(local_lp, VecDeque::new());
        opt_inboxes_init.insert(local_lp, VecDeque::new());

        for peer in peers {
            let addr = parse_endpoint_addr(&peer.endpoint)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
            peer_map.insert(peer.lp_id, addr);
            inboxes_init.insert(peer.lp_id, VecDeque::new());
            opt_inboxes_init.insert(peer.lp_id, VecDeque::new());
        }

        let inboxes = Arc::new(Mutex::new(inboxes_init));
        let optimistic_inboxes = Arc::new(Mutex::new(opt_inboxes_init));
        let migrations = Arc::new(Mutex::new(MigrationState::default()));
        let telemetry_batches = Arc::new(Mutex::new(TelemetryState::default()));
        let heartbeats = Arc::new(Mutex::new(VecDeque::new()));
        let gvt_proposals = Arc::new(Mutex::new(HashMap::new()));
        let decode_failure_count = Arc::new(AtomicUsize::new(0));

        let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);
        let (init_tx, init_rx) = std::sync::mpsc::sync_channel(0);

        let service = SimulationTransportServiceImpl {
            inboxes: Arc::clone(&inboxes),
            optimistic_inboxes: Arc::clone(&optimistic_inboxes),
            migrations: Arc::clone(&migrations),
            telemetry_batches: Arc::clone(&telemetry_batches),
            local_lp,
            heartbeats: Arc::clone(&heartbeats),
            gvt_proposals: Arc::clone(&gvt_proposals),
            decode_failure_count: Arc::clone(&decode_failure_count),
        };

        let server_handle = thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_io()
                .enable_time()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    let _ = init_tx.send(Err(e));
                    return;
                }
            };

            let server = Server::builder()
                .concurrency_limit_per_connection(64)
                .add_service(
                    SimulationTransportServer::new(service)
                        .max_decoding_message_size(MAX_GRPC_MESSAGE_BYTES),
                );

            rt.block_on(async move {
                let listener = match tokio::net::TcpListener::from_std(std_listener) {
                    Ok(l) => l,
                    Err(e) => {
                        let _ = init_tx.send(Err(e));
                        return;
                    }
                };
                let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);
                let _ = init_tx.send(Ok(()));
                let _ = server
                    .serve_with_incoming_shutdown(incoming, async move {
                        let _ = shutdown_rx.changed().await;
                    })
                    .await;
            });
        });

        match init_rx.recv() {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "tonic server thread failed during startup",
                ));
            }
        }

        let client_runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()?;

        Ok(Self {
            local_lp,
            bound_addr,
            peers: peer_map,
            config,
            inboxes,
            optimistic_inboxes,
            migrations,
            telemetry_batches,
            heartbeats,
            gvt_proposals,
            gvt_round: 0,
            decode_failure_count,
            client_runtime,
            clients: Mutex::new(HashMap::new()),
            shutdown_tx: Some(shutdown_tx),
            server_handle: Some(server_handle),
        })
    }

    pub fn bound_addr(&self) -> SocketAddr {
        self.bound_addr
    }

    pub fn decode_failure_count(&self) -> usize {
        self.decode_failure_count.load(Ordering::Relaxed)
    }

    fn get_or_connect_client(
        &self,
        dest: LpId,
    ) -> Result<SimulationTransportClient<Channel>, TransportError> {
        let mut clients = self.clients.lock().unwrap();
        if let Some(client) = clients.get(&dest) {
            return Ok(client.clone());
        }

        let addr = self
            .peers
            .get(&dest)
            .copied()
            .ok_or(TransportError::UnknownLogicalProcess(dest))?;

        let timeout = self.config.request_timeout;
        let endpoint_str = format!("http://{addr}");

        let client = self.client_runtime.block_on(async move {
            let endpoint = Endpoint::from_shared(endpoint_str)
                .map_err(|_| TransportError::UnknownLogicalProcess(dest))?
                .timeout(timeout)
                .connect_timeout(timeout);

            let channel = endpoint
                .connect()
                .await
                .map_err(|_| TransportError::UnknownLogicalProcess(dest))?;

            Ok::<_, TransportError>(SimulationTransportClient::new(channel))
        })?;

        clients.insert(dest, client.clone());
        Ok(client)
    }

    /// Sends a validated optimistic message envelope over standard gRPC.
    pub fn send_optimistic(
        &self,
        dest: LpId,
        message: &OptimisticMessage,
    ) -> Result<WireOptimisticAck, TransportError> {
        let mut client = self.get_or_connect_client(dest)?;
        let envelope =
            KairoOptimisticEnvelopeV1::from_optimistic_message(message, GRPC_PROTOCOL_ID, dest, 0)
                .map_err(|_| TransportError::UnknownLogicalProcess(dest))?;

        let wire = envelope
            .to_wire()
            .map_err(|_| TransportError::UnknownLogicalProcess(dest))?;

        self.client_runtime.block_on(async move {
            let response = client
                .send_optimistic_envelope(Request::new(wire))
                .await
                .map_err(|_| TransportError::UnknownLogicalProcess(dest))?;

            Ok(response.into_inner())
        })
    }

    /// Sends a migration request over gRPC.
    pub fn send_migration(
        &self,
        dest: LpId,
        req: &GrpcMigrationRequest,
    ) -> Result<WireMigrationAck, TransportError> {
        let mut client = self.get_or_connect_client(dest)?;
        let wire = WireMigrationRequest {
            entity_index: req.entity.index,
            entity_generation: req.entity.generation,
            source_lp: req.source_lp.0,
            dest_lp: req.dest_lp.0,
            migration_id: req.migration_id.clone(),
            components: req
                .components
                .iter()
                .map(|c| WireComponentBlob {
                    component_type_id: c.component_type_id.clone(),
                    generation: 0,
                    payload: c.payload.clone(),
                })
                .collect(),
            pending_events: Vec::new(),
        };

        self.client_runtime.block_on(async move {
            let response = client
                .migrate_entity(Request::new(wire))
                .await
                .map_err(|_| TransportError::UnknownLogicalProcess(dest))?;

            Ok(response.into_inner())
        })
    }

    fn detailed_client(
        &self,
        dest: LpId,
    ) -> Result<SimulationTransportClient<Channel>, GrpcCallFailure> {
        let addr = self
            .peers
            .get(&dest)
            .ok_or_else(|| Status::not_found("unknown logical process"))?;
        let timeout = self.config.request_timeout;
        let uri = format!("http://{addr}");
        self.client_runtime.block_on(async move {
            let endpoint = Endpoint::from_shared(uri)
                .map_err(|_| Status::invalid_argument("invalid peer endpoint"))?;
            let channel = tokio::time::timeout(timeout, endpoint.connect())
                .await
                .map_err(|_| {
                    Status::deadline_exceeded(
                        "peer connection deadline exceeded; partition suspicion only",
                    )
                })?
                .map_err(|e| Status::unavailable(format!("peer connection failed: {e}")))?;
            Ok(SimulationTransportClient::new(channel)
                .max_encoding_message_size(MAX_GRPC_MESSAGE_BYTES)
                .max_decoding_message_size(MAX_GRPC_MESSAGE_BYTES))
        })
    }
    /// Full metadata transport. Queue acceptance is not durable application.
    pub fn send_migration_snapshot(
        &self,
        dest: LpId,
        snapshot: &GrpcEntityMigrationSnapshot,
    ) -> Result<WireMigrationAck, GrpcCallFailure> {
        if snapshot.source_lp != self.local_lp || snapshot.dest_lp != dest {
            return Err(Status::invalid_argument("migration sender/routing mismatch").into());
        }
        let wire = snapshot_to_wire(snapshot)?;
        let mut client = self.detailed_client(dest)?;
        let timeout = self.config.request_timeout;
        self.client_runtime.block_on(async move {
            tokio::time::timeout(
                timeout,
                client.migrate_snapshot(Request::new(WireMigrationSnapshotRequest {
                    version: 2,
                    snapshot: Some(wire),
                })),
            )
            .await
            .map_err(|_| {
                GrpcCallFailure::from(Status::deadline_exceeded(
                    "migration RPC deadline exceeded; partition suspicion only",
                ))
            })?
            .map(|r| r.into_inner())
            .map_err(GrpcCallFailure::from)
        })
    }
    /// Legacy u64 range transport. Full-width ranges must use the v2 method.
    pub fn send_telemetry(
        &self,
        dest: LpId,
        batches: &[GrpcTelemetryBatch],
    ) -> Result<WireMigrationAck, GrpcCallFailure> {
        if batches.len() > MAX_SYNC_ITEMS {
            return Err(Status::resource_exhausted("telemetry stream bound").into());
        }
        let mut wire = Vec::new();
        let mut total = 0usize;
        for b in batches {
            if b.source_lp != self.local_lp {
                return Err(Status::invalid_argument("telemetry sender mismatch").into());
            }
            total += b.arrow_ipc_payload.len();
            if total > MAX_SYNC_BYTES {
                return Err(Status::resource_exhausted("telemetry stream bound").into());
            }
            let w = WireTelemetryBatch {
                source_lp: b.source_lp.0,
                arrow_ipc_payload: b.arrow_ipc_payload.clone(),
                tick_start: u64::try_from(b.tick_start.ticks())
                    .map_err(|_| Status::invalid_argument("legacy tick overflow; use v2"))?,
                tick_end: u64::try_from(b.tick_end.ticks())
                    .map_err(|_| Status::invalid_argument("legacy tick overflow; use v2"))?,
            };
            legacy_telemetry(w.clone())?;
            wire.push(w);
        }
        let mut client = self.detailed_client(dest)?;
        let timeout = self.config.request_timeout;
        self.client_runtime.block_on(async move {
            tokio::time::timeout(timeout, client.stream_telemetry(tokio_stream::iter(wire)))
                .await
                .map_err(|_| {
                    GrpcCallFailure::from(Status::deadline_exceeded(
                        "telemetry RPC deadline exceeded; partition suspicion only",
                    ))
                })?
                .map(|r| r.into_inner())
                .map_err(GrpcCallFailure::from)
        })
    }
    /// One attempt; an incomplete/invalid stream publishes no batches.
    pub fn send_versioned_telemetry(
        &self,
        dest: LpId,
        batches: &[VersionedTelemetryBatch],
    ) -> Result<WireTelemetryAck, GrpcCallFailure> {
        let mut total = 0usize;
        if batches.len() > MAX_SYNC_ITEMS {
            return Err(Status::resource_exhausted("telemetry stream bound").into());
        }
        for b in batches {
            if b.source_lp != self.local_lp {
                return Err(Status::invalid_argument("telemetry sender mismatch").into());
            }
            validate_telemetry(b)?;
            total += b.arrow_ipc_payload.len();
        }
        if total > MAX_SYNC_BYTES {
            return Err(Status::resource_exhausted("telemetry stream bound").into());
        }
        let wire = batches.iter().map(telemetry_to_wire).collect::<Vec<_>>();
        let mut client = self.detailed_client(dest)?;
        let timeout = self.config.request_timeout;
        self.client_runtime.block_on(async move {
            tokio::time::timeout(
                timeout,
                client.stream_telemetry_v2(tokio_stream::iter(wire)),
            )
            .await
            .map_err(|_| {
                GrpcCallFailure::from(Status::deadline_exceeded(
                    "telemetry RPC deadline exceeded; partition suspicion only",
                ))
            })?
            .map(|r| r.into_inner())
            .map_err(GrpcCallFailure::from)
        })
    }
    /// Sends a worker heartbeat over gRPC.
    pub fn send_heartbeat(
        &self,
        dest: LpId,
        hb: &WorkerHeartbeat,
    ) -> Result<WireHeartbeatAck, TransportError> {
        let mut client = self.get_or_connect_client(dest)?;
        let wire = WireHeartbeat {
            source_lp: hb.lp_id.0,
            observed_tick: 0,
            elapsed_since_last_seen_ms: hb.elapsed_since_last_seen.as_millis() as u64,
        };

        self.client_runtime.block_on(async move {
            let response = client
                .heartbeat(Request::new(wire))
                .await
                .map_err(|_| TransportError::UnknownLogicalProcess(dest))?;

            Ok(response.into_inner())
        })
    }

    /// Proposes local GVT minimum to a remote peer for a given consensus round.
    pub fn propose_gvt(
        &self,
        dest: LpId,
        round: u64,
        local_min: Tick,
    ) -> Result<WireGvtDecision, TransportError> {
        let mut client = self.get_or_connect_client(dest)?;
        let wire = WireGvtProposal {
            source_lp: self.local_lp.0,
            local_min_tick: local_min.ticks().to_be_bytes().to_vec(),
            round,
        };

        self.client_runtime.block_on(async move {
            let response = client
                .gvt_sync(Request::new(wire))
                .await
                .map_err(|_| TransportError::UnknownLogicalProcess(dest))?;

            Ok(response.into_inner())
        })
    }

    /// Polls optimistic messages for the local logical process.
    pub fn poll_optimistic(
        &mut self,
        lp_id: LpId,
    ) -> Result<Vec<OptimisticMessage>, TransportError> {
        let mut inboxes = self.optimistic_inboxes.lock().unwrap();
        let queue = inboxes
            .get_mut(&lp_id)
            .ok_or(TransportError::UnknownLogicalProcess(lp_id))?;
        Ok(queue.drain(..).collect())
    }

    /// Compatibility projection; use full snapshots to retain generations/pending data.
    pub fn drain_migrations(&self) -> Vec<GrpcMigrationRequest> {
        self.drain_migration_snapshots()
            .into_iter()
            .map(|r| GrpcMigrationRequest {
                entity: r.snapshot.entity,
                source_lp: r.snapshot.source_lp,
                dest_lp: r.snapshot.dest_lp,
                migration_id: r.snapshot.migration_id,
                components: r
                    .snapshot
                    .components
                    .into_iter()
                    .map(|c| GrpcComponentBlob {
                        component_type_id: c.component_type_id,
                        payload: c.payload,
                    })
                    .collect(),
            })
            .collect()
    }
    pub fn drain_migration_snapshots(&self) -> Vec<ReceivedMigrationSnapshot> {
        self.migrations.lock().unwrap().queue.drain(..).collect()
    }
    /// Drains received heartbeats.
    pub fn drain_heartbeats(&self) -> Vec<WorkerHeartbeat> {
        let mut heartbeats = self.heartbeats.lock().unwrap();
        heartbeats.drain(..).collect()
    }

    /// Compatibility projection without versioned batch identities.
    pub fn drain_telemetry_batches(&self) -> Vec<GrpcTelemetryBatch> {
        self.drain_versioned_telemetry()
            .into_iter()
            .map(|b| GrpcTelemetryBatch {
                source_lp: b.source_lp,
                tick_start: b.tick_start,
                tick_end: b.tick_end,
                arrow_ipc_payload: b.arrow_ipc_payload,
            })
            .collect()
    }
    pub fn drain_versioned_telemetry(&self) -> Vec<VersionedTelemetryBatch> {
        self.telemetry_batches
            .lock()
            .unwrap()
            .queue
            .drain(..)
            .collect()
    }
}

impl PdesTransport for TonicTransport {
    fn knows_lp(&self, lp_id: LpId) -> bool {
        lp_id == self.local_lp || self.peers.contains_key(&lp_id)
    }

    fn send(&mut self, dest: LpId, message: PdesMessage) -> Result<(), TransportError> {
        if !self.knows_lp(dest) {
            return Err(TransportError::UnknownLogicalProcess(dest));
        }

        let (source_lp, message_dest) = match &message {
            PdesMessage::Event(e) => (e.source_lp, e.dest_lp),
            PdesMessage::Null(n) => (n.source_lp, n.dest_lp),
        };

        if !self.knows_lp(source_lp) {
            return Err(TransportError::UnknownLogicalProcess(source_lp));
        }

        if message_dest != dest {
            return Err(TransportError::MessageDestinationMismatch {
                send_dest: dest,
                message_dest,
            });
        }

        // If local LP, enqueue directly
        if dest == self.local_lp {
            let mut inboxes = self.inboxes.lock().unwrap();
            inboxes.entry(dest).or_default().push_back(message);
            return Ok(());
        }

        match message {
            PdesMessage::Event(event) => {
                let mut client = self.get_or_connect_client(dest)?;
                let wire = WireRemoteEvent {
                    source_lp: event.source_lp.0,
                    dest_lp: event.dest_lp.0,
                    tick: event.tick.ticks().to_be_bytes().to_vec(),
                    event_payload: event.event_payload,
                };

                self.client_runtime.block_on(async move {
                    client
                        .exchange_events(Request::new(wire))
                        .await
                        .map_err(|_| TransportError::UnknownLogicalProcess(dest))?;
                    Ok(())
                })
            }
            PdesMessage::Null(null_msg) => {
                let mut client = self.get_or_connect_client(dest)?;
                let wire = WireNullMessage {
                    source_lp: null_msg.source_lp.0,
                    dest_lp: null_msg.dest_lp.0,
                    safe_time: null_msg.safe_time.ticks().to_be_bytes().to_vec(),
                };

                self.client_runtime.block_on(async move {
                    client
                        .send_null_message(Request::new(wire))
                        .await
                        .map_err(|_| TransportError::UnknownLogicalProcess(dest))?;
                    Ok(())
                })
            }
        }
    }

    fn recv(&mut self, lp_id: LpId) -> Result<Vec<PdesMessage>, TransportError> {
        if !self.knows_lp(lp_id) {
            return Err(TransportError::UnknownLogicalProcess(lp_id));
        }

        let mut inboxes = self.inboxes.lock().unwrap();
        Ok(inboxes.entry(lp_id).or_default().drain(..).collect())
    }

    fn barrier(&mut self) {
        // Deterministic gRPC barrier
        for dest in self.peers.keys() {
            let wire = WireHeartbeat {
                source_lp: self.local_lp.0,
                observed_tick: 0,
                elapsed_since_last_seen_ms: 0,
            };
            if let Ok(mut client) = self.get_or_connect_client(*dest) {
                let _ = self
                    .client_runtime
                    .block_on(async move { client.heartbeat(Request::new(wire)).await });
            }
        }
    }

    fn all_reduce_min(&mut self, _timestamp: Tick) -> Tick {
        // Inbox samples and responsive contribution RPCs do not account for
        // messages in flight, retirement obligations or restart fencing.
        // A listener with no configured peers is still open to remote ingress.
        Tick::ZERO
    }
}

impl Drop for TonicTransport {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(true);
        }
        if let Some(handle) = self.server_handle.take() {
            let _ = handle.join();
        }
    }
}

impl TonicTransport {
    /// Diagnostic minimum of reported contributions and current local buffers.
    ///
    /// This is not a consistent distributed cut or a safe reclamation floor.
    /// Traffic can arrive after a sample; retirement and restart obligations are
    /// not included. Never pass this value to fossil collection. The scheduler
    /// trait deliberately returns ZERO until those obligations are proved.
    pub fn reduce_reported_minimum(&mut self, timestamp: Tick) -> Tick {
        let timestamp =
            crate::socket::buffered_minimum(timestamp, &self.inboxes, &self.optimistic_inboxes);
        if self.peers.is_empty() {
            return timestamp;
        }

        let round = self.gvt_round;
        self.gvt_round += 1;

        {
            let mut proposals = self.gvt_proposals.lock().unwrap();
            proposals.insert((round, self.local_lp), timestamp);
        }

        let peer_ids: Vec<LpId> = self.peers.keys().copied().collect();
        for dest in &peer_ids {
            let _ = self.propose_gvt(*dest, round, timestamp);
        }

        let deadline = Instant::now() + self.config.request_timeout;
        while Instant::now() < deadline {
            let all_arrived = {
                let proposals = self.gvt_proposals.lock().unwrap();
                peer_ids
                    .iter()
                    .all(|p| proposals.contains_key(&(round, *p)))
            };
            if all_arrived {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }

        // A missing participant may hold an earlier event than every known proposal.
        let mut proposals = self.gvt_proposals.lock().unwrap();
        proposals.remove(&(round, self.local_lp));
        peer_ids.iter().fold(timestamp, |minimum, peer| {
            minimum.min(proposals.remove(&(round, *peer)).unwrap_or(Tick::ZERO))
        })
    }
}

#[cfg(test)]
mod cut_safety_regressions {
    use super::*;
    use kairo_ecs_pdes::{LogicalEventId, OptimisticAuthority, OptimisticMessageKind};

    #[test]
    fn post_sample_work_is_not_covered_by_a_raw_contribution_round() {
        for optimistic_kind in [
            None,
            Some(OptimisticMessageKind::Positive),
            Some(OptimisticMessageKind::Anti),
        ] {
            let port = || {
                std::net::TcpListener::bind("127.0.0.1:0")
                    .unwrap()
                    .local_addr()
                    .unwrap()
                    .port()
            };
            let (first_port, second_port) = (port(), port());
            let mut first = TonicTransport::bind(
                LpId(1),
                format!("127.0.0.1:{first_port}").parse().unwrap(),
                vec![GrpcPeer {
                    lp_id: LpId(2),
                    endpoint: format!("http://127.0.0.1:{second_port}"),
                }],
                GrpcTransportConfig::default(),
            )
            .unwrap();
            let mut second = TonicTransport::bind(
                LpId(2),
                format!("127.0.0.1:{second_port}").parse().unwrap(),
                vec![GrpcPeer {
                    lp_id: LpId(1),
                    endpoint: format!("http://127.0.0.1:{first_port}"),
                }],
                GrpcTransportConfig::default(),
            )
            .unwrap();
            let worker = thread::spawn(move || {
                let diagnostic = first.reduce_reported_minimum(Tick::from_ticks(500));
                (diagnostic, first)
            });
            // Observe the actual received proposal: A's inbox sample has happened.
            // No sleep or thread scheduling assumption determines delivery order.
            let deadline = Instant::now() + Duration::from_secs(5);
            while !second
                .gvt_proposals
                .lock()
                .unwrap()
                .contains_key(&(0, LpId(1)))
            {
                assert!(
                    Instant::now() < deadline,
                    "first contribution never arrived"
                );
                thread::yield_now();
            }
            let event = RemoteEvent {
                source_lp: LpId(2),
                dest_lp: LpId(1),
                tick: Tick::from_ticks(3),
                event_payload: vec![48],
            };
            if let Some(kind) = optimistic_kind {
                let message = OptimisticMessage::try_from_authority_parts(
                    event,
                    LogicalEventId::root(LpId(2), 48),
                    OptimisticAuthority::Scoped {
                        simulation_namespace: 48,
                        ownership_epoch: 1,
                    },
                    0,
                    kind,
                )
                .unwrap();
                assert!(second.send_optimistic(LpId(1), &message).unwrap().accepted);
            } else {
                second.send(LpId(1), PdesMessage::Event(event)).unwrap();
            }
            // Only now release B's contribution, after tick-3 admission was ACKed.
            second
                .propose_gvt(LpId(1), 0, Tick::from_ticks(300))
                .unwrap();
            let (diagnostic, mut first) = worker.join().unwrap();
            assert_eq!(
                diagnostic,
                Tick::from_ticks(300),
                "the raw sample omitted admitted tick-3 work"
            );
            assert_eq!(
                first.all_reduce_min(Tick::from_ticks(500)),
                Tick::ZERO,
                "the raw contribution cannot authorize a scheduler floor"
            );
            if let Some(kind) = optimistic_kind {
                let accepted = first.poll_optimistic(LpId(1)).unwrap();
                assert_eq!(accepted.len(), 1);
                assert_eq!(accepted[0].event().tick, Tick::from_ticks(3));
                assert_eq!(accepted[0].kind(), kind);
            } else {
                let accepted = first.recv(LpId(1)).unwrap();
                assert!(
                    matches!(&accepted[..], [PdesMessage::Event(event)] if event.tick == Tick::from_ticks(3))
                );
            }
        }
    }
}

/// Versioned transport acceptance only. Applying data belongs to the model's
/// authenticated optimistic event path, never this in-memory RPC queue.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReceivedMigrationSnapshot {
    pub version: u32,
    pub snapshot: GrpcEntityMigrationSnapshot,
}

/// A source-scoped batch identity and exact full-width telemetry range.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VersionedTelemetryBatch {
    pub version: u32,
    pub batch_id: String,
    pub run_id: String,
    pub source_lp: LpId,
    pub tick_start: Tick,
    pub tick_end: Tick,
    pub arrow_ipc_payload: Vec<u8>,
}

/// Classification never proves peer death or authorizes ownership/GVT changes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GrpcFailureKind {
    Permanent,
    Transient,
    PartitionSuspected,
}
/// Retains the actual status, including code and metadata, for caller policy.
#[derive(Clone, Debug)]
pub struct GrpcCallFailure {
    pub status: Status,
    pub kind: GrpcFailureKind,
}
impl From<Status> for GrpcCallFailure {
    fn from(status: Status) -> Self {
        let kind = match status.code() {
            tonic::Code::Unavailable | tonic::Code::ResourceExhausted => GrpcFailureKind::Transient,
            tonic::Code::DeadlineExceeded => GrpcFailureKind::PartitionSuspected,
            tonic::Code::Unknown if has_transport_connectivity_source(&status) => {
                GrpcFailureKind::Transient
            }
            _ => GrpcFailureKind::Permanent,
        };
        Self { status, kind }
    }
}
// Tonic preserves some mid-RPC socket failures as Unknown with a typed
// transport -> hyper -> io source chain. Preserve that original Status, while
// recognizing only concrete connectivity IO beneath a maintained transport error.
// Server Unknown statuses, message text and unrelated IO sources are insufficient.
fn has_transport_connectivity_source(status: &Status) -> bool {
    let mut source = std::error::Error::source(status);
    let mut transport = false;
    // Error sources are user-extensible; bound work even for a cyclic chain.
    for _ in 0..32 {
        let Some(error) = source else {
            return false;
        };
        transport |= error.is::<tonic::transport::Error>();
        if transport
            && error.downcast_ref::<std::io::Error>().is_some_and(|io| {
                matches!(
                    io.kind(),
                    std::io::ErrorKind::ConnectionRefused
                        | std::io::ErrorKind::ConnectionReset
                        | std::io::ErrorKind::ConnectionAborted
                        | std::io::ErrorKind::NotConnected
                        | std::io::ErrorKind::BrokenPipe
                        | std::io::ErrorKind::UnexpectedEof
                )
            })
        {
            return true;
        }
        source = error.source();
    }
    false
}
impl std::fmt::Display for GrpcCallFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.status.fmt(f)
    }
}
impl std::error::Error for GrpcCallFailure {}

const MAX_SYNC_ITEMS: usize = 64;
const MAX_SYNC_BYTES: usize = 512 * 1024;
const MAX_RETAINED_SYNC: usize = 256;

#[derive(Default)]
struct MigrationState {
    queue: VecDeque<ReceivedMigrationSnapshot>,
    accepted: BTreeMap<(LpId, String), ReceivedMigrationSnapshot>,
}
#[derive(Default)]
struct TelemetryState {
    queue: VecDeque<VersionedTelemetryBatch>,
    accepted: BTreeMap<(LpId, String), VersionedTelemetryBatch>,
}

fn validate_snapshot(snapshot: &GrpcEntityMigrationSnapshot) -> Result<(), Status> {
    snapshot
        .validate()
        .map_err(|_| Status::invalid_argument("invalid migration snapshot"))?;
    if snapshot.migration_id.len() > 256
        || snapshot.components.len() > MAX_SYNC_ITEMS
        || snapshot.pending_events.len() > MAX_SYNC_ITEMS
    {
        return Err(Status::resource_exhausted("migration metadata bound"));
    }
    let mut total = 0usize;
    let mut names = std::collections::BTreeSet::new();
    for c in &snapshot.components {
        if c.component_type_id.len() > 128 || !names.insert(&c.component_type_id) {
            return Err(Status::invalid_argument(
                "duplicate/oversized component identity",
            ));
        }
        total = total
            .checked_add(c.payload.len())
            .ok_or_else(|| Status::resource_exhausted("migration length overflow"))?;
    }
    if total > MAX_GRPC_MESSAGE_BYTES / 2
        || snapshot
            .pending_events
            .iter()
            .any(|e| e.payload_bytes > MAX_GRPC_MESSAGE_BYTES / 2)
    {
        return Err(Status::resource_exhausted("migration payload bound"));
    }
    Ok(())
}
fn snapshot_from_wire(w: WireMigrationRequest) -> Result<GrpcEntityMigrationSnapshot, Status> {
    let pending_events = w
        .pending_events
        .into_iter()
        .map(|e| {
            Ok(GrpcPendingEventMetadata {
                source_lp: LpId(e.source_lp),
                dest_lp: LpId(e.dest_lp),
                tick: parse_wire_tick(&e.tick)?,
                sequence: e.sequence,
                payload_bytes: usize::try_from(e.payload_bytes)
                    .map_err(|_| Status::resource_exhausted("pending size overflow"))?,
                anti_message: e.anti_message,
            })
        })
        .collect::<Result<Vec<_>, Status>>()?;
    let value = GrpcEntityMigrationSnapshot {
        entity: EntityId::new(w.entity_index, w.entity_generation),
        source_lp: LpId(w.source_lp),
        dest_lp: LpId(w.dest_lp),
        migration_id: w.migration_id,
        components: w
            .components
            .into_iter()
            .map(|c| GrpcVersionedComponentBlob {
                component_type_id: c.component_type_id,
                generation: c.generation,
                payload: c.payload,
            })
            .collect(),
        pending_events,
    };
    validate_snapshot(&value)?;
    Ok(value)
}
fn snapshot_to_wire(s: &GrpcEntityMigrationSnapshot) -> Result<WireMigrationRequest, Status> {
    validate_snapshot(s)?;
    Ok(WireMigrationRequest {
        entity_index: s.entity.index,
        entity_generation: s.entity.generation,
        source_lp: s.source_lp.0,
        dest_lp: s.dest_lp.0,
        migration_id: s.migration_id.clone(),
        components: s
            .components
            .iter()
            .map(|c| WireComponentBlob {
                component_type_id: c.component_type_id.clone(),
                generation: c.generation,
                payload: c.payload.clone(),
            })
            .collect(),
        pending_events: s
            .pending_events
            .iter()
            .map(|e| WirePendingEvent {
                source_lp: e.source_lp.0,
                dest_lp: e.dest_lp.0,
                tick: e.tick.ticks().to_be_bytes().to_vec(),
                sequence: e.sequence,
                payload_bytes: e.payload_bytes as u64,
                anti_message: e.anti_message,
            })
            .collect(),
    })
}
fn telemetry_from_wire(w: WireTelemetryBatchV2) -> Result<VersionedTelemetryBatch, Status> {
    let batch = VersionedTelemetryBatch {
        version: w.version,
        batch_id: w.batch_id,
        run_id: w.run_id,
        source_lp: LpId(w.source_lp),
        tick_start: parse_wire_tick(&w.tick_start)?,
        tick_end: parse_wire_tick(&w.tick_end)?,
        arrow_ipc_payload: w.arrow_ipc_payload,
    };
    validate_telemetry(&batch)?;
    Ok(batch)
}
fn telemetry_to_wire(b: &VersionedTelemetryBatch) -> WireTelemetryBatchV2 {
    WireTelemetryBatchV2 {
        version: b.version,
        batch_id: b.batch_id.clone(),
        run_id: b.run_id.clone(),
        source_lp: b.source_lp.0,
        tick_start: b.tick_start.ticks().to_be_bytes().to_vec(),
        tick_end: b.tick_end.ticks().to_be_bytes().to_vec(),
        arrow_ipc_payload: b.arrow_ipc_payload.clone(),
    }
}
fn validate_telemetry(b: &VersionedTelemetryBatch) -> Result<(), Status> {
    if b.version != 2
        || b.batch_id.is_empty()
        || b.batch_id.len() > 256
        || b.run_id.is_empty()
        || b.run_id.len() > 128
        || b.tick_start > b.tick_end
    {
        return Err(Status::invalid_argument(
            "invalid versioned telemetry identity/range",
        ));
    }
    if b.arrow_ipc_payload.is_empty() || b.arrow_ipc_payload.len() > MAX_GRPC_MESSAGE_BYTES / 2 {
        return Err(Status::resource_exhausted("telemetry IPC bound"));
    }
    validate_ipc(b)
}
#[cfg(not(feature = "telemetry-ipc"))]
fn validate_ipc(_: &VersionedTelemetryBatch) -> Result<(), Status> {
    Err(Status::failed_precondition(
        "telemetry-ipc feature is required for schema validation",
    ))
}
#[cfg(feature = "telemetry-ipc")]
fn ipc_records(payload: &[u8]) -> Result<Vec<kairo_ecs_arrow::EventLogRecord>, Status> {
    use kairo_ecs_arrow::polars_interop::{
        dataframe_to_records, read_ipc_stream, records_to_dataframe,
    };
    preflight_ipc_stream(payload)?;
    let df = read_ipc_stream(std::io::Cursor::new(payload))
        .map_err(|_| Status::invalid_argument("invalid Arrow IPC stream"))?;
    let expected =
        records_to_dataframe(&[]).map_err(|_| Status::internal("event schema unavailable"))?;
    if df.schema() != expected.schema() || df.height() > 4096 {
        return Err(Status::invalid_argument("telemetry schema/row bound"));
    }
    let records = dataframe_to_records(&df)
        .map_err(|_| Status::invalid_argument("invalid telemetry rows"))?;
    Ok(records)
}
#[cfg(feature = "telemetry-ipc")]
fn decode_ipc(b: &VersionedTelemetryBatch) -> Result<Vec<kairo_ecs_arrow::EventLogRecord>, Status> {
    let records = ipc_records(&b.arrow_ipc_payload)?;
    if records.iter().any(|r| {
        r.schema_version != kairo_ecs_arrow::SCHEMA_VERSION_V2
            || r.run_id != b.run_id
            || r.time_ticks < b.tick_start.ticks()
            || r.time_ticks > b.tick_end.ticks()
    }) {
        return Err(Status::invalid_argument("telemetry row identity/range"));
    }
    Ok(records)
}
#[cfg(feature = "telemetry-ipc")]
fn validate_ipc(b: &VersionedTelemetryBatch) -> Result<(), Status> {
    decode_ipc(b).map(|_| ())
}

/// A standard schema-v2 IPC stream with aligned source identities. IPC's native
/// schema has no source-LP column, so row_sources must accompany its rows.
#[cfg(feature = "telemetry-ipc")]
#[derive(Clone, Debug)]
pub struct MergedTelemetry {
    pub records: Vec<kairo_ecs_arrow::EventLogRecord>,
    pub row_sources: Vec<LpId>,
    pub missing_sources: Vec<LpId>,
    pub arrow_ipc_payload: Vec<u8>,
}
/// Deterministic merge with explicit missing participants and conflict rejection.
#[cfg(feature = "telemetry-ipc")]
pub fn merge_telemetry_batches(
    batches: &[VersionedTelemetryBatch],
    expected_sources: &std::collections::BTreeSet<LpId>,
) -> Result<MergedTelemetry, GrpcCallFailure> {
    use kairo_ecs_arrow::polars_interop::{records_to_dataframe, write_ipc_stream};
    let mut identities = BTreeMap::new();
    let mut events = BTreeMap::new();
    let mut seen = std::collections::BTreeSet::new();
    let mut run = None;
    if batches.len() > MAX_RETAINED_SYNC || expected_sources.len() > 128 {
        return Err(Status::resource_exhausted("telemetry merge bound").into());
    }
    for b in batches {
        validate_telemetry(b)?;
        if !expected_sources.contains(&b.source_lp) {
            return Err(Status::invalid_argument("unknown telemetry participant").into());
        }
        match &run {
            Some(value) if value != &b.run_id => {
                return Err(Status::invalid_argument("mixed telemetry runs").into())
            }
            None => run = Some(b.run_id.clone()),
            _ => {}
        }
        let key = (b.source_lp, b.batch_id.clone());
        if let Some(old) = identities.insert(key, b) {
            if old != b {
                return Err(Status::already_exists("conflicting telemetry batch identity").into());
            }
            continue;
        }
        seen.insert(b.source_lp);
        for record in decode_ipc(b)? {
            let key = (
                b.source_lp,
                record.event_id.index,
                record.event_id.generation,
            );
            if !events.contains_key(&key) && events.len() >= 4096 {
                return Err(Status::resource_exhausted("telemetry merged row bound").into());
            }
            if let Some(old) = events.insert(key, record.clone()) {
                if old != record {
                    return Err(
                        Status::already_exists("conflicting telemetry event identity").into(),
                    );
                }
            }
        }
    }
    let mut ordered = events
        .into_iter()
        .map(|((lp, _, _), r)| (lp, r))
        .collect::<Vec<_>>();
    ordered.sort_by_key(|(lp, r)| {
        (
            r.time_ticks,
            r.priority,
            r.sequence,
            *lp,
            r.event_id.index,
            r.event_id.generation,
        )
    });
    let row_sources = ordered.iter().map(|(lp, _)| *lp).collect();
    let records = ordered.into_iter().map(|(_, r)| r).collect::<Vec<_>>();
    let mut df =
        records_to_dataframe(&records).map_err(|_| Status::internal("telemetry encode schema"))?;
    let mut arrow_ipc_payload = Vec::new();
    write_ipc_stream(&mut df, &mut arrow_ipc_payload)
        .map_err(|_| Status::internal("telemetry encode IPC"))?;
    Ok(MergedTelemetry {
        records,
        row_sources,
        missing_sources: expected_sources.difference(&seen).copied().collect(),
        arrow_ipc_payload,
    })
}

// A bounded frame preflight, not an Arrow decoder: the maintained Polars reader
// still validates the schema and arrays. Reject compression and oversized
// declarations before that reader can allocate from untrusted IPC metadata.
#[cfg(feature = "telemetry-ipc")]
fn preflight_ipc_stream(bytes: &[u8]) -> Result<(), Status> {
    fn bad() -> Status {
        Status::invalid_argument("unsupported or oversized IPC frame metadata")
    }
    fn u32_at(data: &[u8], at: usize) -> Result<u32, Status> {
        Ok(u32::from_le_bytes(
            data.get(at..at.checked_add(4).ok_or_else(bad)?)
                .ok_or_else(bad)?
                .try_into()
                .map_err(|_| bad())?,
        ))
    }
    fn i64_at(data: &[u8], at: usize) -> Result<i64, Status> {
        Ok(i64::from_le_bytes(
            data.get(at..at.checked_add(8).ok_or_else(bad)?)
                .ok_or_else(bad)?
                .try_into()
                .map_err(|_| bad())?,
        ))
    }
    fn field(data: &[u8], table: usize, slot: usize, size: usize) -> Result<Option<usize>, Status> {
        let displacement = u32_at(data, table)? as i32;
        let vt = usize::try_from(
            (table as i64)
                .checked_sub(i64::from(displacement))
                .ok_or_else(bad)?,
        )
        .map_err(|_| bad())?;
        let head = data
            .get(vt..vt.checked_add(4).ok_or_else(bad)?)
            .ok_or_else(bad)?;
        let length = usize::from(u16::from_le_bytes([head[0], head[1]]));
        let object = usize::from(u16::from_le_bytes([head[2], head[3]]));
        if length < 4
            || length % 2 != 0
            || data
                .get(vt..vt.checked_add(length).ok_or_else(bad)?)
                .is_none()
            || data
                .get(table..table.checked_add(object).ok_or_else(bad)?)
                .is_none()
        {
            return Err(bad());
        }
        let relative = 4 + slot * 2;
        if relative + 2 > length {
            return Ok(None);
        }
        let at = vt + relative;
        let offset = usize::from(u16::from_le_bytes([data[at], data[at + 1]]));
        if offset == 0 {
            return Ok(None);
        }
        if offset.checked_add(size).ok_or_else(bad)? > object {
            return Err(bad());
        }
        Ok(Some(table + offset))
    }
    fn indirect(data: &[u8], at: usize) -> Result<usize, Status> {
        let offset = u32_at(data, at)? as usize;
        if offset < 4 {
            return Err(bad());
        }
        let target = at.checked_add(offset).ok_or_else(bad)?;
        if target >= data.len() {
            return Err(bad());
        }
        Ok(target)
    }
    fn batch(data: &[u8], table: usize, body: usize) -> Result<usize, Status> {
        let rows = field(data, table, 0, 8)?
            .map(|p| i64_at(data, p))
            .transpose()?
            .unwrap_or(0);
        if !(0..=4096).contains(&rows) || field(data, table, 3, 4)?.is_some() {
            return Err(bad());
        }
        for slot in [1, 2] {
            if let Some(at) = field(data, table, slot, 4)? {
                let vector = indirect(data, at)?;
                let count = u32_at(data, vector)? as usize;
                if count > 256 {
                    return Err(bad());
                }
                let start = vector + 4;
                let end = start
                    .checked_add(count.checked_mul(16).ok_or_else(bad)?)
                    .ok_or_else(bad)?;
                if data.get(start..end).is_none() {
                    return Err(bad());
                }
                for i in 0..count {
                    let first = i64_at(data, start + i * 16)?;
                    let second = i64_at(data, start + i * 16 + 8)?;
                    if first < 0 || second < 0 {
                        return Err(bad());
                    }
                    if slot == 1 {
                        if first > 4096 || second > first {
                            return Err(bad());
                        }
                    } else if (first as u64).checked_add(second as u64).ok_or_else(bad)?
                        > body as u64
                    {
                        return Err(bad());
                    }
                }
            }
        }
        if let Some(at) = field(data, table, 4, 4)? {
            let vector = indirect(data, at)?;
            let count = u32_at(data, vector)? as usize;
            if count > 256 {
                return Err(bad());
            }
            let begin = vector + 4;
            let end = begin
                .checked_add(count.checked_mul(8).ok_or_else(bad)?)
                .ok_or_else(bad)?;
            if data.get(begin..end).is_none() {
                return Err(bad());
            }
            let mut total = 0i64;
            for i in 0..count {
                let n = i64_at(data, begin + i * 8)?;
                if !(0..=256).contains(&n) {
                    return Err(bad());
                }
                total = total.checked_add(n).ok_or_else(bad)?;
                if total > 256 {
                    return Err(bad());
                }
            }
        }
        Ok(rows as usize)
    }
    let fixed_schema = fixed_ipc_schema()?;
    let mut has_schema = false;
    let mut cursor = 0usize;
    let mut frames = 0;
    let mut record_rows = 0usize;
    while cursor < bytes.len() {
        let first = u32_at(bytes, cursor)?;
        cursor += 4;
        let size = if first == u32::MAX {
            let n = u32_at(bytes, cursor)?;
            cursor += 4;
            n as usize
        } else {
            first as usize
        };
        if size == 0 {
            return if cursor == bytes.len() && has_schema {
                Ok(())
            } else {
                Err(bad())
            };
        }
        frames += 1;
        if frames > 128 || size > MAX_GRPC_MESSAGE_BYTES / 2 {
            return Err(bad());
        }
        let end = cursor.checked_add(size).ok_or_else(bad)?;
        let meta = bytes.get(cursor..end).ok_or_else(bad)?;
        cursor = end;
        let root = u32_at(meta, 0)? as usize;
        let kind = field(meta, root, 1, 1)?.map(|p| meta[p]).unwrap_or(0);
        let body = field(meta, root, 3, 8)?
            .map(|p| i64_at(meta, p))
            .transpose()?
            .unwrap_or(0);
        let body = usize::try_from(body).map_err(|_| bad())?;
        let end = cursor.checked_add(body).ok_or_else(bad)?;
        if body > MAX_GRPC_MESSAGE_BYTES / 2 || end > bytes.len() {
            return Err(bad());
        }
        let header = indirect(meta, field(meta, root, 2, 4)?.ok_or_else(bad)?)?;
        match kind {
            1 if body == 0 && frames == 1 => {
                if meta != fixed_schema {
                    return Err(Status::invalid_argument(
                        "noncanonical telemetry schema profile before decode",
                    ));
                }
                has_schema = true;
            }
            3 if has_schema => {
                if field(meta, root, 4, 4)?.is_some() {
                    return Err(bad());
                }
                record_rows = record_rows
                    .checked_add(batch(meta, header, body)?)
                    .ok_or_else(bad)?;
                if record_rows > 4096 {
                    return Err(bad());
                }
            }
            _ => return Err(bad()),
        }
        cursor = end;
    }
    Err(bad()) // Require a complete stream terminator, never a truncated prefix.
}

#[cfg(not(feature = "telemetry-ipc"))]
fn legacy_telemetry(_: WireTelemetryBatch) -> Result<VersionedTelemetryBatch, Status> {
    Err(Status::failed_precondition(
        "telemetry-ipc feature is required for schema validation",
    ))
}
#[cfg(feature = "telemetry-ipc")]
fn legacy_telemetry(w: WireTelemetryBatch) -> Result<VersionedTelemetryBatch, Status> {
    if w.arrow_ipc_payload.len() > MAX_GRPC_MESSAGE_BYTES / 2 {
        return Err(Status::resource_exhausted("telemetry IPC bound"));
    }
    let records = ipc_records(&w.arrow_ipc_payload)?;
    let run_id = records
        .first()
        .ok_or_else(|| Status::invalid_argument("legacy empty stream lacks run identity"))?
        .run_id
        .clone();
    let batch_id = format!(
        "legacy-v1-{}",
        kairo_ecs_arrow::compute_sha256_hex(&prost::Message::encode_to_vec(&w))
    );
    let b = VersionedTelemetryBatch {
        version: 2,
        batch_id,
        run_id,
        source_lp: LpId(w.source_lp),
        tick_start: Tick::from_ticks(u128::from(w.tick_start)),
        tick_end: Tick::from_ticks(u128::from(w.tick_end)),
        arrow_ipc_payload: w.arrow_ipc_payload,
    };
    validate_telemetry(&b)?;
    Ok(b)
}

/// Canonical data-only payload for the authenticated optimistic event path.
/// This encodes no native ownership/accounting capability.
pub fn encode_migration_snapshot(
    snapshot: &GrpcEntityMigrationSnapshot,
) -> Result<Vec<u8>, GrpcCallFailure> {
    let wire = WireMigrationSnapshotRequest {
        version: 2,
        snapshot: Some(snapshot_to_wire(snapshot)?),
    };
    let bytes = prost::Message::encode_to_vec(&wire);
    if bytes.len() > MAX_GRPC_MESSAGE_BYTES {
        return Err(Status::resource_exhausted("migration frame bound").into());
    }
    Ok(bytes)
}
/// Decodes the canonical v2 data subset. Unknown/duplicate/reordered fields
/// cannot create distinct authenticated payloads for one canonical snapshot.
pub fn decode_migration_snapshot(
    bytes: &[u8],
) -> Result<GrpcEntityMigrationSnapshot, GrpcCallFailure> {
    if bytes.len() > MAX_GRPC_MESSAGE_BYTES {
        return Err(Status::resource_exhausted("migration frame bound").into());
    }
    let wire = <WireMigrationSnapshotRequest as prost::Message>::decode(bytes)
        .map_err(|_| Status::invalid_argument("malformed migration frame"))?;
    if wire.version != 2 {
        return Err(Status::invalid_argument("unsupported migration snapshot version").into());
    }
    let snapshot = snapshot_from_wire(
        wire.snapshot
            .ok_or_else(|| Status::invalid_argument("missing migration snapshot"))?,
    )?;
    if encode_migration_snapshot(&snapshot)? != bytes {
        return Err(Status::invalid_argument("noncanonical migration snapshot").into());
    }
    Ok(snapshot)
}

/// Bind the only admitted flat schema before parsing any untrusted field tree.
#[cfg(feature = "telemetry-ipc")]
fn fixed_ipc_schema() -> Result<&'static [u8], Status> {
    static SCHEMA: std::sync::OnceLock<Result<Vec<u8>, String>> = std::sync::OnceLock::new();
    SCHEMA
        .get_or_init(|| {
            use kairo_ecs_arrow::polars_interop::{records_to_dataframe, write_ipc_stream};
            let mut df = records_to_dataframe(&[]).map_err(|e| e.to_string())?;
            let mut bytes = Vec::new();
            write_ipc_stream(&mut df, &mut bytes).map_err(|e| e.to_string())?;
            let first = u32::from_le_bytes(
                bytes
                    .get(..4)
                    .ok_or("missing fixed schema prefix")?
                    .try_into()
                    .map_err(|_| "fixed schema prefix")?,
            );
            let (prefix, length) = if first == u32::MAX {
                (
                    8,
                    u32::from_le_bytes(
                        bytes
                            .get(4..8)
                            .ok_or("missing fixed schema length")?
                            .try_into()
                            .map_err(|_| "fixed schema length")?,
                    ) as usize,
                )
            } else {
                (4, first as usize)
            };
            Ok(bytes
                .get(prefix..prefix + length)
                .ok_or("truncated fixed schema")?
                .to_vec())
        })
        .as_ref()
        .map(|v| v.as_slice())
        .map_err(|_| Status::internal("fixed telemetry schema unavailable"))
}
