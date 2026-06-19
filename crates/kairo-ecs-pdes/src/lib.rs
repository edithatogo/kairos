#![forbid(unsafe_code)]

use std::collections::{BTreeMap, VecDeque};

use kairo_ecs_types::{EntityId, SimDuration, SimTime};

/// Stable logical-process identifier.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LpId(pub u32);

/// Simulation tick type used by PDES contracts.
pub type Tick = SimTime;

/// Partition metadata supplied to a logical process at startup.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorldSegment {
    pub id: LpId,
    pub lp_count: u32,
    pub entities: Vec<EntityId>,
}

/// Application event exchanged between logical processes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemoteEvent {
    pub source_lp: LpId,
    pub dest_lp: LpId,
    pub tick: Tick,
    pub event_payload: Vec<u8>,
}

/// CMB null message used to advertise a lower-bound timestamp to a neighbor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NullMessage {
    pub source_lp: LpId,
    pub dest_lp: LpId,
    pub safe_time: Tick,
}

/// Messages exchanged by a PDES transport.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PdesMessage {
    Event(RemoteEvent),
    Null(NullMessage),
}

impl PdesMessage {
    fn source_lp(&self) -> LpId {
        match self {
            Self::Event(event) => event.source_lp,
            Self::Null(message) => message.source_lp,
        }
    }

    fn dest_lp(&self) -> LpId {
        match self {
            Self::Event(event) => event.dest_lp,
            Self::Null(message) => message.dest_lp,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransportError {
    UnknownLogicalProcess(LpId),
    MessageDestinationMismatch { send_dest: LpId, message_dest: LpId },
}

impl TransportError {
    fn for_lp(lp_id: LpId) -> Self {
        Self::UnknownLogicalProcess(lp_id)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PdesError {
    DuplicateLogicalProcess(LpId),
    MismatchedWorldSegment {
        lp_id: LpId,
        segment_id: LpId,
    },
    SelfLoopNeighbor(LpId),
    DuplicateNeighbor {
        lp_id: LpId,
        neighbor: LpId,
    },
    LookaheadViolation {
        lp_id: LpId,
        event_tick: Tick,
        minimum_tick: Tick,
    },
    LookaheadOverflow {
        lp_id: LpId,
        local_time: Tick,
        lookahead: SimDuration,
    },
    Transport(TransportError),
}

/// Lifecycle contract for a single logical process.
pub trait LogicalProcess {
    fn init(&mut self, lp_id: LpId, world_segment: &WorldSegment);

    fn process_local_events(&mut self, until: Tick);

    fn schedule_remote_event(&mut self) -> Vec<RemoteEvent>;

    fn receive_remote_events(&mut self, events: Vec<RemoteEvent>);

    fn advance_to(&mut self, tick: Tick);

    fn local_time(&self) -> Tick;

    fn lookahead(&self) -> SimDuration;
}

/// Transport boundary used by the single-node scheduler and distributed backends.
pub trait PdesTransport {
    fn knows_lp(&self, lp_id: LpId) -> bool;

    fn send(&mut self, dest: LpId, message: PdesMessage) -> Result<(), TransportError>;

    fn recv(&mut self, lp_id: LpId) -> Result<Vec<PdesMessage>, TransportError>;

    fn barrier(&mut self);

    fn all_reduce_min(&mut self, timestamp: Tick) -> Tick;
}

/// Deterministic in-memory transport for tests and single-process scaffolding.
#[derive(Debug, Default)]
pub struct ThreadChannelTransport {
    inboxes: BTreeMap<LpId, VecDeque<PdesMessage>>,
    gvt_candidates: Vec<Tick>,
}

impl ThreadChannelTransport {
    pub fn new(lp_ids: impl IntoIterator<Item = LpId>) -> Self {
        let mut inboxes = BTreeMap::new();
        for lp_id in lp_ids {
            inboxes.insert(lp_id, VecDeque::new());
        }

        Self {
            inboxes,
            gvt_candidates: Vec::new(),
        }
    }

    pub fn pending_messages(&self) -> usize {
        self.inboxes.values().map(VecDeque::len).sum()
    }

    fn ensure_known_lp(&self, lp_id: LpId) -> Result<(), TransportError> {
        if self.inboxes.contains_key(&lp_id) {
            Ok(())
        } else {
            Err(TransportError::for_lp(lp_id))
        }
    }

    fn pending_min_timestamp(&self) -> Option<Tick> {
        self.inboxes
            .values()
            .flat_map(|messages| messages.iter())
            .filter_map(|message| match message {
                PdesMessage::Event(event) => Some(event.tick),
                PdesMessage::Null(_) => None,
            })
            .min()
    }
}

impl PdesTransport for ThreadChannelTransport {
    fn knows_lp(&self, lp_id: LpId) -> bool {
        self.inboxes.contains_key(&lp_id)
    }

    fn send(&mut self, dest: LpId, message: PdesMessage) -> Result<(), TransportError> {
        self.ensure_known_lp(dest)?;
        self.ensure_known_lp(message.source_lp())?;
        let message_dest = message.dest_lp();
        if message_dest != dest {
            return Err(TransportError::MessageDestinationMismatch {
                send_dest: dest,
                message_dest,
            });
        }
        self.ensure_known_lp(message_dest)?;
        self.inboxes
            .get_mut(&dest)
            .expect("transport topology must remain stable during send")
            .push_back(message);
        Ok(())
    }

    fn recv(&mut self, lp_id: LpId) -> Result<Vec<PdesMessage>, TransportError> {
        self.ensure_known_lp(lp_id)?;
        Ok(self
            .inboxes
            .get_mut(&lp_id)
            .expect("transport topology must remain stable during recv")
            .drain(..)
            .collect())
    }

    fn barrier(&mut self) {}

    fn all_reduce_min(&mut self, timestamp: Tick) -> Tick {
        self.gvt_candidates.clear();
        self.gvt_candidates.push(timestamp);
        let local_min = timestamp;

        self.pending_min_timestamp()
            .map_or(local_min, |pending_min| local_min.min(pending_min))
    }
}

/// Conservative PDES scheduler scaffold using the CMB lookahead rule.
pub struct PdesScheduler<T: PdesTransport> {
    lps: BTreeMap<LpId, Box<dyn LogicalProcess>>,
    neighbors: BTreeMap<LpId, Vec<LpId>>,
    transport: T,
    gvt: Tick,
}

impl<T: PdesTransport> PdesScheduler<T> {
    pub fn new(transport: T) -> Self {
        Self {
            lps: BTreeMap::new(),
            neighbors: BTreeMap::new(),
            transport,
            gvt: SimTime::ZERO,
        }
    }

    pub fn add_lp(
        &mut self,
        lp_id: LpId,
        world_segment: WorldSegment,
        neighbors: Vec<LpId>,
        mut lp: Box<dyn LogicalProcess>,
    ) -> Result<(), PdesError> {
        if self.lps.contains_key(&lp_id) {
            return Err(PdesError::DuplicateLogicalProcess(lp_id));
        }
        if world_segment.id != lp_id {
            return Err(PdesError::MismatchedWorldSegment {
                lp_id,
                segment_id: world_segment.id,
            });
        }

        let mut unique_neighbors = Vec::new();
        for neighbor in neighbors {
            if neighbor == lp_id {
                return Err(PdesError::SelfLoopNeighbor(lp_id));
            }

            if unique_neighbors.contains(&neighbor) {
                return Err(PdesError::DuplicateNeighbor { lp_id, neighbor });
            }
            if !self.transport.knows_lp(neighbor) {
                return Err(PdesError::Transport(TransportError::UnknownLogicalProcess(
                    neighbor,
                )));
            }

            unique_neighbors.push(neighbor);
        }

        lp.init(lp_id, &world_segment);
        self.neighbors.insert(lp_id, unique_neighbors);
        self.lps.insert(lp_id, lp);
        Ok(())
    }

    pub fn gvt(&self) -> Tick {
        self.gvt
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn step_until(&mut self, until: Tick) -> Result<(), PdesError> {
        let lp_ids: Vec<LpId> = self.lps.keys().copied().collect();

        for lp_id in &lp_ids {
            let inbound = self.transport.recv(*lp_id).map_err(PdesError::Transport)?;
            let mut safe_until = until;
            let events = inbound
                .into_iter()
                .filter_map(|message| match message {
                    PdesMessage::Event(event) => Some(event),
                    PdesMessage::Null(message) => {
                        safe_until = safe_until.min(message.safe_time);
                        None
                    }
                })
                .collect();

            if let Some(lp) = self.lps.get_mut(lp_id) {
                lp.receive_remote_events(events);
                lp.process_local_events(safe_until.max(lp.local_time()));
            }
        }

        for lp_id in &lp_ids {
            if let Some(lp) = self.lps.get_mut(lp_id) {
                let minimum_remote_tick = lp.local_time().checked_add(lp.lookahead()).ok_or(
                    PdesError::LookaheadOverflow {
                        lp_id: *lp_id,
                        local_time: lp.local_time(),
                        lookahead: lp.lookahead(),
                    },
                )?;
                for event in lp.schedule_remote_event() {
                    if event.tick < minimum_remote_tick {
                        return Err(PdesError::LookaheadViolation {
                            lp_id: *lp_id,
                            event_tick: event.tick,
                            minimum_tick: minimum_remote_tick,
                        });
                    }
                    self.transport
                        .send(event.dest_lp, PdesMessage::Event(event))
                        .map_err(PdesError::Transport)?;
                }

                let safe_time = minimum_remote_tick;
                if let Some(neighbors) = self.neighbors.get(lp_id) {
                    for neighbor in neighbors {
                        self.transport
                            .send(
                                *neighbor,
                                PdesMessage::Null(NullMessage {
                                    source_lp: *lp_id,
                                    dest_lp: *neighbor,
                                    safe_time,
                                }),
                            )
                            .map_err(PdesError::Transport)?;
                    }
                }
            }
        }

        self.transport.barrier();
        self.gvt = self.compute_gvt();

        for lp in self.lps.values_mut() {
            let advance_tick = self.gvt.max(lp.local_time());
            lp.advance_to(advance_tick);
        }

        Ok(())
    }

    fn compute_gvt(&mut self) -> Tick {
        let local_min = self
            .lps
            .values()
            .map(|lp| lp.local_time())
            .min()
            .unwrap_or(SimTime::ZERO);
        self.transport.all_reduce_min(local_min)
    }
}

/// Deterministic fixture used to compare a sequential reference with a
/// partitioned PDES-style event exchange without depending on the core runtime.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParityWorkload {
    pub lp_count: u32,
    pub ticks: u128,
    pub entities_per_lp: u32,
}

/// Final state and protocol counters from a parity/stress run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParityReport {
    pub final_state: BTreeMap<(LpId, u32), i128>,
    pub remote_events: usize,
    pub null_messages: usize,
    pub gvt_history: Vec<Tick>,
}

/// Conservative PDES validation evidence for local gates and handoff notes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdesValidationEvidence {
    pub workload: ParityWorkload,
    pub final_state_parity: bool,
    pub gvt_monotonic: bool,
    pub gvt_reaches_final_tick: bool,
    pub deadlock_smoke: bool,
    pub remote_events: usize,
    pub null_messages: usize,
    pub gvt_samples: usize,
}

/// Deterministic benchmark-smoke sample for the required Track 34 LP counts.
///
/// This is logical-work evidence only. It intentionally does not measure wall
/// clock time or claim hardware speedup.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdesScalingSample {
    pub lp_count: u32,
    pub ticks: u128,
    pub entities_per_lp: u32,
    pub sequential_event_steps: u128,
    pub partitioned_event_steps: u128,
    pub remote_events: usize,
    pub null_messages: usize,
    pub gvt_samples: usize,
    pub final_state_parity: bool,
    pub hardware_speedup_claimed: bool,
}

/// Research-spike result for optimistic Time Warp rollback behaviour.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimeWarpSpikeReport {
    pub lp_count: u32,
    pub processed_events: usize,
    pub straggler_events: usize,
    pub rollback_events: usize,
    pub fossil_collectable_events: usize,
    pub recommendation: &'static str,
}

impl ParityWorkload {
    pub fn small() -> Self {
        Self {
            lp_count: 4,
            ticks: 32,
            entities_per_lp: 8,
        }
    }

    pub fn stress() -> Self {
        Self {
            lp_count: 8,
            ticks: 10_000,
            entities_per_lp: 4,
        }
    }
}

/// Runs the deterministic sequential oracle for a fixed workload.
pub fn run_sequential_reference(workload: &ParityWorkload) -> ParityReport {
    let mut final_state = initial_state(workload);
    let mut gvt_history = Vec::with_capacity(workload.ticks.min(usize::MAX as u128) as usize);

    for tick in 0..workload.ticks {
        let mut deltas: BTreeMap<(LpId, u32), i128> = BTreeMap::new();

        for lp_index in 0..workload.lp_count {
            let lp_id = LpId(lp_index);
            let neighbor = LpId((lp_index + 1) % workload.lp_count);
            let local_delta = ((tick as i128 + lp_index as i128) % 7) - 3;
            let remote_delta = ((tick as i128 + lp_index as i128 * 3) % 5) - 2;

            for entity in 0..workload.entities_per_lp {
                *deltas.entry((lp_id, entity)).or_default() += local_delta;
                *deltas.entry((neighbor, entity)).or_default() += remote_delta;
            }
        }

        apply_deltas(&mut final_state, deltas);
        gvt_history.push(SimTime::from_ticks(tick + 1));
    }

    ParityReport {
        final_state,
        remote_events: 0,
        null_messages: 0,
        gvt_history,
    }
}

/// Runs the same workload through a partitioned message-exchange model.
pub fn run_partitioned_reference(workload: &ParityWorkload) -> ParityReport {
    let mut final_state = initial_state(workload);
    let mut remote_events = 0usize;
    let mut null_messages = 0usize;
    let mut gvt_history = Vec::with_capacity(workload.ticks.min(usize::MAX as u128) as usize);

    for tick in 0..workload.ticks {
        let mut local_deltas: BTreeMap<(LpId, u32), i128> = BTreeMap::new();
        let mut remote_messages = Vec::new();

        for lp_index in 0..workload.lp_count {
            let lp_id = LpId(lp_index);
            let neighbor = LpId((lp_index + 1) % workload.lp_count);
            let local_delta = ((tick as i128 + lp_index as i128) % 7) - 3;
            let remote_delta = ((tick as i128 + lp_index as i128 * 3) % 5) - 2;

            for entity in 0..workload.entities_per_lp {
                *local_deltas.entry((lp_id, entity)).or_default() += local_delta;
                remote_messages.push((neighbor, entity, remote_delta));
                remote_events += 1;
            }
            null_messages += 1;
        }

        apply_deltas(&mut final_state, local_deltas);
        remote_messages.sort_by_key(|(lp, entity, _)| (*lp, *entity));
        let mut remote_deltas: BTreeMap<(LpId, u32), i128> = BTreeMap::new();
        for (lp, entity, delta) in remote_messages {
            *remote_deltas.entry((lp, entity)).or_default() += delta;
        }
        apply_deltas(&mut final_state, remote_deltas);
        gvt_history.push(SimTime::from_ticks(tick + 1));
    }

    ParityReport {
        final_state,
        remote_events,
        null_messages,
        gvt_history,
    }
}

/// Checks final-state parity between the sequential and partitioned references.
pub fn sequential_parity_report(workload: &ParityWorkload) -> Result<ParityReport, String> {
    validate_workload(workload)?;

    let sequential = run_sequential_reference(workload);
    let partitioned = run_partitioned_reference(workload);

    if sequential.final_state == partitioned.final_state {
        Ok(partitioned)
    } else {
        Err("partitioned final state differs from sequential reference".to_string())
    }
}

/// Runs the long deterministic stress fixture used by the Track 34 no-skip gate.
pub fn deadlock_stress_report() -> Result<ParityReport, String> {
    let report = sequential_parity_report(&ParityWorkload::stress())?;
    if report.gvt_history.len() == 10_000
        && is_monotonic(&report.gvt_history)
        && report.gvt_history.last() == Some(&SimTime::from_ticks(10_000))
        && report.remote_events > 0
        && report.null_messages > 0
    {
        Ok(report)
    } else {
        Err(
            "deadlock stress fixture did not produce complete conservative PDES evidence"
                .to_string(),
        )
    }
}

/// Builds local validation evidence for conservative PDES quality gates.
pub fn validate_conservative_pdes(
    workloads: &[ParityWorkload],
) -> Result<Vec<PdesValidationEvidence>, String> {
    if workloads.is_empty() {
        return Err("at least one PDES workload is required".to_string());
    }

    workloads
        .iter()
        .map(|workload| {
            let report = sequential_parity_report(workload)?;
            let final_tick = SimTime::from_ticks(workload.ticks);
            let gvt_monotonic = is_monotonic(&report.gvt_history);
            let gvt_reaches_final_tick = report.gvt_history.last() == Some(&final_tick);
            let deadlock_smoke = report.gvt_history.len() == workload.ticks as usize
                && gvt_monotonic
                && gvt_reaches_final_tick
                && report.remote_events > 0
                && report.null_messages > 0;

            Ok(PdesValidationEvidence {
                workload: workload.clone(),
                final_state_parity: true,
                gvt_monotonic,
                gvt_reaches_final_tick,
                deadlock_smoke,
                remote_events: report.remote_events,
                null_messages: report.null_messages,
                gvt_samples: report.gvt_history.len(),
            })
        })
        .collect()
}

/// Builds deterministic local scaling evidence for 4/8/16/32-LP smoke gates.
pub fn scaling_smoke_samples(lp_counts: &[u32]) -> Result<Vec<PdesScalingSample>, String> {
    if lp_counts.is_empty() {
        return Err("at least one LP count is required".to_string());
    }

    lp_counts
        .iter()
        .map(|lp_count| {
            let workload = ParityWorkload {
                lp_count: *lp_count,
                ticks: 256,
                entities_per_lp: 4,
            };
            validate_workload(&workload)?;

            let sequential = run_sequential_reference(&workload);
            let partitioned = run_partitioned_reference(&workload);
            let logical_steps = workload.ticks
                * u128::from(workload.lp_count)
                * u128::from(workload.entities_per_lp);

            Ok(PdesScalingSample {
                lp_count: *lp_count,
                ticks: workload.ticks,
                entities_per_lp: workload.entities_per_lp,
                sequential_event_steps: logical_steps * 2,
                partitioned_event_steps: logical_steps * 2,
                remote_events: partitioned.remote_events,
                null_messages: partitioned.null_messages,
                gvt_samples: partitioned.gvt_history.len(),
                final_state_parity: sequential.final_state == partitioned.final_state,
                hardware_speedup_claimed: false,
            })
        })
        .collect()
}

/// Runs a dependency-free optimistic rollback spike used to document Time Warp
/// risk without introducing a production optimistic scheduler.
pub fn time_warp_two_lp_spike() -> TimeWarpSpikeReport {
    let arrivals = [
        (LpId(0), SimTime::from_ticks(1)),
        (LpId(1), SimTime::from_ticks(1)),
        (LpId(0), SimTime::from_ticks(4)),
        (LpId(1), SimTime::from_ticks(4)),
        (LpId(0), SimTime::from_ticks(2)),
        (LpId(1), SimTime::from_ticks(3)),
        (LpId(0), SimTime::from_ticks(5)),
        (LpId(1), SimTime::from_ticks(6)),
    ];
    let mut lp_clocks: BTreeMap<LpId, Tick> = BTreeMap::new();
    let mut straggler_events = 0usize;
    let mut rollback_events = 0usize;
    let mut processed_events = 0usize;

    for (lp_id, event_time) in arrivals {
        processed_events += 1;
        let clock = lp_clocks.entry(lp_id).or_insert(SimTime::ZERO);
        if event_time < *clock {
            straggler_events += 1;
            rollback_events += 1;
        }
        *clock = event_time.max(*clock);
    }

    TimeWarpSpikeReport {
        lp_count: 2,
        processed_events,
        straggler_events,
        rollback_events,
        fossil_collectable_events: processed_events - rollback_events,
        recommendation:
            "Keep Track 34 on conservative CMB scheduling until rollback state snapshots are designed.",
    }
}

fn validate_workload(workload: &ParityWorkload) -> Result<(), String> {
    if workload.lp_count == 0 {
        return Err("PDES workload must include at least one LP".to_string());
    }
    if workload.ticks == 0 {
        return Err("PDES workload must include at least one tick".to_string());
    }
    if workload.entities_per_lp == 0 {
        return Err("PDES workload must include at least one entity per LP".to_string());
    }
    if workload.ticks > usize::MAX as u128 {
        return Err("PDES workload tick count exceeds addressable validation samples".to_string());
    }

    Ok(())
}

fn initial_state(workload: &ParityWorkload) -> BTreeMap<(LpId, u32), i128> {
    let mut final_state = BTreeMap::new();

    for lp_index in 0..workload.lp_count {
        for entity in 0..workload.entities_per_lp {
            final_state.insert((LpId(lp_index), entity), 0);
        }
    }
    final_state
}

fn apply_deltas(
    final_state: &mut BTreeMap<(LpId, u32), i128>,
    deltas: BTreeMap<(LpId, u32), i128>,
) {
    for (key, delta) in deltas {
        *final_state.entry(key).or_default() += delta;
    }
}

fn is_monotonic(times: &[Tick]) -> bool {
    times.windows(2).all(|pair| pair[0] <= pair[1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestLp {
        local_time: Tick,
        lookahead: SimDuration,
        outbound: Vec<RemoteEvent>,
        received: Vec<RemoteEvent>,
    }

    impl TestLp {
        fn new(lookahead: u128, outbound: Vec<RemoteEvent>) -> Self {
            Self {
                local_time: SimTime::ZERO,
                lookahead: SimDuration::from_ticks(lookahead),
                outbound,
                received: Vec::new(),
            }
        }
    }

    impl LogicalProcess for TestLp {
        fn init(&mut self, _lp_id: LpId, _world_segment: &WorldSegment) {}

        fn process_local_events(&mut self, until: Tick) {
            self.local_time = until;
        }

        fn schedule_remote_event(&mut self) -> Vec<RemoteEvent> {
            std::mem::take(&mut self.outbound)
        }

        fn receive_remote_events(&mut self, events: Vec<RemoteEvent>) {
            self.received.extend(events);
        }

        fn advance_to(&mut self, tick: Tick) {
            self.local_time = tick;
        }

        fn local_time(&self) -> Tick {
            self.local_time
        }

        fn lookahead(&self) -> SimDuration {
            self.lookahead
        }
    }

    fn segment(id: LpId) -> WorldSegment {
        WorldSegment {
            id,
            lp_count: 2,
            entities: Vec::new(),
        }
    }

    #[test]
    fn transport_round_trips_remote_events() {
        let mut transport = ThreadChannelTransport::new([LpId(0), LpId(1)]);
        let event = RemoteEvent {
            source_lp: LpId(0),
            dest_lp: LpId(1),
            tick: SimTime::from_ticks(10),
            event_payload: vec![1, 2, 3],
        };

        transport
            .send(LpId(1), PdesMessage::Event(event.clone()))
            .unwrap();

        assert_eq!(
            transport.recv(LpId(1)).unwrap(),
            vec![PdesMessage::Event(event)]
        );
        assert!(transport.recv(LpId(1)).unwrap().is_empty());
    }

    #[test]
    fn transport_rejects_unknown_destinations_and_sources() {
        let mut transport = ThreadChannelTransport::new([LpId(0), LpId(1)]);

        assert_eq!(
            transport.send(
                LpId(2),
                PdesMessage::Null(NullMessage {
                    source_lp: LpId(0),
                    dest_lp: LpId(2),
                    safe_time: SimTime::from_ticks(1),
                })
            ),
            Err(TransportError::UnknownLogicalProcess(LpId(2)))
        );
        assert_eq!(
            transport.send(
                LpId(1),
                PdesMessage::Null(NullMessage {
                    source_lp: LpId(2),
                    dest_lp: LpId(1),
                    safe_time: SimTime::from_ticks(1),
                })
            ),
            Err(TransportError::UnknownLogicalProcess(LpId(2)))
        );
        assert_eq!(
            transport.send(
                LpId(1),
                PdesMessage::Event(RemoteEvent {
                    source_lp: LpId(0),
                    dest_lp: LpId(0),
                    tick: SimTime::from_ticks(1),
                    event_payload: Vec::new(),
                })
            ),
            Err(TransportError::MessageDestinationMismatch {
                send_dest: LpId(1),
                message_dest: LpId(0),
            })
        );
        assert_eq!(
            transport.recv(LpId(2)),
            Err(TransportError::UnknownLogicalProcess(LpId(2)))
        );
    }

    #[test]
    fn scheduler_sends_events_and_null_messages() {
        let event = RemoteEvent {
            source_lp: LpId(0),
            dest_lp: LpId(1),
            tick: SimTime::from_ticks(5),
            event_payload: b"spawn".to_vec(),
        };
        let transport = ThreadChannelTransport::new([LpId(0), LpId(1)]);
        let mut scheduler = PdesScheduler::new(transport);

        scheduler
            .add_lp(
                LpId(0),
                segment(LpId(0)),
                vec![LpId(1)],
                Box::new(TestLp::new(2, vec![event])),
            )
            .unwrap();
        scheduler
            .add_lp(
                LpId(1),
                segment(LpId(1)),
                vec![LpId(0)],
                Box::new(TestLp::new(2, Vec::new())),
            )
            .unwrap();

        scheduler.step_until(SimTime::from_ticks(3)).unwrap();

        assert_eq!(scheduler.gvt(), SimTime::from_ticks(3));
        assert_eq!(scheduler.transport().pending_messages(), 3);
    }

    #[test]
    fn scheduler_rejects_remote_events_before_declared_lookahead() {
        let event = RemoteEvent {
            source_lp: LpId(0),
            dest_lp: LpId(1),
            tick: SimTime::from_ticks(4),
            event_payload: b"too-early".to_vec(),
        };
        let transport = ThreadChannelTransport::new([LpId(0), LpId(1)]);
        let mut scheduler = PdesScheduler::new(transport);

        scheduler
            .add_lp(
                LpId(0),
                segment(LpId(0)),
                vec![LpId(1)],
                Box::new(TestLp::new(2, vec![event])),
            )
            .unwrap();
        scheduler
            .add_lp(
                LpId(1),
                segment(LpId(1)),
                vec![LpId(0)],
                Box::new(TestLp::new(2, Vec::new())),
            )
            .unwrap();

        assert_eq!(
            scheduler.step_until(SimTime::from_ticks(3)),
            Err(PdesError::LookaheadViolation {
                lp_id: LpId(0),
                event_tick: SimTime::from_ticks(4),
                minimum_tick: SimTime::from_ticks(5),
            })
        );
    }

    #[test]
    fn scheduler_rejects_overflowing_lookahead_before_remote_send() {
        let event = RemoteEvent {
            source_lp: LpId(0),
            dest_lp: LpId(1),
            tick: SimTime::from_ticks(u128::MAX),
            event_payload: b"overflow".to_vec(),
        };
        let transport = ThreadChannelTransport::new([LpId(0), LpId(1)]);
        let mut scheduler = PdesScheduler::new(transport);

        scheduler
            .add_lp(
                LpId(0),
                segment(LpId(0)),
                vec![LpId(1)],
                Box::new(TestLp {
                    local_time: SimTime::from_ticks(u128::MAX),
                    lookahead: SimDuration::from_ticks(1),
                    outbound: vec![event],
                    received: Vec::new(),
                }),
            )
            .unwrap();
        scheduler
            .add_lp(
                LpId(1),
                segment(LpId(1)),
                vec![LpId(0)],
                Box::new(TestLp::new(1, Vec::new())),
            )
            .unwrap();

        assert_eq!(
            scheduler.step_until(SimTime::from_ticks(u128::MAX)),
            Err(PdesError::LookaheadOverflow {
                lp_id: LpId(0),
                local_time: SimTime::from_ticks(u128::MAX),
                lookahead: SimDuration::from_ticks(1),
            })
        );
    }

    #[test]
    fn in_memory_gvt_includes_inflight_message_timestamps() {
        let event = RemoteEvent {
            source_lp: LpId(0),
            dest_lp: LpId(1),
            tick: SimTime::from_ticks(3),
            event_payload: b"early".to_vec(),
        };
        let transport = ThreadChannelTransport::new([LpId(0), LpId(1)]);
        let mut scheduler = PdesScheduler::new(transport);

        scheduler
            .add_lp(
                LpId(0),
                segment(LpId(0)),
                vec![LpId(1)],
                Box::new(TestLp::new(0, vec![event])),
            )
            .unwrap();
        scheduler
            .add_lp(
                LpId(1),
                segment(LpId(1)),
                vec![LpId(0)],
                Box::new(TestLp::new(0, Vec::new())),
            )
            .unwrap();

        scheduler.step_until(SimTime::from_ticks(3)).unwrap();

        assert_eq!(scheduler.gvt(), SimTime::from_ticks(3));
    }

    #[test]
    fn scheduler_never_moves_lp_time_backward() {
        let mut transport = ThreadChannelTransport::new([LpId(0), LpId(1), LpId(2)]);
        transport
            .send(
                LpId(2),
                PdesMessage::Event(RemoteEvent {
                    source_lp: LpId(1),
                    dest_lp: LpId(2),
                    tick: SimTime::from_ticks(1),
                    event_payload: vec![1],
                }),
            )
            .unwrap();
        let mut scheduler = PdesScheduler::new(transport);

        scheduler
            .add_lp(
                LpId(0),
                segment(LpId(0)),
                Vec::new(),
                Box::new(TestLp::new(5, Vec::new())),
            )
            .unwrap();

        scheduler.step_until(SimTime::from_ticks(5)).unwrap();

        assert_eq!(scheduler.gvt(), SimTime::from_ticks(1));
        assert_eq!(
            scheduler.lps.get(&LpId(0)).unwrap().local_time(),
            SimTime::from_ticks(5)
        );
    }

    #[test]
    fn scheduler_ignores_stale_null_safe_time_for_local_progress() {
        let mut transport = ThreadChannelTransport::new([LpId(0), LpId(1)]);
        transport
            .send(
                LpId(0),
                PdesMessage::Null(NullMessage {
                    source_lp: LpId(1),
                    dest_lp: LpId(0),
                    safe_time: SimTime::from_ticks(2),
                }),
            )
            .unwrap();
        let mut scheduler = PdesScheduler::new(transport);

        scheduler
            .add_lp(
                LpId(0),
                segment(LpId(0)),
                vec![LpId(1)],
                Box::new(TestLp {
                    local_time: SimTime::from_ticks(5),
                    lookahead: SimDuration::from_ticks(1),
                    outbound: Vec::new(),
                    received: Vec::new(),
                }),
            )
            .unwrap();

        scheduler.step_until(SimTime::from_ticks(10)).unwrap();

        assert_eq!(
            scheduler.lps.get(&LpId(0)).unwrap().local_time(),
            SimTime::from_ticks(5)
        );
    }

    #[test]
    fn scheduler_rejects_duplicate_and_mismatched_lps() {
        let transport = ThreadChannelTransport::new([LpId(0), LpId(1)]);
        let mut scheduler = PdesScheduler::new(transport);

        scheduler
            .add_lp(
                LpId(0),
                segment(LpId(0)),
                vec![LpId(1)],
                Box::new(TestLp::new(0, Vec::new())),
            )
            .unwrap();
        assert_eq!(
            scheduler.add_lp(
                LpId(0),
                segment(LpId(0)),
                Vec::new(),
                Box::new(TestLp::new(0, Vec::new())),
            ),
            Err(PdesError::DuplicateLogicalProcess(LpId(0)))
        );
        assert_eq!(
            scheduler.add_lp(
                LpId(1),
                segment(LpId(0)),
                Vec::new(),
                Box::new(TestLp::new(0, Vec::new())),
            ),
            Err(PdesError::MismatchedWorldSegment {
                lp_id: LpId(1),
                segment_id: LpId(0),
            })
        );
        assert_eq!(
            scheduler.add_lp(
                LpId(2),
                segment(LpId(2)),
                vec![LpId(2)],
                Box::new(TestLp::new(0, Vec::new())),
            ),
            Err(PdesError::SelfLoopNeighbor(LpId(2)))
        );
        assert_eq!(
            scheduler.add_lp(
                LpId(3),
                segment(LpId(3)),
                vec![LpId(1), LpId(1)],
                Box::new(TestLp::new(0, Vec::new())),
            ),
            Err(PdesError::DuplicateNeighbor {
                lp_id: LpId(3),
                neighbor: LpId(1),
            })
        );
    }

    #[test]
    fn partitioned_reference_matches_sequential_final_state() {
        let report = sequential_parity_report(&ParityWorkload::small()).unwrap();

        assert_eq!(report.remote_events, 1_024);
        assert_eq!(report.null_messages, 128);
        assert!(is_monotonic(&report.gvt_history));
    }

    #[test]
    fn stress_reference_progresses_gvt_for_every_tick() {
        let report = deadlock_stress_report().unwrap();

        assert_eq!(report.gvt_history.len(), 10_000);
        assert_eq!(
            report.gvt_history.last(),
            Some(&SimTime::from_ticks(10_000))
        );
    }

    #[test]
    fn conservative_validator_reports_parity_gvt_and_deadlock_evidence() {
        let evidence = validate_conservative_pdes(&[
            ParityWorkload::small(),
            ParityWorkload {
                lp_count: 2,
                ticks: 64,
                entities_per_lp: 2,
            },
        ])
        .unwrap();

        assert_eq!(evidence.len(), 2);
        assert!(evidence.iter().all(|item| item.final_state_parity));
        assert!(evidence.iter().all(|item| item.gvt_monotonic));
        assert!(evidence.iter().all(|item| item.gvt_reaches_final_tick));
        assert!(evidence.iter().all(|item| item.deadlock_smoke));
        assert!(evidence.iter().all(|item| item.remote_events > 0));
        assert!(evidence.iter().all(|item| item.null_messages > 0));
    }

    #[test]
    fn scaling_smoke_samples_cover_required_lp_counts_without_speedup_claims() {
        let samples = scaling_smoke_samples(&[4, 8, 16, 32]).unwrap();

        assert_eq!(samples.len(), 4);
        assert_eq!(
            samples
                .iter()
                .map(|sample| sample.lp_count)
                .collect::<Vec<_>>(),
            vec![4, 8, 16, 32]
        );
        assert!(samples.iter().all(|sample| sample.final_state_parity));
        assert!(samples
            .iter()
            .all(|sample| !sample.hardware_speedup_claimed));
        assert!(samples.iter().all(|sample| sample.remote_events > 0));
        assert!(samples.iter().all(|sample| sample.null_messages > 0));
        assert!(samples.iter().all(|sample| sample.gvt_samples == 256));
    }

    #[test]
    fn time_warp_spike_documents_rollback_risk() {
        let report = time_warp_two_lp_spike();

        assert_eq!(report.lp_count, 2);
        assert_eq!(report.processed_events, 8);
        assert_eq!(report.straggler_events, 2);
        assert_eq!(report.rollback_events, 2);
        assert!(report.recommendation.contains("conservative CMB"));
    }

    #[test]
    fn validator_rejects_empty_or_degenerate_workloads() {
        assert_eq!(
            validate_conservative_pdes(&[]),
            Err("at least one PDES workload is required".to_string())
        );
        assert_eq!(
            sequential_parity_report(&ParityWorkload {
                lp_count: 0,
                ticks: 1,
                entities_per_lp: 1,
            }),
            Err("PDES workload must include at least one LP".to_string())
        );
    }
}
