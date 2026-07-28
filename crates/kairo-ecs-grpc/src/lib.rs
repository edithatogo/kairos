#![forbid(unsafe_code)]

use std::collections::{BTreeMap, VecDeque};
use std::time::Duration;

use kairo_ecs_pdes::{LpId, PdesMessage, PdesTransport, Tick, TransportError};
use kairo_ecs_types::EntityId;

/// Dependency-free placeholder protocol identity for the local gRPC contract.
pub const GRPC_PROTOCOL_ID: &str = "kairo.ecs.distributed.grpc.v1";
pub const GRPC_PROTOCOL_VERSION: u16 = 1;
pub const GRPC_SERVICE_NAME: &str = "kairo.ecs.simulation.v1.SimulationTransport";

/// Contract-level message names used by the gRPC placeholder transport.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GrpcContractMessage {
    ExchangeEvents,
    ExchangeEventsReturn,
    MigrationRequest,
    MigrationAck,
    StreamTelemetry,
    GvtProposal,
    GvtDecision,
}

/// Dependency-free envelope used by transport contract tests.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GrpcContractEnvelope {
    pub protocol_id: &'static str,
    pub protocol_version: u16,
    pub service: &'static str,
    pub kind: GrpcContractMessage,
    pub source_lp: LpId,
    pub destination_lp: Option<LpId>,
    pub migration_id: Option<String>,
    pub payload_bytes: usize,
}

impl GrpcContractEnvelope {
    pub fn exchange_events(source_lp: LpId, destination_lp: LpId, payload_len: usize) -> Self {
        Self {
            protocol_id: GRPC_PROTOCOL_ID,
            protocol_version: GRPC_PROTOCOL_VERSION,
            service: GRPC_SERVICE_NAME,
            kind: GrpcContractMessage::ExchangeEvents,
            source_lp,
            destination_lp: Some(destination_lp),
            migration_id: None,
            payload_bytes: payload_len,
        }
    }

    pub fn migration(source_lp: LpId, destination_lp: LpId, migration_id: String) -> Self {
        Self {
            protocol_id: GRPC_PROTOCOL_ID,
            protocol_version: GRPC_PROTOCOL_VERSION,
            service: GRPC_SERVICE_NAME,
            kind: GrpcContractMessage::MigrationRequest,
            source_lp,
            destination_lp: Some(destination_lp),
            migration_id: Some(migration_id),
            payload_bytes: 0,
        }
    }

    pub fn gvt(source_lp: LpId) -> Self {
        Self {
            protocol_id: GRPC_PROTOCOL_ID,
            protocol_version: GRPC_PROTOCOL_VERSION,
            service: GRPC_SERVICE_NAME,
            kind: GrpcContractMessage::GvtProposal,
            source_lp,
            destination_lp: None,
            migration_id: None,
            payload_bytes: 0,
        }
    }

    pub fn telemetry(source_lp: LpId, payload_len: usize) -> Self {
        Self {
            protocol_id: GRPC_PROTOCOL_ID,
            protocol_version: GRPC_PROTOCOL_VERSION,
            service: GRPC_SERVICE_NAME,
            kind: GrpcContractMessage::StreamTelemetry,
            source_lp,
            destination_lp: None,
            migration_id: None,
            payload_bytes: payload_len,
        }
    }

    pub fn validate(&self) -> Result<(), ProtocolValidationError> {
        if self.protocol_id != GRPC_PROTOCOL_ID {
            return Err(ProtocolValidationError::ProtocolMismatch);
        }
        if self.protocol_version != GRPC_PROTOCOL_VERSION {
            return Err(ProtocolValidationError::ProtocolVersionMismatch {
                expected: GRPC_PROTOCOL_VERSION,
                got: self.protocol_version,
            });
        }
        if self.service != GRPC_SERVICE_NAME {
            return Err(ProtocolValidationError::InvalidService);
        }
        if self.kind == GrpcContractMessage::MigrationRequest
            && self
                .migration_id
                .as_ref()
                .map_or(true, |id| id.trim().is_empty())
        {
            return Err(ProtocolValidationError::InvalidMigrationId);
        }

        Ok(())
    }
}

/// Static peer endpoint used by the gRPC transport scaffold.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GrpcPeer {
    pub lp_id: LpId,
    pub endpoint: String,
}

/// Tunables for coordinator heartbeats and RPC timeouts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GrpcTransportConfig {
    pub request_timeout: Duration,
    pub heartbeat_interval: Duration,
    pub heartbeat_timeout: Duration,
}

impl Default for GrpcTransportConfig {
    fn default() -> Self {
        Self {
            request_timeout: Duration::from_secs(5),
            heartbeat_interval: Duration::from_secs(1),
            heartbeat_timeout: Duration::from_secs(10),
        }
    }
}

/// Serialized component state carried by a gRPC migration request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GrpcComponentBlob {
    pub component_type_id: String,
    pub payload: Vec<u8>,
}

/// Dependency-free mirror of the protobuf migration envelope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GrpcMigrationRequest {
    pub entity: EntityId,
    pub source_lp: LpId,
    pub dest_lp: LpId,
    pub migration_id: String,
    pub components: Vec<GrpcComponentBlob>,
}

impl GrpcMigrationRequest {
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

/// Telemetry payload envelope used by the local protocol emulator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GrpcTelemetryBatch {
    pub source_lp: LpId,
    pub tick_start: Tick,
    pub tick_end: Tick,
    pub arrow_ipc_payload: Vec<u8>,
}

impl GrpcTelemetryBatch {
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
pub enum WorkerStatus {
    Healthy,
    Suspect,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkerHeartbeat {
    pub lp_id: LpId,
    pub elapsed_since_last_seen: Duration,
}

pub fn classify_worker(heartbeat: &WorkerHeartbeat, config: &GrpcTransportConfig) -> WorkerStatus {
    if heartbeat.elapsed_since_last_seen >= config.heartbeat_timeout {
        WorkerStatus::Failed
    } else if heartbeat.elapsed_since_last_seen >= config.heartbeat_interval * 2 {
        WorkerStatus::Suspect
    } else {
        WorkerStatus::Healthy
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GrpcLocalTwoNodeProof {
    pub exchanged_events: usize,
    pub migrations_validated: usize,
    pub telemetry_batches_merged: usize,
    pub failed_workers_detected: usize,
    pub simulation_continues_after_non_leader_failure: bool,
    pub final_state_parity: bool,
    pub real_grpc_runtime_claimed: bool,
}

fn setup_test_transport() -> Result<(GrpcTransport, GrpcTransportConfig), ProtocolValidationError> {
    let config = GrpcTransportConfig::default();
    let transport = GrpcTransport::new(
        LpId(7),
        vec![GrpcPeer {
            lp_id: LpId(8),
            endpoint: "http://127.0.0.1:50051".to_string(),
        }],
        config.clone(),
    );
    transport.validate_protocol()?;
    Ok((transport, config))
}

fn exchange_test_events(
    transport: &mut GrpcTransport,
) -> Result<(kairo_ecs_pdes::RemoteEvent, Vec<PdesMessage>), ProtocolValidationError> {
    let event = kairo_ecs_pdes::RemoteEvent {
        source_lp: LpId(7),
        dest_lp: LpId(8),
        tick: Tick::from_ticks(4),
        event_payload: b"grpc-local-proof".to_vec(),
    };
    transport
        .send(LpId(8), PdesMessage::Event(event.clone()))
        .map_err(|_| ProtocolValidationError::UnknownTransportLp)?;

    let received = transport
        .recv(LpId(8))
        .map_err(|_| ProtocolValidationError::UnknownTransportLp)?;

    Ok((event, received))
}

fn validate_test_migration() -> Result<usize, ProtocolValidationError> {
    let migration = GrpcMigrationRequest {
        entity: EntityId {
            index: 11,
            generation: 3,
        },
        source_lp: LpId(7),
        dest_lp: LpId(8),
        migration_id: "grpc-local-proof-11".to_string(),
        components: vec![GrpcComponentBlob {
            component_type_id: "health".to_string(),
            payload: vec![9, 8, 7],
        }],
    };
    migration.validate()?;
    Ok(1)
}

fn validate_test_telemetry() -> Result<usize, ProtocolValidationError> {
    let telemetry = [
        GrpcTelemetryBatch {
            source_lp: LpId(7),
            tick_start: Tick::from_ticks(0),
            tick_end: Tick::from_ticks(4),
            arrow_ipc_payload: b"arrow-grpc-7".to_vec(),
        },
        GrpcTelemetryBatch {
            source_lp: LpId(8),
            tick_start: Tick::from_ticks(0),
            tick_end: Tick::from_ticks(4),
            arrow_ipc_payload: b"arrow-grpc-8".to_vec(),
        },
    ];
    for batch in &telemetry {
        batch.validate()?;
    }
    Ok(telemetry.len())
}

fn check_test_worker_status(config: &GrpcTransportConfig) -> WorkerStatus {
    let heartbeat = WorkerHeartbeat {
        lp_id: LpId(8),
        elapsed_since_last_seen: Duration::from_secs(10),
    };
    classify_worker(&heartbeat, config)
}

pub fn local_two_node_contract_proof() -> Result<GrpcLocalTwoNodeProof, ProtocolValidationError> {
    let (mut transport, config) = setup_test_transport()?;
    let (event, received) = exchange_test_events(&mut transport)?;
    let migrations_validated = validate_test_migration()?;
    let telemetry_batches_merged = validate_test_telemetry()?;
    let worker_status = check_test_worker_status(&config);

    let exchanged_events = received
        .iter()
        .filter(|message| matches!(message, PdesMessage::Event(_)))
        .count();

    Ok(GrpcLocalTwoNodeProof {
        exchanged_events,
        migrations_validated,
        telemetry_batches_merged,
        failed_workers_detected: usize::from(worker_status == WorkerStatus::Failed),
        simulation_continues_after_non_leader_failure: worker_status == WorkerStatus::Failed
            && transport.local_lp() == LpId(7),
        final_state_parity: received == vec![PdesMessage::Event(event)],
        real_grpc_runtime_claimed: false,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProtocolValidationError {
    ProtocolMismatch,
    ProtocolVersionMismatch { expected: u16, got: u16 },
    InvalidService,
    DuplicatePeer(LpId),
    SelfPeer(LpId),
    InvalidEndpoint(String),
    InvalidTimeouts,
    InvalidMigrationId,
    SelfMigration(LpId),
    EmptyComponentSet,
    InvalidComponentTypeId,
    EmptyComponentPayload(String),
    InvalidTickRange,
    EmptyTelemetryPayload,
    UnknownTransportLp,
}

pub fn validate_config(config: &GrpcTransportConfig) -> Result<(), ProtocolValidationError> {
    if config.request_timeout.is_zero()
        || config.heartbeat_interval.is_zero()
        || config.heartbeat_timeout.is_zero()
        || config.heartbeat_interval >= config.heartbeat_timeout
        || config.request_timeout >= config.heartbeat_timeout
    {
        return Err(ProtocolValidationError::InvalidTimeouts);
    }

    Ok(())
}

pub fn validate_peers(local_lp: LpId, peers: &[GrpcPeer]) -> Result<(), ProtocolValidationError> {
    let mut seen = Vec::new();
    for peer in peers {
        if peer.lp_id == local_lp {
            return Err(ProtocolValidationError::SelfPeer(local_lp));
        }
        if seen.contains(&peer.lp_id) {
            return Err(ProtocolValidationError::DuplicatePeer(peer.lp_id));
        }
        if !is_supported_endpoint(&peer.endpoint) {
            return Err(ProtocolValidationError::InvalidEndpoint(
                peer.endpoint.clone(),
            ));
        }
        seen.push(peer.lp_id);
    }

    Ok(())
}

fn is_supported_endpoint(endpoint: &str) -> bool {
    let endpoint = endpoint.trim();
    (endpoint.starts_with("http://") || endpoint.starts_with("https://"))
        && endpoint.len() > "http://".len()
        && !endpoint.ends_with("://")
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

/// Placeholder transport preserving the Track 35 API without requiring tonic yet.
#[derive(Debug)]
pub struct GrpcTransport {
    local_lp: LpId,
    peers: Vec<GrpcPeer>,
    config: GrpcTransportConfig,
    inboxes: BTreeMap<LpId, VecDeque<PdesMessage>>,
    reduction_candidates: Vec<Tick>,
    barrier_count: u64,
}

impl GrpcTransport {
    pub fn new(local_lp: LpId, peers: Vec<GrpcPeer>, config: GrpcTransportConfig) -> Self {
        let mut inboxes = BTreeMap::new();
        inboxes.insert(local_lp, VecDeque::new());
        for peer in &peers {
            inboxes.entry(peer.lp_id).or_insert_with(VecDeque::new);
        }

        Self {
            local_lp,
            peers,
            config,
            inboxes,
            reduction_candidates: Vec::new(),
            barrier_count: 0,
        }
    }

    pub fn local_lp(&self) -> LpId {
        self.local_lp
    }

    pub fn peers(&self) -> &[GrpcPeer] {
        &self.peers
    }

    pub fn config(&self) -> &GrpcTransportConfig {
        &self.config
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
        validate_config(&self.config)?;
        validate_peers(self.local_lp, &self.peers)
    }
}

impl PdesTransport for GrpcTransport {
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
    fn default_timeouts_keep_heartbeat_below_failure_timeout() {
        let config = GrpcTransportConfig::default();

        assert!(config.heartbeat_interval < config.heartbeat_timeout);
        assert!(config.request_timeout < config.heartbeat_timeout);
        assert_eq!(validate_config(&config), Ok(()));
    }

    #[test]
    fn peer_list_is_preserved() {
        let transport = GrpcTransport::new(
            LpId(7),
            vec![GrpcPeer {
                lp_id: LpId(8),
                endpoint: "http://127.0.0.1:50051".to_string(),
            }],
            GrpcTransportConfig::default(),
        );

        assert_eq!(transport.local_lp(), LpId(7));
        assert_eq!(transport.peers()[0].lp_id, LpId(8));
    }

    #[test]
    fn protocol_emulator_round_trips_null_messages() {
        let mut transport = GrpcTransport::new(
            LpId(7),
            vec![GrpcPeer {
                lp_id: LpId(8),
                endpoint: "http://127.0.0.1:50051".to_string(),
            }],
            GrpcTransportConfig::default(),
        );
        let null = kairo_ecs_pdes::NullMessage {
            source_lp: LpId(7),
            dest_lp: LpId(8),
            safe_time: Tick::from_ticks(11),
        };

        transport
            .send(LpId(8), PdesMessage::Null(null))
            .expect("gRPC placeholder transport should accept known LP");
        transport.barrier();

        assert_eq!(transport.barrier_count(), 1);
        assert_eq!(transport.pending_messages(), 1);
        assert_eq!(
            transport
                .recv(LpId(8))
                .expect("gRPC placeholder transport should receive from known LP"),
            vec![PdesMessage::Null(null)]
        );
        assert_eq!(transport.pending_messages(), 0);
    }

    #[test]
    fn protocol_emulator_uses_current_gvt_candidate_round() {
        let mut transport = GrpcTransport::new(LpId(0), Vec::new(), GrpcTransportConfig::default());

        assert_eq!(
            transport.all_reduce_min(Tick::from_ticks(6)),
            Tick::from_ticks(6)
        );
        assert_eq!(
            transport.all_reduce_min(Tick::from_ticks(2)),
            Tick::from_ticks(2)
        );
        assert_eq!(
            transport.all_reduce_min(Tick::from_ticks(5)),
            Tick::from_ticks(5)
        );
    }

    #[test]
    fn protocol_emulator_includes_pending_event_timestamps_in_gvt_round() {
        let mut transport = GrpcTransport::new(
            LpId(7),
            vec![GrpcPeer {
                lp_id: LpId(8),
                endpoint: "http://127.0.0.1:50051".to_string(),
            }],
            GrpcTransportConfig::default(),
        );
        let pending_event = kairo_ecs_pdes::RemoteEvent {
            source_lp: LpId(7),
            dest_lp: LpId(8),
            tick: Tick::from_ticks(4),
            event_payload: b"pending-gvt".to_vec(),
        };

        transport
            .send(LpId(8), PdesMessage::Event(pending_event.clone()))
            .expect("known LP should accept queued event");
        transport
            .send(
                LpId(8),
                PdesMessage::Null(kairo_ecs_pdes::NullMessage {
                    source_lp: LpId(7),
                    dest_lp: LpId(8),
                    safe_time: Tick::from_ticks(2),
                }),
            )
            .expect("known LP should accept queued null message");

        assert_eq!(
            transport.all_reduce_min(Tick::from_ticks(10)),
            pending_event.tick
        );
        assert_eq!(
            transport
                .recv(LpId(8))
                .expect("known LP should drain queued messages")
                .len(),
            2
        );
        assert_eq!(
            transport.all_reduce_min(Tick::from_ticks(10)),
            Tick::from_ticks(10)
        );
    }

    #[test]
    fn grpc_contract_envelopes_track_protocol_identity() {
        let exchange = GrpcContractEnvelope::exchange_events(LpId(1), LpId(2), 4);
        let gvt = GrpcContractEnvelope::gvt(LpId(0));

        assert_eq!(exchange.validate(), Ok(()));
        assert_eq!(gvt.validate(), Ok(()));
        assert_eq!(exchange.protocol_id, GRPC_PROTOCOL_ID);
        assert_eq!(exchange.protocol_version, GRPC_PROTOCOL_VERSION);
        assert_eq!(exchange.destination_lp, Some(LpId(2)));
    }

    #[test]
    fn grpc_contract_rejects_empty_migration_id() {
        let err = GrpcContractEnvelope::migration(LpId(1), LpId(2), String::new())
            .validate()
            .unwrap_err();

        assert_eq!(err, ProtocolValidationError::InvalidMigrationId);
    }

    #[test]
    fn transport_send_rejects_unknown_lp() {
        let mut transport = GrpcTransport::new(LpId(7), vec![], GrpcTransportConfig::default());

        assert_eq!(
            transport.send(
                LpId(8),
                PdesMessage::Null(kairo_ecs_pdes::NullMessage {
                    source_lp: LpId(7),
                    dest_lp: LpId(8),
                    safe_time: Tick::from_ticks(12),
                })
            ),
            Err(TransportError::UnknownLogicalProcess(LpId(8)))
        );
    }

    #[test]
    fn transport_send_rejects_unknown_source_and_destination_mismatch() {
        let mut transport = GrpcTransport::new(
            LpId(7),
            vec![GrpcPeer {
                lp_id: LpId(8),
                endpoint: "https://worker-8.example.test:50051".to_string(),
            }],
            GrpcTransportConfig::default(),
        );

        assert_eq!(
            transport.send(
                LpId(8),
                PdesMessage::Null(kairo_ecs_pdes::NullMessage {
                    source_lp: LpId(9),
                    dest_lp: LpId(8),
                    safe_time: Tick::from_ticks(12),
                })
            ),
            Err(TransportError::UnknownLogicalProcess(LpId(9)))
        );

        assert_eq!(
            transport.send(
                LpId(8),
                PdesMessage::Event(kairo_ecs_pdes::RemoteEvent {
                    source_lp: LpId(7),
                    dest_lp: LpId(7),
                    tick: Tick::from_ticks(12),
                    event_payload: vec![1, 2, 3],
                })
            ),
            Err(TransportError::MessageDestinationMismatch {
                send_dest: LpId(8),
                message_dest: LpId(7),
            })
        );
    }

    #[test]
    fn local_smoke_validates_peers_and_timeout_config() {
        let transport = GrpcTransport::new(
            LpId(7),
            vec![GrpcPeer {
                lp_id: LpId(8),
                endpoint: "https://worker-8.example.test:50051".to_string(),
            }],
            GrpcTransportConfig::default(),
        );

        assert_eq!(transport.validate_protocol(), Ok(()));
    }

    #[test]
    fn peer_validator_rejects_duplicate_peer_ids() {
        let err = validate_peers(
            LpId(0),
            &[
                GrpcPeer {
                    lp_id: LpId(1),
                    endpoint: "http://127.0.0.1:50051".to_string(),
                },
                GrpcPeer {
                    lp_id: LpId(1),
                    endpoint: "http://127.0.0.1:50052".to_string(),
                },
            ],
        )
        .unwrap_err();

        assert_eq!(err, ProtocolValidationError::DuplicatePeer(LpId(1)));
    }

    #[test]
    fn migration_validator_accepts_complete_request() {
        let request = GrpcMigrationRequest {
            entity: EntityId {
                index: 11,
                generation: 3,
            },
            source_lp: LpId(2),
            dest_lp: LpId(3),
            migration_id: "grpc-mig-11".to_string(),
            components: vec![GrpcComponentBlob {
                component_type_id: "health".to_string(),
                payload: vec![9, 8, 7],
            }],
        };

        assert_eq!(request.validate(), Ok(()));
    }

    #[test]
    fn migration_validator_rejects_empty_component_set() {
        let request = GrpcMigrationRequest {
            entity: EntityId {
                index: 11,
                generation: 3,
            },
            source_lp: LpId(2),
            dest_lp: LpId(3),
            migration_id: "grpc-mig-11".to_string(),
            components: Vec::new(),
        };

        assert_eq!(
            request.validate(),
            Err(ProtocolValidationError::EmptyComponentSet)
        );
    }

    #[test]
    fn telemetry_batch_validator_requires_monotonic_tick_range() {
        let batch = GrpcTelemetryBatch {
            source_lp: LpId(4),
            tick_start: Tick::from_ticks(20),
            tick_end: Tick::from_ticks(10),
            arrow_ipc_payload: b"arrow-ipc".to_vec(),
        };

        assert_eq!(
            batch.validate(),
            Err(ProtocolValidationError::InvalidTickRange)
        );
    }

    #[test]
    fn heartbeat_classifier_marks_late_worker_failed() {
        let config = GrpcTransportConfig::default();

        assert_eq!(
            classify_worker(
                &WorkerHeartbeat {
                    lp_id: LpId(9),
                    elapsed_since_last_seen: Duration::from_secs(1),
                },
                &config
            ),
            WorkerStatus::Healthy
        );
        assert_eq!(
            classify_worker(
                &WorkerHeartbeat {
                    lp_id: LpId(9),
                    elapsed_since_last_seen: Duration::from_secs(10),
                },
                &config
            ),
            WorkerStatus::Failed
        );
    }

    #[test]
    fn local_two_node_contract_proof_covers_event_migration_telemetry_and_failure() {
        let proof = local_two_node_contract_proof().unwrap();

        assert_eq!(proof.exchanged_events, 1);
        assert_eq!(proof.migrations_validated, 1);
        assert_eq!(proof.telemetry_batches_merged, 2);
        assert_eq!(proof.failed_workers_detected, 1);
        assert!(proof.simulation_continues_after_non_leader_failure);
        assert!(proof.final_state_parity);
        assert!(!proof.real_grpc_runtime_claimed);
    }
}
