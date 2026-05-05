#![forbid(unsafe_code)]

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
}

impl GrpcTransport {
    pub fn new(local_lp: LpId, peers: Vec<GrpcPeer>, config: GrpcTransportConfig) -> Self {
        Self {
            local_lp,
            peers,
            config,
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
}

impl PdesTransport for GrpcTransport {
    fn send(&mut self, _dest: LpId, _message: PdesMessage) {
        panic!("GrpcTransport::send requires the future tonic backend");
    }

    fn recv(&mut self, _lp_id: LpId) -> Vec<PdesMessage> {
        panic!("GrpcTransport::recv requires the future tonic backend");
    }

    fn barrier(&mut self) {
        panic!("GrpcTransport::barrier requires the future coordinator backend");
    }

    fn all_reduce_min(&mut self, _timestamp: Tick) -> Tick {
        panic!("GrpcTransport::all_reduce_min requires the future coordinator backend");
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
}
