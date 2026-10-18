#![forbid(unsafe_code)]

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use kairo_ecs_pdes::{
    LpId, NullMessage, OptimisticMessage, PdesMessage, PdesTransport, RemoteEvent, Tick,
    TransportError,
};
use kairo_ecs_types::EntityId;

use crate::{
    GrpcComponentBlob, GrpcMigrationRequest, GrpcPeer, GrpcTelemetryBatch, GrpcTransportConfig,
    KairoOptimisticEnvelopeV1, WorkerHeartbeat, GRPC_PROTOCOL_ID,
};

const MSG_TYPE_EVENT: u8 = 1;
const MSG_TYPE_NULL: u8 = 2;
const MSG_TYPE_OPTIMISTIC: u8 = 3;
const MSG_TYPE_MIGRATION: u8 = 4;
const MSG_TYPE_TELEMETRY: u8 = 5;
const MSG_TYPE_HEARTBEAT: u8 = 6;
const MSG_TYPE_GVT_PROPOSAL: u8 = 7;
const MSG_TYPE_GVT_DECISION: u8 = 8;
const MSG_TYPE_BARRIER: u8 = 9;

/// Maximum framed message size accepted before buffering (128 KiB).
const MAX_FRAME_BYTES: usize = 128 * 1024;

struct InboundConnection {
    stream: TcpStream,
    buf: Vec<u8>,
}

/// Real socket transport that communicates across distinct OS processes or network endpoints over TCP.
pub struct GrpcSocketTransport {
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
    barrier_signals: Arc<Mutex<HashMap<LpId, u64>>>,
    barrier_count: u64,
    shutdown: Arc<AtomicBool>,
    listener_handle: Option<JoinHandle<()>>,
}

impl GrpcSocketTransport {
    /// Binds a real TCP socket listener and launches the inbound message processing worker.
    pub fn bind(
        local_lp: LpId,
        bind_addr: SocketAddr,
        peers: Vec<GrpcPeer>,
        config: GrpcTransportConfig,
    ) -> Result<Self, io::Error> {
        let listener = TcpListener::bind(bind_addr)?;
        listener.set_nonblocking(true)?;
        let bound_addr = listener.local_addr()?;

        let mut peer_map = HashMap::new();
        let mut inboxes_init = BTreeMap::new();
        let mut opt_inboxes_init = BTreeMap::new();
        inboxes_init.insert(local_lp, VecDeque::new());
        opt_inboxes_init.insert(local_lp, VecDeque::new());

        for peer in peers {
            let addr = parse_endpoint_addr(&peer.endpoint)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
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
        let barrier_signals = Arc::new(Mutex::new(HashMap::new()));
        let shutdown = Arc::new(AtomicBool::new(false));

        let inboxes_bg = Arc::clone(&inboxes);
        let opt_inboxes_bg = Arc::clone(&optimistic_inboxes);
        let migrations_bg = Arc::clone(&migrations);
        let telemetry_bg = Arc::clone(&telemetry_batches);
        let heartbeats_bg = Arc::clone(&heartbeats);
        let gvt_bg = Arc::clone(&gvt_proposals);
        let barrier_bg = Arc::clone(&barrier_signals);
        let shutdown_bg = Arc::clone(&shutdown);

        let listener_handle = thread::spawn(move || {
            let mut conns: Vec<InboundConnection> = Vec::new();
            let mut chunk = [0u8; 4096];

            while !shutdown_bg.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let _ = stream.set_nonblocking(true);
                        conns.push(InboundConnection {
                            stream,
                            buf: Vec::new(),
                        });
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {}
                    Err(_) => break,
                }

                let mut i = 0;
                while i < conns.len() {
                    let mut eof = false;
                    let mut malformed = false;

                    loop {
                        if conns[i].buf.len() > MAX_FRAME_BYTES + 4 {
                            malformed = true;
                            break;
                        }
                        match conns[i].stream.read(&mut chunk) {
                            Ok(0) => {
                                eof = true;
                                break;
                            }
                            Ok(n) => {
                                conns[i].buf.extend_from_slice(&chunk[..n]);
                                if conns[i].buf.len() >= 4 {
                                    let announced =
                                        u32::from_be_bytes(conns[i].buf[0..4].try_into().unwrap())
                                            as usize;
                                    if announced == 0 || announced > MAX_FRAME_BYTES {
                                        malformed = true;
                                        break;
                                    }
                                }
                            }
                            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                                break;
                            }
                            Err(_) => {
                                eof = true;
                                break;
                            }
                        }
                    }

                    while !malformed && conns[i].buf.len() >= 4 {
                        let total_len =
                            u32::from_be_bytes(conns[i].buf[0..4].try_into().unwrap()) as usize;
                        if total_len == 0 || total_len > MAX_FRAME_BYTES {
                            malformed = true;
                            break;
                        }
                        if conns[i].buf.len() < 4 + total_len {
                            break;
                        }

                        let msg_type = conns[i].buf[4];
                        let payload = conns[i].buf[5..4 + total_len].to_vec();
                        conns[i].buf.drain(..4 + total_len);

                        handle_inbound_frame(
                            msg_type,
                            &payload,
                            &inboxes_bg,
                            &opt_inboxes_bg,
                            &migrations_bg,
                            &telemetry_bg,
                            &heartbeats_bg,
                            &gvt_bg,
                            &barrier_bg,
                        );
                    }

                    if eof || malformed {
                        conns.swap_remove(i);
                    } else {
                        i += 1;
                    }
                }

                thread::sleep(Duration::from_millis(1));
            }
        });

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
            barrier_signals,
            barrier_count: 0,
            shutdown,
            listener_handle: Some(listener_handle),
        })
    }

    pub fn local_lp(&self) -> LpId {
        self.local_lp
    }

    pub fn bound_addr(&self) -> SocketAddr {
        self.bound_addr
    }

    pub fn real_grpc_runtime_claimed(&self) -> bool {
        true
    }

    pub fn barrier_count(&self) -> u64 {
        self.barrier_count
    }

    pub fn received_barrier_count(&self, peer: LpId) -> u64 {
        let signals = self.barrier_signals.lock().unwrap();
        signals.get(&peer).copied().unwrap_or(0)
    }

    pub fn send_optimistic_envelope(
        &self,
        dest: LpId,
        envelope: &KairoOptimisticEnvelopeV1,
    ) -> Result<(), TransportError> {
        envelope
            .validate(GRPC_PROTOCOL_ID)
            .map_err(|_| TransportError::UnknownLogicalProcess(dest))?;
        let bytes = envelope
            .encode()
            .map_err(|_| TransportError::UnknownLogicalProcess(dest))?;
        self.send_frame(dest, MSG_TYPE_OPTIMISTIC, &bytes)
    }

    /// Drains validated optimistic messages (preserving positive/anti kind, ancestry, authority, and incarnation).
    pub fn recv_optimistic(
        &mut self,
        lp_id: LpId,
    ) -> Result<Vec<OptimisticMessage>, TransportError> {
        if !self.knows_lp(lp_id) {
            return Err(TransportError::UnknownLogicalProcess(lp_id));
        }

        let mut opt_inboxes = self.optimistic_inboxes.lock().unwrap();
        Ok(opt_inboxes.entry(lp_id).or_default().drain(..).collect())
    }

    pub fn send_migration(
        &self,
        dest: LpId,
        migration: &GrpcMigrationRequest,
    ) -> Result<(), TransportError> {
        migration
            .validate()
            .map_err(|_| TransportError::UnknownLogicalProcess(dest))?;

        let mut payload = Vec::new();
        payload.extend_from_slice(&migration.entity.index.to_be_bytes());
        payload.extend_from_slice(&migration.entity.generation.to_be_bytes());
        payload.extend_from_slice(&migration.source_lp.0.to_be_bytes());
        payload.extend_from_slice(&migration.dest_lp.0.to_be_bytes());
        let id_bytes = migration.migration_id.as_bytes();
        payload.extend_from_slice(&(id_bytes.len() as u32).to_be_bytes());
        payload.extend_from_slice(id_bytes);

        payload.extend_from_slice(&(migration.components.len() as u32).to_be_bytes());
        for comp in &migration.components {
            let type_bytes = comp.component_type_id.as_bytes();
            payload.extend_from_slice(&(type_bytes.len() as u32).to_be_bytes());
            payload.extend_from_slice(type_bytes);
            payload.extend_from_slice(&(comp.payload.len() as u32).to_be_bytes());
            payload.extend_from_slice(&comp.payload);
        }

        self.send_frame(dest, MSG_TYPE_MIGRATION, &payload)
    }

    /// Drains received entity migration requests.
    pub fn recv_migrations(&mut self) -> Vec<GrpcMigrationRequest> {
        let mut migrations = self.migrations.lock().unwrap();
        migrations.drain(..).collect()
    }

    pub fn send_telemetry(
        &self,
        dest: LpId,
        telemetry: &GrpcTelemetryBatch,
    ) -> Result<(), TransportError> {
        telemetry
            .validate()
            .map_err(|_| TransportError::UnknownLogicalProcess(dest))?;

        let mut payload = Vec::new();
        payload.extend_from_slice(&telemetry.source_lp.0.to_be_bytes());
        payload.extend_from_slice(&telemetry.tick_start.ticks().to_be_bytes());
        payload.extend_from_slice(&telemetry.tick_end.ticks().to_be_bytes());
        payload.extend_from_slice(&(telemetry.arrow_ipc_payload.len() as u32).to_be_bytes());
        payload.extend_from_slice(&telemetry.arrow_ipc_payload);

        self.send_frame(dest, MSG_TYPE_TELEMETRY, &payload)
    }

    /// Drains received telemetry batches.
    pub fn recv_telemetry(&mut self) -> Vec<GrpcTelemetryBatch> {
        let mut batches = self.telemetry_batches.lock().unwrap();
        batches.drain(..).collect()
    }

    pub fn send_heartbeat(
        &self,
        dest: LpId,
        heartbeat: &WorkerHeartbeat,
    ) -> Result<(), TransportError> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&heartbeat.lp_id.0.to_be_bytes());
        payload.extend_from_slice(&heartbeat.elapsed_since_last_seen.as_millis().to_be_bytes());

        self.send_frame(dest, MSG_TYPE_HEARTBEAT, &payload)
    }

    /// Drains received worker heartbeats.
    pub fn recv_heartbeats(&mut self) -> Vec<WorkerHeartbeat> {
        let mut heartbeats = self.heartbeats.lock().unwrap();
        heartbeats.drain(..).collect()
    }

    /// Sends a raw typed frame to a designated peer.
    fn send_frame(&self, dest: LpId, msg_type: u8, payload: &[u8]) -> Result<(), TransportError> {
        if payload.len() + 1 > MAX_FRAME_BYTES {
            return Err(TransportError::UnknownLogicalProcess(dest));
        }
        let addr = self
            .peers
            .get(&dest)
            .ok_or(TransportError::UnknownLogicalProcess(dest))?;
        let mut stream = TcpStream::connect_timeout(addr, self.config.request_timeout)
            .map_err(|_| TransportError::UnknownLogicalProcess(dest))?;

        write_framed_message(&mut stream, msg_type, payload)
            .map_err(|_| TransportError::UnknownLogicalProcess(dest))?;

        Ok(())
    }
}

impl Drop for GrpcSocketTransport {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        let _ = TcpStream::connect_timeout(&self.bound_addr, Duration::from_millis(50));
        if let Some(handle) = self.listener_handle.take() {
            let _ = handle.join();
        }
    }
}

impl PdesTransport for GrpcSocketTransport {
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

        // Encode and send over real socket
        match message {
            PdesMessage::Event(event) => {
                let mut payload = Vec::with_capacity(28 + event.event_payload.len());
                payload.extend_from_slice(&event.source_lp.0.to_be_bytes());
                payload.extend_from_slice(&event.dest_lp.0.to_be_bytes());
                payload.extend_from_slice(&event.tick.ticks().to_be_bytes());
                payload.extend_from_slice(&(event.event_payload.len() as u32).to_be_bytes());
                payload.extend_from_slice(&event.event_payload);
                self.send_frame(dest, MSG_TYPE_EVENT, &payload)?;
            }
            PdesMessage::Null(null) => {
                let mut payload = Vec::with_capacity(24);
                payload.extend_from_slice(&null.source_lp.0.to_be_bytes());
                payload.extend_from_slice(&null.dest_lp.0.to_be_bytes());
                payload.extend_from_slice(&null.safe_time.ticks().to_be_bytes());
                self.send_frame(dest, MSG_TYPE_NULL, &payload)?;
            }
        }

        Ok(())
    }

    fn recv(&mut self, lp_id: LpId) -> Result<Vec<PdesMessage>, TransportError> {
        if !self.knows_lp(lp_id) {
            return Err(TransportError::UnknownLogicalProcess(lp_id));
        }

        let mut inboxes = self.inboxes.lock().unwrap();
        Ok(inboxes.entry(lp_id).or_default().drain(..).collect())
    }

    fn barrier(&mut self) {
        self.barrier_count += 1;
        let count = self.barrier_count;

        // Broadcast barrier signal to peers
        for dest in self.peers.keys() {
            let mut payload = Vec::with_capacity(12);
            payload.extend_from_slice(&self.local_lp.0.to_be_bytes());
            payload.extend_from_slice(&count.to_be_bytes());
            let _ = self.send_frame(*dest, MSG_TYPE_BARRIER, &payload);
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

        // Broadcast local GVT proposal with round identifier to all configured peers
        let peer_ids: Vec<LpId> = self.peers.keys().copied().collect();
        for dest in &peer_ids {
            let mut payload = Vec::with_capacity(28);
            payload.extend_from_slice(&self.local_lp.0.to_be_bytes());
            payload.extend_from_slice(&round.to_be_bytes());
            payload.extend_from_slice(&timestamp.ticks().to_be_bytes());
            let _ = self.send_frame(*dest, MSG_TYPE_GVT_PROPOSAL, &payload);
        }

        // Wait until every configured peer has proposed for the current round (bounded by request_timeout)
        let deadline = Instant::now() + self.config.request_timeout;
        while Instant::now() < deadline {
            let all_arrived = {
                let proposals = self.gvt_proposals.lock().unwrap();
                peer_ids
                    .iter()
                    .all(|peer| proposals.contains_key(&(round, *peer)))
            };
            if all_arrived {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }

        // Compute minimum over current round proposals only and remove consumed round entries
        let mut min_tick = timestamp;
        let mut proposals = self.gvt_proposals.lock().unwrap();
        proposals.remove(&(round, self.local_lp));
        for peer in &peer_ids {
            if let Some(peer_tick) = proposals.remove(&(round, *peer)) {
                if peer_tick < min_tick {
                    min_tick = peer_tick;
                }
            }
        }

        min_tick
    }
}

fn parse_endpoint_addr(endpoint: &str) -> Result<SocketAddr, String> {
    if endpoint.starts_with("https://") {
        return Err(format!(
            "HTTPS endpoint {endpoint} is not supported by plaintext GrpcSocketTransport; configure TLS or use http://"
        ));
    }

    let stripped = endpoint.strip_prefix("http://").unwrap_or(endpoint);

    stripped
        .parse::<SocketAddr>()
        .map_err(|e| format!("Invalid socket address {stripped}: {e}"))
}

fn write_framed_message(stream: &mut TcpStream, msg_type: u8, payload: &[u8]) -> io::Result<()> {
    let len = (payload.len() + 1) as u32;
    stream.write_all(&len.to_be_bytes())?;
    stream.write_all(&[msg_type])?;
    stream.write_all(payload)?;
    stream.flush()?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn handle_inbound_frame(
    msg_type: u8,
    payload: &[u8],
    inboxes: &Arc<Mutex<BTreeMap<LpId, VecDeque<PdesMessage>>>>,
    optimistic_inboxes: &Arc<Mutex<BTreeMap<LpId, VecDeque<OptimisticMessage>>>>,
    migrations: &Arc<Mutex<VecDeque<GrpcMigrationRequest>>>,
    telemetry_batches: &Arc<Mutex<VecDeque<GrpcTelemetryBatch>>>,
    heartbeats: &Arc<Mutex<VecDeque<WorkerHeartbeat>>>,
    gvt_proposals: &Arc<Mutex<HashMap<(u64, LpId), Tick>>>,
    barrier_signals: &Arc<Mutex<HashMap<LpId, u64>>>,
) {
    match msg_type {
        MSG_TYPE_EVENT if payload.len() >= 24 => {
            let src = u32::from_be_bytes(payload[0..4].try_into().unwrap());
            let dst = u32::from_be_bytes(payload[4..8].try_into().unwrap());
            let tick_raw = u128::from_be_bytes(payload[8..24].try_into().unwrap());
            let data = if payload.len() >= 28 {
                let len = u32::from_be_bytes(payload[24..28].try_into().unwrap()) as usize;
                payload[28..28 + len.min(payload.len() - 28)].to_vec()
            } else {
                vec![]
            };

            let event = RemoteEvent {
                source_lp: LpId(src),
                dest_lp: LpId(dst),
                tick: Tick::from_ticks(tick_raw),
                event_payload: data,
            };

            let mut inboxes = inboxes.lock().unwrap();
            inboxes
                .entry(LpId(dst))
                .or_default()
                .push_back(PdesMessage::Event(event));
        }
        MSG_TYPE_NULL if payload.len() >= 24 => {
            let src = u32::from_be_bytes(payload[0..4].try_into().unwrap());
            let dst = u32::from_be_bytes(payload[4..8].try_into().unwrap());
            let safe_time_raw = u128::from_be_bytes(payload[8..24].try_into().unwrap());

            let null = NullMessage {
                source_lp: LpId(src),
                dest_lp: LpId(dst),
                safe_time: Tick::from_ticks(safe_time_raw),
            };

            let mut inboxes = inboxes.lock().unwrap();
            inboxes
                .entry(LpId(dst))
                .or_default()
                .push_back(PdesMessage::Null(null));
        }
        MSG_TYPE_OPTIMISTIC => {
            if let Ok(env) = KairoOptimisticEnvelopeV1::decode(payload) {
                if let Ok(opt_msg) = env.to_optimistic_message(GRPC_PROTOCOL_ID) {
                    let dest_lp = opt_msg.event().dest_lp;
                    let mut opt_inboxes = optimistic_inboxes.lock().unwrap();
                    opt_inboxes.entry(dest_lp).or_default().push_back(opt_msg);
                }
            }
        }
        MSG_TYPE_MIGRATION => {
            if let Some(migration) = decode_migration_payload(payload) {
                if migration.validate().is_ok() {
                    let mut migs = migrations.lock().unwrap();
                    migs.push_back(migration);
                }
            }
        }
        MSG_TYPE_TELEMETRY => {
            if let Some(batch) = decode_telemetry_payload(payload) {
                if batch.validate().is_ok() {
                    let mut batches = telemetry_batches.lock().unwrap();
                    batches.push_back(batch);
                }
            }
        }
        MSG_TYPE_HEARTBEAT if payload.len() >= 20 => {
            let lp_id = u32::from_be_bytes(payload[0..4].try_into().unwrap());
            let elapsed_ms = u128::from_be_bytes(payload[4..20].try_into().unwrap());
            let elapsed_ms_u64 = u64::try_from(elapsed_ms).unwrap_or(u64::MAX);
            let hb = WorkerHeartbeat {
                lp_id: LpId(lp_id),
                elapsed_since_last_seen: Duration::from_millis(elapsed_ms_u64),
            };
            let mut hbs = heartbeats.lock().unwrap();
            hbs.push_back(hb);
        }
        MSG_TYPE_GVT_PROPOSAL if payload.len() >= 28 => {
            let src = u32::from_be_bytes(payload[0..4].try_into().unwrap());
            let round = u64::from_be_bytes(payload[4..12].try_into().unwrap());
            let tick_raw = u128::from_be_bytes(payload[12..28].try_into().unwrap());
            let mut props = gvt_proposals.lock().unwrap();
            props.insert((round, LpId(src)), Tick::from_ticks(tick_raw));
        }
        MSG_TYPE_GVT_DECISION => {
            // Handled as notification
        }
        MSG_TYPE_BARRIER if payload.len() >= 12 => {
            let src = u32::from_be_bytes(payload[0..4].try_into().unwrap());
            let count = u64::from_be_bytes(payload[4..12].try_into().unwrap());
            let mut signals = barrier_signals.lock().unwrap();
            signals.insert(LpId(src), count);
        }
        _ => {}
    }
}

fn decode_migration_payload(payload: &[u8]) -> Option<GrpcMigrationRequest> {
    if payload.len() < 24 {
        return None;
    }
    let index = u64::from_be_bytes(payload[0..8].try_into().ok()?);
    let generation = u32::from_be_bytes(payload[8..12].try_into().ok()?);
    let source_lp = u32::from_be_bytes(payload[12..16].try_into().ok()?);
    let dest_lp = u32::from_be_bytes(payload[16..20].try_into().ok()?);
    let id_len = u32::from_be_bytes(payload[20..24].try_into().ok()?) as usize;

    let mut offset = 24;
    if payload.len() < offset + id_len + 4 {
        return None;
    }
    let migration_id = std::str::from_utf8(&payload[offset..offset + id_len])
        .ok()?
        .to_string();
    offset += id_len;

    let comp_count = u32::from_be_bytes(payload[offset..offset + 4].try_into().ok()?) as usize;
    offset += 4;

    let mut components = Vec::with_capacity(comp_count.min(64));
    for _ in 0..comp_count {
        if payload.len() < offset + 4 {
            return None;
        }
        let type_len = u32::from_be_bytes(payload[offset..offset + 4].try_into().ok()?) as usize;
        offset += 4;
        if payload.len() < offset + type_len + 4 {
            return None;
        }
        let component_type_id = std::str::from_utf8(&payload[offset..offset + type_len])
            .ok()?
            .to_string();
        offset += type_len;
        let data_len = u32::from_be_bytes(payload[offset..offset + 4].try_into().ok()?) as usize;
        offset += 4;
        if payload.len() < offset + data_len {
            return None;
        }
        let comp_payload = payload[offset..offset + data_len].to_vec();
        offset += data_len;
        components.push(GrpcComponentBlob {
            component_type_id,
            payload: comp_payload,
        });
    }

    Some(GrpcMigrationRequest {
        entity: EntityId { index, generation },
        source_lp: LpId(source_lp),
        dest_lp: LpId(dest_lp),
        migration_id,
        components,
    })
}

fn decode_telemetry_payload(payload: &[u8]) -> Option<GrpcTelemetryBatch> {
    if payload.len() < 40 {
        return None;
    }
    let source_lp = u32::from_be_bytes(payload[0..4].try_into().ok()?);
    let tick_start = u128::from_be_bytes(payload[4..20].try_into().ok()?);
    let tick_end = u128::from_be_bytes(payload[20..36].try_into().ok()?);
    let ipc_len = u32::from_be_bytes(payload[36..40].try_into().ok()?) as usize;
    if payload.len() < 40 + ipc_len {
        return None;
    }
    let arrow_ipc_payload = payload[40..40 + ipc_len].to_vec();

    Some(GrpcTelemetryBatch {
        source_lp: LpId(source_lp),
        tick_start: Tick::from_ticks(tick_start),
        tick_end: Tick::from_ticks(tick_end),
        arrow_ipc_payload,
    })
}
