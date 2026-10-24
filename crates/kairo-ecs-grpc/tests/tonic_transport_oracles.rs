#![forbid(unsafe_code)]

use std::net::{SocketAddr, TcpListener};
use std::thread;
use std::time::{Duration, Instant};

use kairo_ecs_grpc::service::simulation_transport_client::SimulationTransportClient;
use kairo_ecs_grpc::{
    GrpcComponentBlob, GrpcMigrationRequest, GrpcPeer, GrpcTransportConfig, TonicTransport,
    WireGvtProposal, WireHeartbeat, WireNullMessage, WireOptimisticEnvelope, WireRemoteEvent,
    WorkerHeartbeat, ENVELOPE_PROTOCOL_VERSION_V2, ENVELOPE_SCHEMA_VERSION_PROTOBUF_V2,
    GRPC_PROTOCOL_ID,
};
use kairo_ecs_pdes::{
    LogicalEventId, LpId, NullMessage, OptimisticAuthority, OptimisticMessage,
    OptimisticMessageKind, PdesMessage, PdesTransport, RemoteEvent,
};
use kairo_ecs_types::{EntityId, SimTime as Tick};
use tonic::{Code, Request};

fn get_available_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

#[test]
fn oracle_tonic_transport_clean_lifecycle_and_port_rebinding() {
    let port = get_available_port();
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let config = GrpcTransportConfig::default();

    let transport = TonicTransport::bind(LpId(1), addr, vec![], config).expect("bind transport");
    assert_eq!(transport.bound_addr().port(), port);

    // Drop transport and verify immediate clean shutdown without hanging
    let start = Instant::now();
    drop(transport);
    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_millis(1000),
        "tonic shutdown took too long: {elapsed:?}"
    );

    // Verify port is immediately re-bindable
    let rebind = TcpListener::bind(addr);
    assert!(
        rebind.is_ok(),
        "port should be immediately reusable after tonic transport drop"
    );
}

#[test]
fn oracle_tonic_transport_independent_grpc_peer_rpc_interoperability() {
    let port = get_available_port();
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let config = GrpcTransportConfig::default();

    let mut transport =
        TonicTransport::bind(LpId(1), addr, vec![], config).expect("bind transport");

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .expect("build client runtime");

    rt.block_on(async {
        let endpoint_str = format!("http://127.0.0.1:{port}");
        let mut client = SimulationTransportClient::connect(endpoint_str)
            .await
            .expect("independent peer connect");

        // 1. Foreign peer sends Heartbeat RPC
        let hb_req = Request::new(WireHeartbeat {
            source_lp: 99,
            observed_tick: 42,
            elapsed_since_last_seen_ms: 15,
        });
        let hb_resp = client
            .heartbeat(hb_req)
            .await
            .expect("heartbeat response")
            .into_inner();
        assert!(hb_resp.accepted);
        assert_eq!(hb_resp.source_lp, 99);

        // 2. Foreign peer sends GvtProposal RPC
        let gvt_req = Request::new(WireGvtProposal {
            source_lp: 99,
            local_min_tick: 100u128.to_be_bytes().to_vec(),
            round: 0,
        });
        let gvt_resp = client
            .gvt_sync(gvt_req)
            .await
            .expect("gvt_sync response")
            .into_inner();
        assert_eq!(gvt_resp.gvt_tick, 100u128.to_be_bytes().to_vec());
        assert_eq!(gvt_resp.round, 0);

        // 3. Foreign peer sends standard Event RPC with full 128-bit timestamp
        let event_req = Request::new(WireRemoteEvent {
            source_lp: 99,
            dest_lp: 1,
            tick: 120u128.to_be_bytes().to_vec(),
            event_payload: b"independent_grpc_payload".to_vec(),
        });
        let event_resp = client
            .exchange_events(event_req)
            .await
            .expect("exchange_events response")
            .into_inner();
        assert_eq!(event_resp.tick, 120u128.to_be_bytes().to_vec());

        // 4. Foreign peer sends NullMessage RPC
        let null_req = Request::new(WireNullMessage {
            source_lp: 99,
            dest_lp: 1,
            safe_time: 150u128.to_be_bytes().to_vec(),
        });
        let null_resp = client
            .send_null_message(null_req)
            .await
            .expect("send_null_message response")
            .into_inner();
        assert!(null_resp.accepted);
    });

    // Server should have recorded the received heartbeat and events in PDES queue
    let heartbeats = transport.drain_heartbeats();
    assert_eq!(heartbeats.len(), 1);
    assert_eq!(heartbeats[0].lp_id, LpId(99));

    let messages = transport.recv(LpId(1)).expect("recv messages");
    assert_eq!(messages.len(), 2);
    match &messages[0] {
        PdesMessage::Event(e) => {
            assert_eq!(e.source_lp, LpId(99));
            assert_eq!(e.dest_lp, LpId(1));
            assert_eq!(e.tick, Tick::from_ticks(120));
            assert_eq!(e.event_payload, b"independent_grpc_payload");
        }
        _ => panic!("expected event message"),
    }
    match &messages[1] {
        PdesMessage::Null(n) => {
            assert_eq!(n.source_lp, LpId(99));
            assert_eq!(n.dest_lp, LpId(1));
            assert_eq!(n.safe_time, Tick::from_ticks(150));
        }
        _ => panic!("expected null message"),
    }
}

#[test]
fn oracle_tonic_transport_optimistic_envelope_full_roundtrip_and_pdes_ordering() {
    let port1 = get_available_port();
    let port2 = get_available_port();
    let addr1: SocketAddr = format!("127.0.0.1:{port1}").parse().unwrap();
    let addr2: SocketAddr = format!("127.0.0.1:{port2}").parse().unwrap();

    let peer_for_1 = vec![GrpcPeer {
        lp_id: LpId(2),
        endpoint: format!("http://127.0.0.1:{port2}"),
    }];
    let peer_for_2 = vec![GrpcPeer {
        lp_id: LpId(1),
        endpoint: format!("http://127.0.0.1:{port1}"),
    }];

    let transport1 =
        TonicTransport::bind(LpId(1), addr1, peer_for_1, GrpcTransportConfig::default())
            .expect("bind transport 1");
    let mut transport2 =
        TonicTransport::bind(LpId(2), addr2, peer_for_2, GrpcTransportConfig::default())
            .expect("bind transport 2");

    let authority = OptimisticAuthority::Scoped {
        simulation_namespace: 0x9999,
        ownership_epoch: 1,
    };

    // LP 1 sends a sequence of 5 ordered optimistic events to LP 2
    for seq in 1..=5 {
        let root_id = LogicalEventId::root(LpId(1), seq);
        let event = RemoteEvent {
            source_lp: LpId(1),
            dest_lp: LpId(2),
            tick: Tick::from_ticks(100 + seq as u128),
            event_payload: format!("payload_{seq}").into_bytes(),
        };

        let msg = OptimisticMessage::try_from_authority_parts(
            event,
            root_id,
            authority,
            seq,
            OptimisticMessageKind::Positive,
        )
        .expect("construct native optimistic message");

        let ack = transport1
            .send_optimistic(LpId(2), &msg)
            .expect("send optimistic envelope");
        assert!(ack.accepted);
    }

    // LP 2 polls optimistic inbox and verifies strict FIFO / tick monotonicity
    let received = transport2
        .poll_optimistic(LpId(2))
        .expect("poll optimistic messages");
    assert_eq!(received.len(), 5);

    for (idx, msg) in received.iter().enumerate() {
        let expected_seq = (idx + 1) as u64;
        assert_eq!(
            msg.event().tick,
            Tick::from_ticks(100 + expected_seq as u128)
        );
        assert_eq!(
            msg.event().event_payload,
            format!("payload_{expected_seq}").into_bytes()
        );
    }
}

#[test]
fn oracle_tonic_transport_wire_protocol_and_version_rejection() {
    let port = get_available_port();
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let config = GrpcTransportConfig::default();

    let transport = TonicTransport::bind(LpId(1), addr, vec![], config).expect("bind transport");
    assert_eq!(transport.decode_failure_count(), 0);

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .expect("client runtime");

    rt.block_on(async {
        let endpoint_str = format!("http://127.0.0.1:{port}");
        let mut client = None;
        for _ in 0..20 {
            match SimulationTransportClient::connect(endpoint_str.clone()).await {
                Ok(c) => {
                    client = Some(c);
                    break;
                }
                Err(_) => tokio::time::sleep(std::time::Duration::from_millis(25)).await,
            }
        }
        let mut client = client.expect("connect client after retries");

        // 1. Rejected: mismatched protocol_id
        let bad_proto = WireOptimisticEnvelope {
            envelope_schema_version: ENVELOPE_SCHEMA_VERSION_PROTOBUF_V2,
            protocol_id: "incompatible.protocol.id".into(),
            protocol_version: ENVELOPE_PROTOCOL_VERSION_V2 as u32,
            authority_mode: 0,
            simulation_namespace: vec![0x11; 8],
            ownership_epoch: 1,
            emission_epoch: 1,
            source_lp: 2,
            dest_lp: 1,
            tick: 100u128.to_be_bytes().to_vec(),
            ancestry_frames: Vec::new(),
            incarnation: 0,
            kind: 0,
            event_payload: b"test".to_vec(),
        };

        let err1 = client
            .send_optimistic_envelope(Request::new(bad_proto))
            .await
            .expect_err("mismatched protocol_id must be rejected");
        assert_eq!(err1.code(), Code::InvalidArgument);
        assert!(err1.message().contains("protocol ID mismatch"));

        // 2. Rejected: unsupported schema version
        let bad_schema = WireOptimisticEnvelope {
            envelope_schema_version: 999,
            protocol_id: GRPC_PROTOCOL_ID.into(),
            protocol_version: ENVELOPE_PROTOCOL_VERSION_V2 as u32,
            authority_mode: 0,
            simulation_namespace: vec![0x11; 8],
            ownership_epoch: 1,
            emission_epoch: 1,
            source_lp: 2,
            dest_lp: 1,
            tick: 100u128.to_be_bytes().to_vec(),
            ancestry_frames: Vec::new(),
            incarnation: 0,
            kind: 0,
            event_payload: b"test".to_vec(),
        };

        let err2 = client
            .send_optimistic_envelope(Request::new(bad_schema))
            .await
            .expect_err("unsupported schema version must be rejected");
        assert_eq!(err2.code(), Code::InvalidArgument);
        assert!(err2
            .message()
            .contains("unsupported envelope schema version"));
    });

    // Verify rejection failures were recorded in server statistics
    assert_eq!(transport.decode_failure_count(), 2);
}

#[test]
fn oracle_tonic_transport_transient_failure_recovery_and_deadline_policy() {
    let port = get_available_port();
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let config = GrpcTransportConfig {
        request_timeout: Duration::from_millis(200),
        heartbeat_interval: Duration::from_millis(50),
        heartbeat_timeout: Duration::from_millis(500),
    };

    let peer = vec![GrpcPeer {
        lp_id: LpId(2),
        endpoint: format!("http://127.0.0.1:{port}"),
    }];

    // 1. Client attempts to connect to peer before peer server is up
    let transport = TonicTransport::bind(
        LpId(1),
        format!("127.0.0.1:{}", get_available_port())
            .parse()
            .unwrap(),
        peer,
        config.clone(),
    )
    .expect("bind client transport");

    let hb = WorkerHeartbeat {
        lp_id: LpId(1),
        elapsed_since_last_seen: Duration::from_millis(10),
    };

    let err = transport.send_heartbeat(LpId(2), &hb);
    assert!(
        err.is_err(),
        "must fail gracefully when peer server is not up"
    );

    // 2. Bring up peer server on port
    let peer_transport =
        TonicTransport::bind(LpId(2), addr, vec![], config).expect("bind peer transport");

    // Retry sending heartbeat: client should successfully connect and deliver
    let mut success = false;
    for _ in 0..15 {
        if transport.send_heartbeat(LpId(2), &hb).is_ok() {
            success = true;
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    assert!(
        success,
        "must recover and deliver heartbeat once peer server becomes available"
    );

    let received = peer_transport.drain_heartbeats();
    assert_eq!(received.len(), 1);
    assert_eq!(received[0].lp_id, LpId(1));
}

#[test]
fn oracle_tonic_transport_migration_and_gvt_consensus_round() {
    let port1 = get_available_port();
    let port2 = get_available_port();
    let addr1: SocketAddr = format!("127.0.0.1:{port1}").parse().unwrap();
    let addr2: SocketAddr = format!("127.0.0.1:{port2}").parse().unwrap();

    let peer1 = vec![GrpcPeer {
        lp_id: LpId(2),
        endpoint: format!("http://127.0.0.1:{port2}"),
    }];
    let peer2 = vec![GrpcPeer {
        lp_id: LpId(1),
        endpoint: format!("http://127.0.0.1:{port1}"),
    }];

    let mut transport1 =
        TonicTransport::bind(LpId(1), addr1, peer1, GrpcTransportConfig::default())
            .expect("bind transport 1");
    let mut transport2 =
        TonicTransport::bind(LpId(2), addr2, peer2, GrpcTransportConfig::default())
            .expect("bind transport 2");

    // 1. Entity Migration over gRPC
    let migration_req = GrpcMigrationRequest {
        entity: EntityId::new(42, 1),
        source_lp: LpId(1),
        dest_lp: LpId(2),
        migration_id: "mig_42_epoch_1".to_string(),
        components: vec![GrpcComponentBlob {
            component_type_id: "Position".to_string(),
            payload: vec![1, 2, 3, 4],
        }],
    };

    let ack = transport1
        .send_migration(LpId(2), &migration_req)
        .expect("send migration");
    assert!(ack.applied);

    let received_migrations = transport2.drain_migrations();
    assert_eq!(received_migrations.len(), 1);
    assert_eq!(received_migrations[0].entity.index, 42);
    assert_eq!(received_migrations[0].entity.generation, 1);
    assert_eq!(received_migrations[0].components.len(), 1);
    assert_eq!(
        received_migrations[0].components[0].component_type_id,
        "Position"
    );

    // 2. Null message delivery over gRPC (Finding 2)
    transport1
        .send(
            LpId(2),
            PdesMessage::Null(NullMessage {
                source_lp: LpId(1),
                dest_lp: LpId(2),
                safe_time: Tick::from_ticks(250),
            }),
        )
        .expect("send remote null message");
    let recvd_null = transport2.recv(LpId(2)).expect("recv null messages");
    assert_eq!(recvd_null.len(), 1);
    match &recvd_null[0] {
        PdesMessage::Null(n) => {
            assert_eq!(n.source_lp, LpId(1));
            assert_eq!(n.dest_lp, LpId(2));
            assert_eq!(n.safe_time, Tick::from_ticks(250));
        }
        _ => panic!("expected null message"),
    }

    // 3. Large u128 timestamp preservation (> u64::MAX) (Finding 4)
    let large_tick = Tick::from_ticks(u128::MAX - 100);
    transport1
        .send(
            LpId(2),
            PdesMessage::Event(RemoteEvent {
                source_lp: LpId(1),
                dest_lp: LpId(2),
                tick: large_tick,
                event_payload: b"large_tick_payload".to_vec(),
            }),
        )
        .expect("send large tick event");
    let recvd_event = transport2.recv(LpId(2)).expect("recv event");
    assert_eq!(recvd_event.len(), 1);
    match &recvd_event[0] {
        PdesMessage::Event(e) => {
            assert_eq!(e.tick, large_tick);
            assert_eq!(e.event_payload, b"large_tick_payload");
        }
        _ => panic!("expected event message"),
    }

    // 4. Multiple consecutive Mattern GVT Consensus Rounds over gRPC (Finding 3)
    // Round 0: LP 1 has timestamp 500, LP 2 has timestamp 300 -> min(500, 300) = 300
    let t1_round0 = thread::spawn(move || {
        let r0 = transport1.all_reduce_min(Tick::from_ticks(500));
        let r1 = transport1.all_reduce_min(Tick::from_ticks(600));
        (r0, r1)
    });

    let t2_r0 = transport2.all_reduce_min(Tick::from_ticks(300));
    let t2_r1 = transport2.all_reduce_min(Tick::from_ticks(400));
    let (t1_r0, t1_r1) = t1_round0.join().expect("join t1");

    assert_eq!(t1_r0, Tick::from_ticks(300));
    assert_eq!(t2_r0, Tick::from_ticks(300));

    // Round 1 must use round 1 proposals rather than being confused by round 0
    assert_eq!(t1_r1, Tick::from_ticks(400));
    assert_eq!(t2_r1, Tick::from_ticks(400));
}
