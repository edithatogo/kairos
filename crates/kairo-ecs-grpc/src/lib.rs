#![forbid(unsafe_code)]

use std::collections::{BTreeMap, VecDeque};
use std::time::Duration;

use kairo_ecs_pdes::{LpId, PdesMessage, PdesTransport, Tick};
use kairo_ecs_types::EntityId;

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
pub enum ProtocolValidationError {
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

    pub fn validate_protocol(&self) -> Result<(), ProtocolValidationError> {
        validate_config(&self.config)?;
        validate_peers(self.local_lp, &self.peers)
    }
}

impl PdesTransport for GrpcTransport {
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

        transport.send(LpId(8), PdesMessage::Null(null));
        transport.barrier();

        assert_eq!(transport.barrier_count(), 1);
        assert_eq!(transport.pending_messages(), 1);
        assert_eq!(transport.recv(LpId(8)), vec![PdesMessage::Null(null)]);
        assert_eq!(transport.pending_messages(), 0);
    }

    #[test]
    fn protocol_emulator_reduces_minimum_gvt_candidate() {
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
            Tick::from_ticks(2)
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
}
