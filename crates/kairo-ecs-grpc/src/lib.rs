#![forbid(unsafe_code)]

use std::collections::{BTreeMap, VecDeque};
use std::time::Duration;

use kairo_ecs_pdes::{LpId, PdesMessage, PdesTransport, Tick};

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
}
