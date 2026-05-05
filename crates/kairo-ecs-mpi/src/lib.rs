#![forbid(unsafe_code)]

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
}

impl MpiTransport {
    pub fn new(rank: i32, assignments: Vec<MpiRankAssignment>) -> Self {
        Self { rank, assignments }
    }

    pub fn rank(&self) -> i32 {
        self.rank
    }

    pub fn assignments(&self) -> &[MpiRankAssignment] {
        &self.assignments
    }
}

impl PdesTransport for MpiTransport {
    fn send(&mut self, _dest: LpId, _message: PdesMessage) {
        panic!("MpiTransport::send requires the future rsmpi backend");
    }

    fn recv(&mut self, _lp_id: LpId) -> Vec<PdesMessage> {
        panic!("MpiTransport::recv requires the future rsmpi backend");
    }

    fn barrier(&mut self) {
        panic!("MpiTransport::barrier requires the future rsmpi backend");
    }

    fn all_reduce_min(&mut self, _timestamp: Tick) -> Tick {
        panic!("MpiTransport::all_reduce_min requires the future rsmpi backend");
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
}
