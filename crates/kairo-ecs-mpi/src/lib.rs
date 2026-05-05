#![forbid(unsafe_code)]

use std::collections::{BTreeMap, VecDeque};

use kairo_ecs_pdes::{LpId, PdesMessage, PdesTransport, Tick};
use kairo_ecs_types::EntityId;

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

impl MpiMessageTag {
    pub fn validate_stable_values() -> Result<(), ProtocolValidationError> {
        let expected = [
            (Self::Event, 100),
            (Self::Null, 101),
            (Self::Migration, 102),
            (Self::Telemetry, 103),
        ];

        for (tag, value) in expected {
            if tag as i32 != value {
                return Err(ProtocolValidationError::UnstableMessageTag {
                    tag: tag as i32,
                    expected: value,
                });
            }
        }

        Ok(())
    }
}

/// Serialized component state carried by an entity migration message.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MpiComponentBlob {
    pub component_type_id: String,
    pub payload: Vec<u8>,
}

/// Dependency-free migration envelope used by local protocol validation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MpiMigrationRequest {
    pub entity: EntityId,
    pub source_lp: LpId,
    pub dest_lp: LpId,
    pub migration_id: String,
    pub components: Vec<MpiComponentBlob>,
}

impl MpiMigrationRequest {
    pub fn validate(&self) -> Result<(), ProtocolValidationError> {
        validate_migration_envelope(
            self.source_lp,
            self.dest_lp,
            &self.migration_id,
            self.components.iter().map(|component| {
                (
                    component.component_type_id.as_str(),
                    component.payload.as_slice(),
                )
            }),
        )
    }
}

/// Telemetry payload envelope used before Track 04 Arrow runtime wiring lands.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MpiTelemetryBatch {
    pub source_lp: LpId,
    pub tick_start: Tick,
    pub tick_end: Tick,
    pub arrow_ipc_payload: Vec<u8>,
}

impl MpiTelemetryBatch {
    pub fn validate(&self) -> Result<(), ProtocolValidationError> {
        if self.tick_start > self.tick_end {
            return Err(ProtocolValidationError::InvalidTickRange);
        }
        if self.arrow_ipc_payload.is_empty() {
            return Err(ProtocolValidationError::EmptyTelemetryPayload);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProtocolValidationError {
    DuplicateRank(i32),
    DuplicateLp(LpId),
    InvalidLocalRank(i32),
    InvalidMigrationId,
    SelfMigration(LpId),
    EmptyComponentSet,
    InvalidComponentTypeId,
    EmptyComponentPayload(String),
    InvalidTickRange,
    EmptyTelemetryPayload,
    UnstableMessageTag { tag: i32, expected: i32 },
}

pub fn validate_rank_assignments(
    local_rank: i32,
    assignments: &[MpiRankAssignment],
) -> Result<(), ProtocolValidationError> {
    let mut ranks = Vec::new();
    let mut lps = Vec::new();
    let mut has_local_rank = false;

    for assignment in assignments {
        if assignment.rank == local_rank {
            has_local_rank = true;
        }
        if ranks.contains(&assignment.rank) {
            return Err(ProtocolValidationError::DuplicateRank(assignment.rank));
        }
        if lps.contains(&assignment.lp_id) {
            return Err(ProtocolValidationError::DuplicateLp(assignment.lp_id));
        }
        ranks.push(assignment.rank);
        lps.push(assignment.lp_id);
    }

    if !assignments.is_empty() && !has_local_rank {
        return Err(ProtocolValidationError::InvalidLocalRank(local_rank));
    }

    Ok(())
}

fn validate_migration_envelope<'a>(
    source_lp: LpId,
    dest_lp: LpId,
    migration_id: &str,
    components: impl IntoIterator<Item = (&'a str, &'a [u8])>,
) -> Result<(), ProtocolValidationError> {
    if migration_id.trim().is_empty() {
        return Err(ProtocolValidationError::InvalidMigrationId);
    }
    if source_lp == dest_lp {
        return Err(ProtocolValidationError::SelfMigration(source_lp));
    }

    let mut saw_component = false;
    for (component_type_id, payload) in components {
        saw_component = true;
        if component_type_id.trim().is_empty() {
            return Err(ProtocolValidationError::InvalidComponentTypeId);
        }
        if payload.is_empty() {
            return Err(ProtocolValidationError::EmptyComponentPayload(
                component_type_id.to_string(),
            ));
        }
    }

    if !saw_component {
        return Err(ProtocolValidationError::EmptyComponentSet);
    }

    Ok(())
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

    pub fn validate_protocol(&self) -> Result<(), ProtocolValidationError> {
        MpiMessageTag::validate_stable_values()?;
        validate_rank_assignments(self.rank, &self.assignments)
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
        assert_eq!(MpiMessageTag::validate_stable_values(), Ok(()));
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

    #[test]
    fn local_smoke_validates_rank_mapping_and_protocol_tags() {
        let transport = MpiTransport::new(
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

        assert_eq!(transport.validate_protocol(), Ok(()));
    }

    #[test]
    fn rank_validator_rejects_duplicate_lps() {
        let err = validate_rank_assignments(
            0,
            &[
                MpiRankAssignment {
                    rank: 0,
                    lp_id: LpId(0),
                },
                MpiRankAssignment {
                    rank: 1,
                    lp_id: LpId(0),
                },
            ],
        )
        .unwrap_err();

        assert_eq!(err, ProtocolValidationError::DuplicateLp(LpId(0)));
    }

    #[test]
    fn migration_validator_accepts_component_payloads() {
        let request = MpiMigrationRequest {
            entity: EntityId {
                index: 42,
                generation: 2,
            },
            source_lp: LpId(0),
            dest_lp: LpId(1),
            migration_id: "mig-42".to_string(),
            components: vec![
                MpiComponentBlob {
                    component_type_id: "position".to_string(),
                    payload: vec![1, 2, 3],
                },
                MpiComponentBlob {
                    component_type_id: "velocity".to_string(),
                    payload: vec![4, 5, 6],
                },
            ],
        };

        assert_eq!(request.validate(), Ok(()));
    }

    #[test]
    fn migration_validator_rejects_self_migration() {
        let request = MpiMigrationRequest {
            entity: EntityId {
                index: 42,
                generation: 2,
            },
            source_lp: LpId(1),
            dest_lp: LpId(1),
            migration_id: "mig-42".to_string(),
            components: vec![MpiComponentBlob {
                component_type_id: "position".to_string(),
                payload: vec![1],
            }],
        };

        assert_eq!(
            request.validate(),
            Err(ProtocolValidationError::SelfMigration(LpId(1)))
        );
    }

    #[test]
    fn telemetry_batch_validator_requires_ordered_non_empty_payload() {
        let batch = MpiTelemetryBatch {
            source_lp: LpId(2),
            tick_start: Tick::from_ticks(3),
            tick_end: Tick::from_ticks(5),
            arrow_ipc_payload: b"arrow-ipc".to_vec(),
        };

        assert_eq!(batch.validate(), Ok(()));

        let empty = MpiTelemetryBatch {
            arrow_ipc_payload: Vec::new(),
            ..batch
        };
        assert_eq!(
            empty.validate(),
            Err(ProtocolValidationError::EmptyTelemetryPayload)
        );
    }
}
