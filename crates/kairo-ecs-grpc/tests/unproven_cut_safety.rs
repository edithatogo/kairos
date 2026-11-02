// SPDX-License-Identifier: Apache-2.0
#![forbid(unsafe_code)]

use std::net::TcpListener;
use std::thread;

use kairo_ecs_grpc::{GrpcPeer, GrpcSocketTransport, GrpcTransportConfig, TonicTransport};
use kairo_ecs_pdes::{LpId, PdesTransport, Tick};

fn port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

#[test]
fn responsive_tonic_peers_do_not_certify_a_distributed_cut() {
    let (a, b) = (port(), port());
    let mut first = TonicTransport::bind(
        LpId(0),
        format!("127.0.0.1:{a}").parse().unwrap(),
        vec![GrpcPeer {
            lp_id: LpId(1),
            endpoint: format!("http://127.0.0.1:{b}"),
        }],
        GrpcTransportConfig::default(),
    )
    .unwrap();
    let mut second = TonicTransport::bind(
        LpId(1),
        format!("127.0.0.1:{b}").parse().unwrap(),
        vec![GrpcPeer {
            lp_id: LpId(0),
            endpoint: format!("http://127.0.0.1:{a}"),
        }],
        GrpcTransportConfig::default(),
    )
    .unwrap();
    let peer = thread::spawn(move || {
        let diagnostic = second.reduce_reported_minimum(Tick::from_ticks(300));
        (diagnostic, second.all_reduce_min(Tick::from_ticks(300)))
    });
    assert_eq!(
        first.reduce_reported_minimum(Tick::from_ticks(500)),
        Tick::from_ticks(300),
        "same participants must first complete a positive diagnostic round"
    );
    assert_eq!(
        first.all_reduce_min(Tick::from_ticks(500)),
        Tick::ZERO,
        "responsive proposals do not prove in-flight/retirement accounting"
    );
    assert_eq!(peer.join().unwrap(), (Tick::from_ticks(300), Tick::ZERO));
}

#[test]
fn responsive_custom_tcp_peers_do_not_certify_a_distributed_cut() {
    let (a, b) = (port(), port());
    let mut first = GrpcSocketTransport::bind(
        LpId(0),
        format!("127.0.0.1:{a}").parse().unwrap(),
        vec![GrpcPeer {
            lp_id: LpId(1),
            endpoint: format!("http://127.0.0.1:{b}"),
        }],
        GrpcTransportConfig::default(),
    )
    .unwrap();
    let mut second = GrpcSocketTransport::bind(
        LpId(1),
        format!("127.0.0.1:{b}").parse().unwrap(),
        vec![GrpcPeer {
            lp_id: LpId(0),
            endpoint: format!("http://127.0.0.1:{a}"),
        }],
        GrpcTransportConfig::default(),
    )
    .unwrap();
    let peer = thread::spawn(move || {
        let diagnostic = second.reduce_reported_minimum(Tick::from_ticks(300));
        (diagnostic, second.all_reduce_min(Tick::from_ticks(300)))
    });
    assert_eq!(
        first.reduce_reported_minimum(Tick::from_ticks(500)),
        Tick::from_ticks(300),
        "same participants must first complete a positive diagnostic round"
    );
    assert_eq!(first.all_reduce_min(Tick::from_ticks(500)), Tick::ZERO);
    assert_eq!(peer.join().unwrap(), (Tick::from_ticks(300), Tick::ZERO));
}

#[test]
fn unconfigured_network_listeners_are_not_closed_local_transports() {
    let mut tonic = TonicTransport::bind(
        LpId(0),
        "127.0.0.1:0".parse().unwrap(),
        vec![],
        GrpcTransportConfig::default(),
    )
    .unwrap();
    let mut socket = GrpcSocketTransport::bind(
        LpId(0),
        "127.0.0.1:0".parse().unwrap(),
        vec![],
        GrpcTransportConfig::default(),
    )
    .unwrap();
    assert_eq!(tonic.all_reduce_min(Tick::from_ticks(500)), Tick::ZERO);
    assert_eq!(socket.all_reduce_min(Tick::from_ticks(500)), Tick::ZERO);
    assert_eq!(
        tonic.reduce_reported_minimum(Tick::from_ticks(500)),
        Tick::from_ticks(500)
    );
    assert_eq!(
        socket.reduce_reported_minimum(Tick::from_ticks(500)),
        Tick::from_ticks(500)
    );
}
