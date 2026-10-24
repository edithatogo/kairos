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
    WireHeartbeatAck, WireMigrationAck, WireMigrationRequest, WireNullMessage, WireNullMessageAck,
    WireOptimisticAck, WireOptimisticEnvelope, WireRemoteEvent,
    ENVELOPE_SCHEMA_VERSION_PROTOBUF_V2,
};
use crate::service::simulation_transport_client::SimulationTransportClient;
use crate::service::simulation_transport_server::{SimulationTransport, SimulationTransportServer};
use crate::socket::parse_endpoint_addr;
use crate::{
    GrpcComponentBlob, GrpcMigrationRequest, GrpcPeer, GrpcTelemetryBatch, GrpcTransportConfig,
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
    migrations: Arc<Mutex<VecDeque<GrpcMigrationRequest>>>,
    _telemetry_batches: Arc<Mutex<VecDeque<GrpcTelemetryBatch>>>,
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
        let wire = request.into_inner();
        if wire.migration_id.is_empty() {
            return Err(Status::invalid_argument("migration_id must not be empty"));
        }
        if wire.components.is_empty() {
            return Err(Status::invalid_argument(
                "components payload must not be empty",
            ));
        }

        let migration = GrpcMigrationRequest {
            entity: EntityId::new(wire.entity_index, wire.entity_generation),
            source_lp: LpId(wire.source_lp),
            dest_lp: LpId(wire.dest_lp),
            migration_id: wire.migration_id.clone(),
            components: wire
                .components
                .into_iter()
                .map(|c| GrpcComponentBlob {
                    component_type_id: c.component_type_id,
                    payload: c.payload,
                })
                .collect(),
        };

        self.migrations.lock().unwrap().push_back(migration);
        Ok(Response::new(WireMigrationAck {
            migration_id: wire.migration_id,
            applied: true,
            message: "entity migration accepted".into(),
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

/// Standards-compliant gRPC transport backed by Tonic HTTP/2 and Prost envelopes.
pub struct TonicTransport {
    local_lp: LpId,
    bound_addr: SocketAddr,
    peers: HashMap<LpId, SocketAddr>,
    config: GrpcTransportConfig,
    inboxes: Arc<Mutex<BTreeMap<LpId, VecDeque<PdesMessage>>>>,
    optimistic_inboxes: Arc<Mutex<BTreeMap<LpId, VecDeque<OptimisticMessage>>>>,
    migrations: Arc<Mutex<VecDeque<GrpcMigrationRequest>>>,
    telemetry_batches: Arc<Mutex<VecDeque<GrpcTelemetryBatch>>>,
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
        let bound_addr = std_listener.local_addr()?;
        drop(std_listener);

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
        let migrations = Arc::new(Mutex::new(VecDeque::new()));
        let telemetry_batches = Arc::new(Mutex::new(VecDeque::new()));
        let heartbeats = Arc::new(Mutex::new(VecDeque::new()));
        let gvt_proposals = Arc::new(Mutex::new(HashMap::new()));
        let decode_failure_count = Arc::new(AtomicUsize::new(0));

        let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);
        let (init_tx, init_rx) = std::sync::mpsc::sync_channel(0);

        let service = SimulationTransportServiceImpl {
            inboxes: Arc::clone(&inboxes),
            optimistic_inboxes: Arc::clone(&optimistic_inboxes),
            migrations: Arc::clone(&migrations),
            _telemetry_batches: Arc::clone(&telemetry_batches),
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
                let _ = init_tx.send(Ok(()));
                let _ = server
                    .serve_with_shutdown(bound_addr, async move {
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
                    generation: 1,
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

    /// Drains received migrations.
    pub fn drain_migrations(&self) -> Vec<GrpcMigrationRequest> {
        let mut migrations = self.migrations.lock().unwrap();
        migrations.drain(..).collect()
    }

    /// Drains received heartbeats.
    pub fn drain_heartbeats(&self) -> Vec<WorkerHeartbeat> {
        let mut heartbeats = self.heartbeats.lock().unwrap();
        heartbeats.drain(..).collect()
    }

    /// Drains received telemetry batches.
    pub fn drain_telemetry_batches(&self) -> Vec<GrpcTelemetryBatch> {
        let mut telemetry = self.telemetry_batches.lock().unwrap();
        telemetry.drain(..).collect()
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

    fn all_reduce_min(&mut self, timestamp: Tick) -> Tick {
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

        let mut min_tick = timestamp;
        let proposals = self.gvt_proposals.lock().unwrap();
        for dest in &peer_ids {
            if let Some(&prop) = proposals.get(&(round, *dest)) {
                if prop < min_tick {
                    min_tick = prop;
                }
            }
        }

        min_tick
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
