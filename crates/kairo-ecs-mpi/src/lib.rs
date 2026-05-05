#![forbid(unsafe_code)]

use std::collections::{BTreeMap, VecDeque};

use kairo_ecs_pdes::{LpId, PdesMessage, PdesTransport, Tick};

/// MPI rank to logical-process mapping.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MpiRankAssignment {
    pub rank: i32,
    pub lp_id: LpId,
}

/// MPI message classes reserved for the distributed PDES protocol.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MpiMessageTag {
    Event = 100,
    Null = 101,
    Migration = 102,
    Telemetry = 103,
}

/// Placeholder transport preserving the Track 35 API without requiring MPI at build time.
#[derive(Debug)]
pub struct MpiTransport {
    rank: i32,
    assignments: Vec<MpiRankAssignment>,
    inboxes: BTreeMap<LpId, VecDeque<PdesMessage>>,
    reduction_candidates: Vec<Tick>,
    barrier_count: u64,
}

impl MpiTransport {
    pub fn new(rank: i32, assignments: Vec<MpiRankAssignment>) -> Self {
        let inboxes = assignments
            .iter()
            .map(|assignment| (assignment.lp_id, VecDeque::new()))
            .collect();

        Self {
            rank,
            assignments,
            inboxes,
            reduction_candidates: Vec::new(),
            barrier_count: 0,
        }
    }

    pub fn rank(&self) -> i32 {
        self.rank
    }

    pub fn assignments(&self) -> &[MpiRankAssignment] {
        &self.assignments
    }

    pub fn barrier_count(&self) -> u64 {
        self.barrier_count
    }

    pub fn pending_messages(&self) -> usize {
        self.inboxes.values().map(VecDeque::len).sum()
    }
}

impl PdesTransport for MpiTransport {
    fn send(&mut self, dest: LpId, message: PdesMessage) {
        self.inboxes.entry(dest).or_default().push_back(message);
    }

    fn recv(&mut self, lp_id: LpId) -> Vec<PdesMessage> {
        self.inboxes.entry(lp_id).or_default().drain(..).collect()
    }

    fn barrier(&mut self) {
        self.barrier_count += 1;
    }

    fn all_reduce_min(&mut self, timestamp: Tick) -> Tick {
        self.reduction_candidates.push(timestamp);
        self.reduction_candidates
            .iter()
            .copied()
            .min()
            .unwrap_or(timestamp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rank_assignment_preserves_lp_mapping() {
        let transport = MpiTransport::new(
            1,
            vec![
                MpiRankAssignment {
                    rank: 0,
                    lp_id: LpId(10),
                },
                MpiRankAssignment {
                    rank: 1,
                    lp_id: LpId(11),
                },
            ],
        );

        assert_eq!(transport.rank(), 1);
        assert_eq!(transport.assignments()[1].lp_id, LpId(11));
    }

    #[test]
    fn message_tags_are_stable() {
        assert_eq!(MpiMessageTag::Event as i32, 100);
        assert_eq!(MpiMessageTag::Telemetry as i32, 103);
    }

    #[test]
    fn protocol_emulator_round_trips_event_messages() {
        let mut transport = MpiTransport::new(
            0,
            vec![
                MpiRankAssignment {
                    rank: 0,
                    lp_id: LpId(0),
                },
                MpiRankAssignment {
                    rank: 1,
                    lp_id: LpId(1),
                },
            ],
        );
        let event = kairo_ecs_pdes::RemoteEvent {
            source_lp: LpId(0),
            dest_lp: LpId(1),
            tick: Tick::from_ticks(7),
            event_payload: b"mpi".to_vec(),
        };

        transport.send(LpId(1), PdesMessage::Event(event.clone()));
        transport.barrier();

        assert_eq!(transport.barrier_count(), 1);
        assert_eq!(transport.pending_messages(), 1);
        assert_eq!(transport.recv(LpId(1)), vec![PdesMessage::Event(event)]);
        assert_eq!(transport.pending_messages(), 0);
    }

    #[test]
    fn protocol_emulator_reduces_minimum_gvt_candidate() {
        let mut transport = MpiTransport::new(0, Vec::new());

        assert_eq!(
            transport.all_reduce_min(Tick::from_ticks(9)),
            Tick::from_ticks(9)
        );
        assert_eq!(
            transport.all_reduce_min(Tick::from_ticks(4)),
            Tick::from_ticks(4)
        );
        assert_eq!(
            transport.all_reduce_min(Tick::from_ticks(8)),
            Tick::from_ticks(4)
        );
    }
}
