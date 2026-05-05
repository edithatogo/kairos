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
    fn send(&mut self, dest: LpId, message: PdesMessage);

    fn recv(&mut self, lp_id: LpId) -> Vec<PdesMessage>;

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
}

impl PdesTransport for ThreadChannelTransport {
    fn send(&mut self, dest: LpId, message: PdesMessage) {
        self.inboxes.entry(dest).or_default().push_back(message);
    }

    fn recv(&mut self, lp_id: LpId) -> Vec<PdesMessage> {
        self.inboxes.entry(lp_id).or_default().drain(..).collect()
    }

    fn barrier(&mut self) {}

    fn all_reduce_min(&mut self, timestamp: Tick) -> Tick {
        self.gvt_candidates.push(timestamp);
        self.gvt_candidates
            .iter()
            .copied()
            .min()
            .unwrap_or(timestamp)
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
    ) {
        lp.init(lp_id, &world_segment);
        self.neighbors.insert(lp_id, neighbors);
        self.lps.insert(lp_id, lp);
    }

    pub fn gvt(&self) -> Tick {
        self.gvt
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn step_until(&mut self, until: Tick) {
        let lp_ids: Vec<LpId> = self.lps.keys().copied().collect();

        for lp_id in &lp_ids {
            let inbound = self.transport.recv(*lp_id);
            let events = inbound
                .into_iter()
                .filter_map(|message| match message {
                    PdesMessage::Event(event) => Some(event),
                    PdesMessage::Null(_) => None,
                })
                .collect();

            if let Some(lp) = self.lps.get_mut(lp_id) {
                lp.receive_remote_events(events);
                lp.process_local_events(until);
            }
        }

        for lp_id in &lp_ids {
            if let Some(lp) = self.lps.get_mut(lp_id) {
                for event in lp.schedule_remote_event() {
                    self.transport
                        .send(event.dest_lp, PdesMessage::Event(event));
                }

                let safe_time = lp
                    .local_time()
                    .checked_add(lp.lookahead())
                    .unwrap_or(lp.local_time());
                if let Some(neighbors) = self.neighbors.get(lp_id) {
                    for neighbor in neighbors {
                        self.transport.send(
                            *neighbor,
                            PdesMessage::Null(NullMessage {
                                source_lp: *lp_id,
                                dest_lp: *neighbor,
                                safe_time,
                            }),
                        );
                    }
                }
            }
        }

        self.transport.barrier();
        self.gvt = self.compute_gvt();

        for lp in self.lps.values_mut() {
            lp.advance_to(self.gvt);
        }
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

        transport.send(LpId(1), PdesMessage::Event(event.clone()));

        assert_eq!(transport.recv(LpId(1)), vec![PdesMessage::Event(event)]);
        assert!(transport.recv(LpId(1)).is_empty());
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

        scheduler.add_lp(
            LpId(0),
            segment(LpId(0)),
            vec![LpId(1)],
            Box::new(TestLp::new(2, vec![event])),
        );
        scheduler.add_lp(
            LpId(1),
            segment(LpId(1)),
            vec![LpId(0)],
            Box::new(TestLp::new(2, Vec::new())),
        );

        scheduler.step_until(SimTime::from_ticks(3));

        assert_eq!(scheduler.gvt(), SimTime::from_ticks(3));
        assert_eq!(scheduler.transport().pending_messages(), 3);
    }
}
