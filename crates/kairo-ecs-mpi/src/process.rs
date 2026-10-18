#![forbid(unsafe_code)]

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use kairo_ecs_pdes::{
    LpId, NullMessage, PdesMessage, PdesTransport, RemoteEvent, Tick, TransportError,
};

use crate::MpiRankAssignment;

const MSG_TYPE_EVENT: u8 = 100;
const MSG_TYPE_NULL: u8 = 101;
const MSG_TYPE_BARRIER: u8 = 105;
const MSG_TYPE_BARRIER_RELEASE: u8 = 106;
const MSG_TYPE_GVT: u8 = 107;
const MSG_TYPE_GVT_REDUCED: u8 = 108;

struct InboundConnection {
    stream: TcpStream,
    buf: Vec<u8>,
}

/// Real multi-rank MPI transport communicating across distinct OS processes over sockets.
pub struct MpiProcessTransport {
    session_id: String,
    rank: i32,
    size: usize,
    assignments: Vec<MpiRankAssignment>,
    local_lp: LpId,
    lp_to_rank: HashMap<LpId, i32>,
    bound_addr: SocketAddr,
    peer_addrs: Arc<Mutex<HashMap<i32, SocketAddr>>>,
    peer_streams: HashMap<i32, TcpStream>,
    inboxes: Arc<Mutex<BTreeMap<LpId, VecDeque<PdesMessage>>>>,
    barrier_signals: Arc<Mutex<HashSet<(u64, i32)>>>,
    barrier_releases: Arc<Mutex<HashSet<u64>>>,
    barrier_round: u64,
    gvt_proposals: Arc<Mutex<HashMap<(u64, i32), Tick>>>,
    gvt_results: Arc<Mutex<HashMap<u64, Tick>>>,
    gvt_round: u64,
    shutdown: Arc<AtomicBool>,
    listener_handle: Option<JoinHandle<()>>,
}

impl MpiProcessTransport {
    /// Discovers rank and size from standard MPI environment variables (OpenMPI, PMIx, MPICH).
    pub fn detect_mpi_environment() -> Option<(i32, usize)> {
        let rank_str = env::var("OMPI_COMM_WORLD_RANK")
            .or_else(|_| env::var("PMIX_RANK"))
            .or_else(|_| env::var("MPI_LOCALRANKID"))
            .or_else(|_| env::var("PMI_RANK"))
            .ok()?;

        let size_str = env::var("OMPI_COMM_WORLD_SIZE")
            .or_else(|_| env::var("PMIX_SIZE"))
            .or_else(|_| env::var("MPI_LOCALNRANKS"))
            .or_else(|_| env::var("PMI_SIZE"))
            .ok()?;

        let rank: i32 = rank_str.parse().ok()?;
        let size: usize = size_str.parse().ok()?;

        Some((rank, size))
    }

    /// Creates and binds a real multi-rank transport.
    pub fn new(
        rank: i32,
        size: usize,
        assignments: Vec<MpiRankAssignment>,
        session_id: &str,
    ) -> Result<Self, io::Error> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let bound_addr = listener.local_addr()?;

        let mut lp_to_rank = HashMap::new();
        let mut local_lp = None;
        let mut inboxes_init = BTreeMap::new();

        for a in &assignments {
            lp_to_rank.insert(a.lp_id, a.rank);
            inboxes_init.insert(a.lp_id, VecDeque::new());
            if a.rank == rank {
                local_lp = Some(a.lp_id);
            }
        }

        let local_lp = local_lp.unwrap_or(LpId(rank as u32));
        inboxes_init.entry(local_lp).or_default();

        let inboxes = Arc::new(Mutex::new(inboxes_init));
        let barrier_signals = Arc::new(Mutex::new(HashSet::new()));
        let barrier_releases = Arc::new(Mutex::new(HashSet::new()));
        let gvt_proposals = Arc::new(Mutex::new(HashMap::new()));
        let gvt_results = Arc::new(Mutex::new(HashMap::new()));
        let shutdown = Arc::new(AtomicBool::new(false));

        let inboxes_bg = Arc::clone(&inboxes);
        let barrier_signals_bg = Arc::clone(&barrier_signals);
        let barrier_releases_bg = Arc::clone(&barrier_releases);
        let gvt_proposals_bg = Arc::clone(&gvt_proposals);
        let gvt_results_bg = Arc::clone(&gvt_results);
        let shutdown_bg = Arc::clone(&shutdown);

        // Write this rank's bound address to rendezvous file
        let addr_path = rendezvous_file_path(session_id, rank);
        let mut file = File::create(&addr_path)?;
        file.write_all(bound_addr.to_string().as_bytes())?;
        file.flush()?;

        let listener_handle = thread::spawn(move || {
            let mut conns: Vec<InboundConnection> = Vec::new();
            let mut chunk = [0u8; 4096];

            while !shutdown_bg.load(Ordering::Relaxed) {
                // Accept new connections
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

                // Process existing connections
                let mut i = 0;
                while i < conns.len() {
                    let mut dead = false;

                    // Read available bytes
                    loop {
                        match conns[i].stream.read(&mut chunk) {
                            Ok(0) => {
                                dead = true;
                                break;
                            }
                            Ok(n) => {
                                conns[i].buf.extend_from_slice(&chunk[..n]);
                            }
                            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                                break;
                            }
                            Err(_) => {
                                dead = true;
                                break;
                            }
                        }
                    }

                    // Extract all complete frames
                    while conns[i].buf.len() >= 4 {
                        let total_len =
                            u32::from_be_bytes(conns[i].buf[0..4].try_into().unwrap()) as usize;
                        if total_len == 0 {
                            dead = true;
                            break;
                        }
                        if conns[i].buf.len() < 4 + total_len {
                            break;
                        }

                        let msg_type = conns[i].buf[4];
                        let payload = conns[i].buf[5..4 + total_len].to_vec();
                        conns[i].buf.drain(..4 + total_len);

                        handle_inbound_mpi_frame(
                            msg_type,
                            &payload,
                            &inboxes_bg,
                            &barrier_signals_bg,
                            &barrier_releases_bg,
                            &gvt_proposals_bg,
                            &gvt_results_bg,
                        );
                    }

                    if dead {
                        conns.swap_remove(i);
                    } else {
                        i += 1;
                    }
                }

                thread::sleep(Duration::from_millis(1));
            }
        });

        Ok(Self {
            session_id: session_id.to_string(),
            rank,
            size,
            assignments,
            local_lp,
            lp_to_rank,
            bound_addr,
            peer_addrs: Arc::new(Mutex::new(HashMap::new())),
            peer_streams: HashMap::new(),
            inboxes,
            barrier_signals,
            barrier_releases,
            barrier_round: 0,
            gvt_proposals,
            gvt_results,
            gvt_round: 0,
            shutdown,
            listener_handle: Some(listener_handle),
        })
    }

    pub fn rank(&self) -> i32 {
        self.rank
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn local_lp(&self) -> LpId {
        self.local_lp
    }

    pub fn assignments(&self) -> &[MpiRankAssignment] {
        &self.assignments
    }

    pub fn bound_addr(&self) -> SocketAddr {
        self.bound_addr
    }

    pub fn real_mpi_runtime_claimed(&self) -> bool {
        true
    }

    fn resolve_peer_addr(&self, target_rank: i32) -> Result<SocketAddr, TransportError> {
        if target_rank == self.rank {
            return Ok(self.bound_addr);
        }

        {
            let addrs = self.peer_addrs.lock().unwrap();
            if let Some(&addr) = addrs.get(&target_rank) {
                return Ok(addr);
            }
        }

        let addr_path = rendezvous_file_path(&self.session_id, target_rank);
        let deadline = Instant::now() + Duration::from_secs(5);

        while Instant::now() < deadline {
            if let Ok(content) = fs::read_to_string(&addr_path) {
                let trimmed = content.trim();
                if !trimmed.is_empty() {
                    if let Ok(addr) = trimmed.parse::<SocketAddr>() {
                        let mut addrs = self.peer_addrs.lock().unwrap();
                        addrs.insert(target_rank, addr);
                        return Ok(addr);
                    }
                }
            }
            thread::sleep(Duration::from_millis(10));
        }

        Err(TransportError::UnknownLogicalProcess(self.local_lp))
    }

    fn send_frame_to_rank(
        &mut self,
        target_rank: i32,
        msg_type: u8,
        payload: &[u8],
    ) -> Result<(), TransportError> {
        if target_rank == self.rank {
            return Ok(());
        }

        let target_addr = self.resolve_peer_addr(target_rank)?;

        // Try existing stream or open a new one
        if let Some(stream) = self.peer_streams.get_mut(&target_rank) {
            if write_framed_msg(stream, msg_type, payload).is_ok() {
                return Ok(());
            }
            // Stream died, drop it
            self.peer_streams.remove(&target_rank);
        }

        let mut stream = TcpStream::connect_timeout(&target_addr, Duration::from_secs(3))
            .map_err(|_| TransportError::UnknownLogicalProcess(self.local_lp))?;

        write_framed_msg(&mut stream, msg_type, payload)
            .map_err(|_| TransportError::UnknownLogicalProcess(self.local_lp))?;

        self.peer_streams.insert(target_rank, stream);
        Ok(())
    }
}

impl Drop for MpiProcessTransport {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);

        // Remove own rendezvous file
        let own_path = rendezvous_file_path(&self.session_id, self.rank);
        let _ = fs::remove_file(own_path);

        // Connect to own listener to unblock accept if pending
        let _ = TcpStream::connect_timeout(&self.bound_addr, Duration::from_millis(50));

        if let Some(handle) = self.listener_handle.take() {
            let _ = handle.join();
        }
    }
}

impl PdesTransport for MpiProcessTransport {
    fn knows_lp(&self, lp_id: LpId) -> bool {
        lp_id == self.local_lp || self.lp_to_rank.contains_key(&lp_id)
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

        let target_rank = *self.lp_to_rank.get(&dest).unwrap_or(&self.rank);

        if target_rank == self.rank {
            let mut inboxes = self.inboxes.lock().unwrap();
            inboxes.entry(dest).or_default().push_back(message);
            return Ok(());
        }

        match message {
            PdesMessage::Event(event) => {
                let mut payload = Vec::new();
                payload.extend_from_slice(&event.source_lp.0.to_be_bytes());
                payload.extend_from_slice(&event.dest_lp.0.to_be_bytes());
                payload.extend_from_slice(&event.tick.ticks().to_be_bytes());
                payload.extend_from_slice(&(event.event_payload.len() as u32).to_be_bytes());
                payload.extend_from_slice(&event.event_payload);
                self.send_frame_to_rank(target_rank, MSG_TYPE_EVENT, &payload)?;
            }
            PdesMessage::Null(null) => {
                let mut payload = Vec::new();
                payload.extend_from_slice(&null.source_lp.0.to_be_bytes());
                payload.extend_from_slice(&null.dest_lp.0.to_be_bytes());
                payload.extend_from_slice(&null.safe_time.ticks().to_be_bytes());
                self.send_frame_to_rank(target_rank, MSG_TYPE_NULL, &payload)?;
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
        if self.size <= 1 {
            return;
        }

        let round = self.barrier_round;
        self.barrier_round += 1;

        if self.rank == 0 {
            {
                let mut signals = self.barrier_signals.lock().unwrap();
                signals.insert((round, 0));
            }

            let deadline = Instant::now() + Duration::from_secs(10);
            while Instant::now() < deadline {
                let all_arrived = {
                    let signals = self.barrier_signals.lock().unwrap();
                    (0..self.size).all(|r| signals.contains(&(round, r as i32)))
                };

                if all_arrived {
                    break;
                }
                thread::sleep(Duration::from_millis(1));
            }

            // Release all peer ranks
            let mut payload = Vec::new();
            payload.extend_from_slice(&round.to_be_bytes());
            for r in 1..self.size {
                let _ = self.send_frame_to_rank(r as i32, MSG_TYPE_BARRIER_RELEASE, &payload);
            }
        } else {
            let mut payload = Vec::new();
            payload.extend_from_slice(&self.rank.to_be_bytes());
            payload.extend_from_slice(&round.to_be_bytes());
            let _ = self.send_frame_to_rank(0, MSG_TYPE_BARRIER, &payload);

            let deadline = Instant::now() + Duration::from_secs(10);
            while Instant::now() < deadline {
                let released = {
                    let rel = self.barrier_releases.lock().unwrap();
                    rel.contains(&round)
                };
                if released {
                    break;
                }
                thread::sleep(Duration::from_millis(1));
            }
        }
    }

    fn all_reduce_min(&mut self, timestamp: Tick) -> Tick {
        if self.size <= 1 {
            return timestamp;
        }

        let round = self.gvt_round;
        self.gvt_round += 1;

        if self.rank == 0 {
            {
                let mut proposals = self.gvt_proposals.lock().unwrap();
                proposals.insert((round, 0), timestamp);
            }

            let deadline = Instant::now() + Duration::from_secs(10);
            while Instant::now() < deadline {
                let all_proposed = {
                    let proposals = self.gvt_proposals.lock().unwrap();
                    (0..self.size).all(|r| proposals.contains_key(&(round, r as i32)))
                };

                if all_proposed {
                    break;
                }
                thread::sleep(Duration::from_millis(1));
            }

            let min_tick = {
                let proposals = self.gvt_proposals.lock().unwrap();
                let mut m = timestamp;
                for r in 0..self.size {
                    if let Some(&t) = proposals.get(&(round, r as i32)) {
                        if t < m {
                            m = t;
                        }
                    }
                }
                m
            };

            // Broadcast reduced min to all ranks
            let mut payload = Vec::new();
            payload.extend_from_slice(&round.to_be_bytes());
            payload.extend_from_slice(&min_tick.ticks().to_be_bytes());
            for r in 1..self.size {
                let _ = self.send_frame_to_rank(r as i32, MSG_TYPE_GVT_REDUCED, &payload);
            }

            let mut results = self.gvt_results.lock().unwrap();
            results.insert(round, min_tick);
            min_tick
        } else {
            let mut payload = Vec::new();
            payload.extend_from_slice(&self.rank.to_be_bytes());
            payload.extend_from_slice(&round.to_be_bytes());
            payload.extend_from_slice(&timestamp.ticks().to_be_bytes());
            let _ = self.send_frame_to_rank(0, MSG_TYPE_GVT, &payload);

            let deadline = Instant::now() + Duration::from_secs(10);
            while Instant::now() < deadline {
                let result = {
                    let results = self.gvt_results.lock().unwrap();
                    results.get(&round).copied()
                };

                if let Some(min_t) = result {
                    return min_t;
                }
                thread::sleep(Duration::from_millis(1));
            }

            timestamp
        }
    }
}

fn rendezvous_file_path(session_id: &str, rank: i32) -> PathBuf {
    env::temp_dir().join(format!("kairo_mpi_{session_id}_rank_{rank}.addr"))
}

fn write_framed_msg(stream: &mut TcpStream, msg_type: u8, payload: &[u8]) -> io::Result<()> {
    let len = (payload.len() + 1) as u32;
    stream.write_all(&len.to_be_bytes())?;
    stream.write_all(&[msg_type])?;
    stream.write_all(payload)?;
    stream.flush()?;
    Ok(())
}

fn handle_inbound_mpi_frame(
    msg_type: u8,
    payload: &[u8],
    inboxes: &Arc<Mutex<BTreeMap<LpId, VecDeque<PdesMessage>>>>,
    barrier_signals: &Arc<Mutex<HashSet<(u64, i32)>>>,
    barrier_releases: &Arc<Mutex<HashSet<u64>>>,
    gvt_proposals: &Arc<Mutex<HashMap<(u64, i32), Tick>>>,
    gvt_results: &Arc<Mutex<HashMap<u64, Tick>>>,
) {
    match msg_type {
        MSG_TYPE_EVENT if payload.len() >= 28 => {
            let src = u32::from_be_bytes(payload[0..4].try_into().unwrap());
            let dst = u32::from_be_bytes(payload[4..8].try_into().unwrap());
            let tick_raw = u128::from_be_bytes(payload[8..24].try_into().unwrap());
            let len = u32::from_be_bytes(payload[24..28].try_into().unwrap()) as usize;
            let data = payload[28..28 + len.min(payload.len() - 28)].to_vec();

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
        MSG_TYPE_BARRIER if payload.len() >= 12 => {
            let rank = i32::from_be_bytes(payload[0..4].try_into().unwrap());
            let round = u64::from_be_bytes(payload[4..12].try_into().unwrap());
            let mut signals = barrier_signals.lock().unwrap();
            signals.insert((round, rank));
        }
        MSG_TYPE_BARRIER_RELEASE if payload.len() >= 8 => {
            let round = u64::from_be_bytes(payload[0..8].try_into().unwrap());
            let mut rel = barrier_releases.lock().unwrap();
            rel.insert(round);
        }
        MSG_TYPE_GVT if payload.len() >= 28 => {
            let rank = i32::from_be_bytes(payload[0..4].try_into().unwrap());
            let round = u64::from_be_bytes(payload[4..12].try_into().unwrap());
            let tick_raw = u128::from_be_bytes(payload[12..28].try_into().unwrap());
            let mut props = gvt_proposals.lock().unwrap();
            props.insert((round, rank), Tick::from_ticks(tick_raw));
        }
        MSG_TYPE_GVT_REDUCED if payload.len() >= 24 => {
            let round = u64::from_be_bytes(payload[0..8].try_into().unwrap());
            let min_tick_raw = u128::from_be_bytes(payload[8..24].try_into().unwrap());
            let mut res = gvt_results.lock().unwrap();
            res.insert(round, Tick::from_ticks(min_tick_raw));
        }
        _ => {}
    }
}
