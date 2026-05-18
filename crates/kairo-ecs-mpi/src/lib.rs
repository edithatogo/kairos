#![forbid(unsafe_code)]

use std::collections::{BTreeMap, VecDeque};

use kairo_ecs_pdes::{LpId, PdesMessage, PdesTransport, Tick, TransportError};
use kairo_ecs_types::EntityId;

/// Dependency-free placeholder protocol identity for the local transport contract.
pub const MPI_PROTOCOL_ID: &str = "kairo.ecs.distributed.mpi.v1";
pub const MPI_PROTOCOL_VERSION: u16 = 1;

/// MPI rank to logical-process mapping.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MpiRankAssignment {
    pub rank: i32,
    pub lp_id: LpId,
}

/// Contract-level message class used by the MPI placeholder transport.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MpiContractMessage {
    Event,
    Null,
    Migration,
    Telemetry,
}

impl MpiContractMessage {
    pub fn as_tag(self) -> MpiMessageTag {
        match self {
            Self::Event => MpiMessageTag::Event,
            Self::Null => MpiMessageTag::Null,
            Self::Migration => MpiMessageTag::Migration,
            Self::Telemetry => MpiMessageTag::Telemetry,
        }
    }

    pub fn from_tag(tag: i32) -> Option<Self> {
        match tag {
            x if x == MpiMessageTag::Event as i32 => Some(Self::Event),
            x if x == MpiMessageTag::Null as i32 => Some(Self::Null),
            x if x == MpiMessageTag::Migration as i32 => Some(Self::Migration),
            x if x == MpiMessageTag::Telemetry as i32 => Some(Self::Telemetry),
            _ => None,
        }
    }
}

/// Dependency-free wire envelope for compile-time contract verification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MpiContractEnvelope {
    pub protocol_id: &'static str,
    pub protocol_version: u16,
    pub kind: MpiContractMessage,
    pub source_rank: i32,
    pub destination_rank: i32,
    pub source_lp: LpId,
    pub destination_lp: LpId,
    pub tick: Tick,
    pub tick_end: Option<Tick>,
    pub migration_id: Option<String>,
    pub payload_bytes: usize,
}

impl MpiContractEnvelope {
    pub fn event(
        source_rank: i32,
        destination_rank: i32,
        source_lp: LpId,
        destination_lp: LpId,
        tick: Tick,
        payload_len: usize,
    ) -> Self {
        Self {
            protocol_id: MPI_PROTOCOL_ID,
            protocol_version: MPI_PROTOCOL_VERSION,
            kind: MpiContractMessage::Event,
            source_rank,
            destination_rank,
            source_lp,
            destination_lp,
            tick,
            tick_end: None,
            migration_id: None,
            payload_bytes: payload_len,
        }
    }

    pub fn null(
        source_rank: i32,
        destination_rank: i32,
        source_lp: LpId,
        destination_lp: LpId,
        safe_time: Tick,
    ) -> Self {
        Self {
            protocol_id: MPI_PROTOCOL_ID,
            protocol_version: MPI_PROTOCOL_VERSION,
            kind: MpiContractMessage::Null,
            source_rank,
            destination_rank,
            source_lp,
            destination_lp,
            tick: safe_time,
            migration_id: None,
            tick_end: None,
            payload_bytes: 0,
        }
    }

    pub fn migration(
        source_rank: i32,
        destination_rank: i32,
        source_lp: LpId,
        destination_lp: LpId,
        migration_id: String,
        payload_len: usize,
    ) -> Self {
        Self {
            protocol_id: MPI_PROTOCOL_ID,
            protocol_version: MPI_PROTOCOL_VERSION,
            kind: MpiContractMessage::Migration,
            source_rank,
            destination_rank,
            source_lp,
            destination_lp,
            tick: Tick::from_ticks(0),
            tick_end: None,
            migration_id: Some(migration_id),
            payload_bytes: payload_len,
        }
    }

    pub fn telemetry(
        source_rank: i32,
        destination_rank: i32,
        source_lp: LpId,
        destination_lp: LpId,
        tick_start: Tick,
        tick_end: Tick,
        payload_len: usize,
    ) -> Self {
        Self {
            protocol_id: MPI_PROTOCOL_ID,
            protocol_version: MPI_PROTOCOL_VERSION,
            kind: MpiContractMessage::Telemetry,
            source_rank,
            destination_rank,
            source_lp,
            destination_lp,
            tick: tick_start,
            tick_end: Some(tick_end),
            migration_id: None,
            payload_bytes: payload_len,
        }
    }

    pub fn validate(&self) -> Result<(), ProtocolValidationError> {
        if self.protocol_id != MPI_PROTOCOL_ID {
            return Err(ProtocolValidationError::ProtocolMismatch);
        }

        if self.protocol_version != MPI_PROTOCOL_VERSION {
            return Err(ProtocolValidationError::ProtocolVersionMismatch {
                expected: MPI_PROTOCOL_VERSION,
                got: self.protocol_version,
            });
        }

        if self.source_rank < 0 || self.destination_rank < 0 {
            return Err(ProtocolValidationError::InvalidLocalRank(self.source_rank));
        }

        match self.kind {
            MpiContractMessage::Migration => {
                if self.migration_id.as_deref().unwrap_or("").trim().is_empty() {
                    return Err(ProtocolValidationError::InvalidMigrationId);
                }
                if self.payload_bytes == 0 {
                    return Err(ProtocolValidationError::EmptyComponentPayload(
                        "migration".to_string(),
                    ));
                }
            }
            MpiContractMessage::Telemetry => {
                if let Some(tick_end) = self.tick_end {
                    if tick_end < self.tick {
                        return Err(ProtocolValidationError::InvalidTickRange);
                    }
                }
                if self.payload_bytes == 0 {
                    return Err(ProtocolValidationError::EmptyTelemetryPayload);
                }
            }
            MpiContractMessage::Event | MpiContractMessage::Null => {}
        }

        if self.source_lp == self.destination_lp && !matches!(self.kind, MpiContractMessage::Null) {
            return Err(ProtocolValidationError::SelfMigration(self.source_lp));
        }

        Ok(())
    }
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
pub struct MpiLocalTwoRankProof {
    pub exchanged_events: usize,
    pub migrations_validated: usize,
    pub telemetry_batches_merged: usize,
    pub gvt_floor: Tick,
    pub final_state_parity: bool,
    pub real_mpi_runtime_claimed: bool,
}

pub fn local_two_rank_contract_proof() -> Result<MpiLocalTwoRankProof, ProtocolValidationError> {
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
    transport.validate_protocol()?;

    let event = kairo_ecs_pdes::RemoteEvent {
        source_lp: LpId(0),
        dest_lp: LpId(1),
        tick: Tick::from_ticks(3),
        event_payload: b"rank0-to-rank1".to_vec(),
    };
    transport
        .send(LpId(1), PdesMessage::Event(event.clone()))
        .map_err(|_| ProtocolValidationError::UnknownTransportLp)?;

    let migration = MpiMigrationRequest {
        entity: EntityId {
            index: 7,
            generation: 1,
        },
        source_lp: LpId(0),
        dest_lp: LpId(1),
        migration_id: "mpi-local-proof-7".to_string(),
        components: vec![MpiComponentBlob {
            component_type_id: "position".to_string(),
            payload: vec![1, 2, 3, 4],
        }],
    };
    migration.validate()?;

    let telemetry = [
        MpiTelemetryBatch {
            source_lp: LpId(0),
            tick_start: Tick::from_ticks(0),
            tick_end: Tick::from_ticks(3),
            arrow_ipc_payload: b"arrow-rank0".to_vec(),
        },
        MpiTelemetryBatch {
            source_lp: LpId(1),
            tick_start: Tick::from_ticks(0),
            tick_end: Tick::from_ticks(3),
            arrow_ipc_payload: b"arrow-rank1".to_vec(),
        },
    ];
    for batch in &telemetry {
        batch.validate()?;
    }

    let received = transport
        .recv(LpId(1))
        .map_err(|_| ProtocolValidationError::UnknownTransportLp)?;
    let exchanged_events = received
        .iter()
        .filter(|message| matches!(message, PdesMessage::Event(_)))
        .count();

    Ok(MpiLocalTwoRankProof {
        exchanged_events,
        migrations_validated: 1,
        telemetry_batches_merged: telemetry.len(),
        gvt_floor: transport.all_reduce_min(Tick::from_ticks(3)),
        final_state_parity: received == vec![PdesMessage::Event(event)],
        real_mpi_runtime_claimed: false,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProtocolValidationError {
    DuplicateRank(i32),
    DuplicateLp(LpId),
    InvalidLocalRank(i32),
    ProtocolMismatch,
    ProtocolVersionMismatch { expected: u16, got: u16 },
    InvalidMigrationId,
    SelfMigration(LpId),
    EmptyComponentSet,
    InvalidComponentTypeId,
    EmptyComponentPayload(String),
    InvalidTickRange,
    EmptyTelemetryPayload,
    UnstableMessageTag { tag: i32, expected: i32 },
    UnknownTransportLp,
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

    pub fn validate_protocol(&self) -> Result<(), ProtocolValidationError> {
        MpiMessageTag::validate_stable_values()?;
        validate_rank_assignments(self.rank, &self.assignments)
    }
}

impl PdesTransport for MpiTransport {
    fn knows_lp(&self, lp_id: LpId) -> bool {
        self.inboxes.contains_key(&lp_id)
    }

    fn send(&mut self, dest: LpId, message: PdesMessage) -> Result<(), TransportError> {
        if !self.inboxes.contains_key(&dest) {
            return Err(TransportError::UnknownLogicalProcess(dest));
        }
        let (source_lp, message_dest) = match &message {
            PdesMessage::Event(event) => (event.source_lp, event.dest_lp),
            PdesMessage::Null(message) => (message.source_lp, message.dest_lp),
        };
        if !self.inboxes.contains_key(&source_lp) {
            return Err(TransportError::UnknownLogicalProcess(source_lp));
        }
        if message_dest != dest {
            return Err(TransportError::MessageDestinationMismatch {
                send_dest: dest,
                message_dest,
            });
        }

        self.inboxes.entry(dest).or_default().push_back(message);
        Ok(())
    }

    fn recv(&mut self, lp_id: LpId) -> Result<Vec<PdesMessage>, TransportError> {
        if !self.inboxes.contains_key(&lp_id) {
            return Err(TransportError::UnknownLogicalProcess(lp_id));
        }

        Ok(self.inboxes.entry(lp_id).or_default().drain(..).collect())
    }

    fn barrier(&mut self) {
        self.barrier_count += 1;
    }

    fn all_reduce_min(&mut self, timestamp: Tick) -> Tick {
        self.reduction_candidates.clear();
        self.reduction_candidates.push(timestamp);

        self.pending_min_timestamp()
            .map_or(timestamp, |pending_min| timestamp.min(pending_min))
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

        transport
            .send(LpId(1), PdesMessage::Event(event.clone()))
            .expect("MPI placeholder transport should accept known LP");
        transport.barrier();

        assert_eq!(transport.barrier_count(), 1);
        assert_eq!(transport.pending_messages(), 1);
        assert_eq!(
            transport
                .recv(LpId(1))
                .expect("MPI placeholder transport should receive from known LP"),
            vec![PdesMessage::Event(event)]
        );
        assert_eq!(transport.pending_messages(), 0);
    }

    #[test]
    fn protocol_emulator_uses_current_gvt_candidate_round() {
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
            Tick::from_ticks(8)
        );
    }

    #[test]
    fn protocol_emulator_includes_pending_event_timestamps_in_gvt_round() {
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
        let pending_event = kairo_ecs_pdes::RemoteEvent {
            source_lp: LpId(0),
            dest_lp: LpId(1),
            tick: Tick::from_ticks(3),
            event_payload: b"pending-gvt".to_vec(),
        };

        transport
            .send(LpId(1), PdesMessage::Event(pending_event.clone()))
            .expect("known LP should accept queued event");
        transport
            .send(
                LpId(1),
                PdesMessage::Null(kairo_ecs_pdes::NullMessage {
                    source_lp: LpId(0),
                    dest_lp: LpId(1),
                    safe_time: Tick::from_ticks(1),
                }),
            )
            .expect("known LP should accept queued null message");

        assert_eq!(
            transport.all_reduce_min(Tick::from_ticks(9)),
            pending_event.tick
        );
        assert_eq!(
            transport
                .recv(LpId(1))
                .expect("known LP should drain queued messages")
                .len(),
            2
        );
        assert_eq!(
            transport.all_reduce_min(Tick::from_ticks(9)),
            Tick::from_ticks(9)
        );
    }

    #[test]
    fn transport_contracts_use_stable_mpi_protocol_metadata() {
        let envelope = MpiContractEnvelope::event(0, 1, LpId(0), LpId(1), Tick::from_ticks(7), 4);

        assert_eq!(envelope.protocol_id, MPI_PROTOCOL_ID);
        assert_eq!(envelope.protocol_version, MPI_PROTOCOL_VERSION);
        assert_eq!(
            MpiContractMessage::from_tag(envelope.kind.as_tag() as i32),
            Some(MpiContractMessage::Event)
        );
        assert_eq!(envelope.validate(), Ok(()));
    }

    #[test]
    fn transport_contract_rejects_migration_without_payload_or_id() {
        let err = MpiContractEnvelope::migration(0, 1, LpId(0), LpId(1), String::new(), 0)
            .validate()
            .unwrap_err();

        assert_eq!(err, ProtocolValidationError::InvalidMigrationId);
    }

    #[test]
    fn transport_send_rejects_unknown_lp() {
        let mut transport = MpiTransport::new(
            0,
            vec![MpiRankAssignment {
                rank: 0,
                lp_id: LpId(0),
            }],
        );

        assert_eq!(
            transport.send(
                LpId(1),
                PdesMessage::Null(kairo_ecs_pdes::NullMessage {
                    source_lp: LpId(0),
                    dest_lp: LpId(1),
                    safe_time: Tick::from_ticks(11),
                })
            ),
            Err(TransportError::UnknownLogicalProcess(LpId(1)))
        );
    }

    #[test]
    fn transport_send_rejects_unknown_source_and_destination_mismatch() {
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

        assert_eq!(
            transport.send(
                LpId(1),
                PdesMessage::Null(kairo_ecs_pdes::NullMessage {
                    source_lp: LpId(2),
                    dest_lp: LpId(1),
                    safe_time: Tick::from_ticks(11),
                })
            ),
            Err(TransportError::UnknownLogicalProcess(LpId(2)))
        );

        assert_eq!(
            transport.send(
                LpId(1),
                PdesMessage::Event(kairo_ecs_pdes::RemoteEvent {
                    source_lp: LpId(0),
                    dest_lp: LpId(0),
                    tick: Tick::from_ticks(11),
                    event_payload: vec![1, 2, 3],
                })
            ),
            Err(TransportError::MessageDestinationMismatch {
                send_dest: LpId(1),
                message_dest: LpId(0),
            })
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

    #[test]
    fn local_two_rank_contract_proof_covers_event_migration_and_telemetry() {
        let proof = local_two_rank_contract_proof().unwrap();

        assert_eq!(proof.exchanged_events, 1);
        assert_eq!(proof.migrations_validated, 1);
        assert_eq!(proof.telemetry_batches_merged, 2);
        assert_eq!(proof.gvt_floor, Tick::from_ticks(3));
        assert!(proof.final_state_parity);
        assert!(!proof.real_mpi_runtime_claimed);
    }
}
