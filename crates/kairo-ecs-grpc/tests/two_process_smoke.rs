#![forbid(unsafe_code)]

use std::env;
use std::net::{SocketAddr, TcpListener};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use kairo_ecs_grpc::{
    classify_worker, GrpcComponentBlob, GrpcMigrationRequest, GrpcPeer, GrpcSocketTransport,
    GrpcTelemetryBatch, GrpcTransportConfig, WorkerHeartbeat, WorkerStatus,
};
use kairo_ecs_pdes::{LpId, PdesMessage, PdesTransport, RemoteEvent, Tick};
use kairo_ecs_types::EntityId;

#[test]
fn two_process_smoke() {
    // If running as the spawned child worker process:
    if env::var("KAIRO_CHILD_WORKER").is_ok() {
        let parent_port: u16 = env::var("KAIRO_PARENT_PORT")
            .expect("parent port env")
            .parse()
            .expect("valid parent port");
        let child_port: u16 = env::var("KAIRO_CHILD_PORT")
            .expect("child port env")
            .parse()
            .expect("valid child port");
        run_child_worker(parent_port, child_port);
        return;
    }

    // Running as primary test runner (Parent Process / Node 1)
    let parent_listener = TcpListener::bind("127.0.0.1:0").expect("bind parent listener");
    let parent_port = parent_listener.local_addr().unwrap().port();
    drop(parent_listener);

    let child_listener = TcpListener::bind("127.0.0.1:0").expect("bind child listener");
    let child_port = child_listener.local_addr().unwrap().port();
    drop(child_listener);

    // Spawn child process with environment variables
    let current_exe = env::current_exe().expect("current exe");
    let mut child = Command::new(&current_exe)
        .arg("--exact")
        .arg("two_process_smoke")
        .arg("--nocapture")
        .env("KAIRO_CHILD_WORKER", "1")
        .env("KAIRO_PARENT_PORT", parent_port.to_string())
        .env("KAIRO_CHILD_PORT", child_port.to_string())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("spawn child worker process");

    // Parent Node 1 setup (LP 7)
    let config = GrpcTransportConfig::default();
    let parent_addr: SocketAddr = format!("127.0.0.1:{parent_port}").parse().unwrap();
    let child_addr = format!("http://127.0.0.1:{child_port}");

    let mut parent_transport = GrpcSocketTransport::bind(
        LpId(7),
        parent_addr,
        vec![GrpcPeer {
            lp_id: LpId(8),
            endpoint: child_addr,
        }],
        config.clone(),
    )
    .expect("bind parent socket transport");

    assert!(parent_transport.real_grpc_runtime_claimed());

    // 1. Send application event to child (LP 8) over real TCP socket with retry while child starts up
    let event = RemoteEvent {
        source_lp: LpId(7),
        dest_lp: LpId(8),
        tick: Tick::from_ticks(10),
        event_payload: b"grpc-two-process-payload".to_vec(),
    };

    let mut sent = false;
    for _ in 0..40 {
        if parent_transport
            .send(LpId(8), PdesMessage::Event(event.clone()))
            .is_ok()
        {
            sent = true;
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    assert!(
        sent,
        "parent should successfully send event to child worker over real TCP socket"
    );

    // 2. Send entity migration request preserving component generations
    let migration = GrpcMigrationRequest {
        entity: EntityId {
            index: 42,
            generation: 3,
        },
        source_lp: LpId(7),
        dest_lp: LpId(8),
        migration_id: "mig-7-to-8-42".to_string(),
        components: vec![GrpcComponentBlob {
            component_type_id: "velocity".to_string(),
            payload: vec![1, 2, 3, 4],
        }],
    };
    parent_transport
        .send_migration(LpId(8), &migration)
        .expect("parent send migration over socket");

    // 3. Send telemetry batch
    let telemetry = GrpcTelemetryBatch {
        source_lp: LpId(7),
        tick_start: Tick::from_ticks(0),
        tick_end: Tick::from_ticks(10),
        arrow_ipc_payload: b"arrow-telemetry-data".to_vec(),
    };
    parent_transport
        .send_telemetry(LpId(8), &telemetry)
        .expect("parent send telemetry over socket");

    // 4. Test failure classification
    let healthy_heartbeat = WorkerHeartbeat {
        lp_id: LpId(8),
        elapsed_since_last_seen: Duration::from_millis(500),
    };
    assert_eq!(
        classify_worker(&healthy_heartbeat, &config),
        WorkerStatus::Healthy
    );

    let suspect_heartbeat = WorkerHeartbeat {
        lp_id: LpId(8),
        elapsed_since_last_seen: Duration::from_secs(3),
    };
    assert_eq!(
        classify_worker(&suspect_heartbeat, &config),
        WorkerStatus::Suspect
    );

    let failed_heartbeat = WorkerHeartbeat {
        lp_id: LpId(8),
        elapsed_since_last_seen: Duration::from_secs(12),
    };
    assert_eq!(
        classify_worker(&failed_heartbeat, &config),
        WorkerStatus::Failed
    );

    // Wait for response from child worker
    let mut received_events = Vec::new();
    for _ in 0..50 {
        let msgs = parent_transport.recv(LpId(7)).unwrap();
        received_events.extend(msgs);
        if !received_events.is_empty() {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }

    assert_eq!(
        received_events.len(),
        1,
        "parent should receive reply event from child worker"
    );
    if let PdesMessage::Event(reply) = &received_events[0] {
        assert_eq!(reply.source_lp, LpId(8));
        assert_eq!(reply.dest_lp, LpId(7));
        assert_eq!(reply.event_payload, b"reply-from-worker-lp8");
    } else {
        panic!("expected event reply from worker");
    }

    // Wait for child process to exit cleanly
    let status = child.wait().expect("wait on child worker process");
    assert!(
        status.success(),
        "child process exited with non-zero status: {status:?}"
    );
}

fn run_child_worker(parent_port: u16, child_port: u16) {
    let config = GrpcTransportConfig::default();
    let child_addr: SocketAddr = format!("127.0.0.1:{child_port}").parse().unwrap();
    let parent_endpoint = format!("http://127.0.0.1:{parent_port}");

    let mut worker_transport = GrpcSocketTransport::bind(
        LpId(8),
        child_addr,
        vec![GrpcPeer {
            lp_id: LpId(7),
            endpoint: parent_endpoint,
        }],
        config,
    )
    .expect("bind child socket transport");

    // Wait for event from parent
    let mut received = Vec::new();
    for _ in 0..50 {
        let msgs = worker_transport.recv(LpId(8)).unwrap();
        received.extend(msgs);
        if !received.is_empty() {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }

    assert_eq!(
        received.len(),
        1,
        "worker should receive 1 event from parent"
    );
    if let PdesMessage::Event(ev) = &received[0] {
        assert_eq!(ev.source_lp, LpId(7));
        assert_eq!(ev.dest_lp, LpId(8));
        assert_eq!(ev.event_payload, b"grpc-two-process-payload");
    } else {
        panic!("worker expected PdesMessage::Event");
    }

    // Send reply event back to parent
    let reply = RemoteEvent {
        source_lp: LpId(8),
        dest_lp: LpId(7),
        tick: Tick::from_ticks(15),
        event_payload: b"reply-from-worker-lp8".to_vec(),
    };
    worker_transport
        .send(LpId(7), PdesMessage::Event(reply))
        .expect("worker send reply to parent");

    thread::sleep(Duration::from_millis(100));
}

#[test]
fn socket_transport_preserves_optimistic_migration_telemetry_heartbeat_and_rejects_https() {
    use kairo_ecs_grpc::{
        AncestryFrame, GrpcComponentBlob, GrpcMigrationRequest, GrpcTelemetryBatch,
        KairoOptimisticEnvelopeV1, OptimisticAuthorityMode, WireMessageKind, GRPC_PROTOCOL_ID,
    };
    use kairo_ecs_pdes::OptimisticMessageKind;
    use kairo_ecs_types::EntityId;

    let config = GrpcTransportConfig::default();

    // 1. Reject https:// endpoints explicitly
    let https_err = GrpcSocketTransport::bind(
        LpId(1),
        "127.0.0.1:0".parse().unwrap(),
        vec![GrpcPeer {
            lp_id: LpId(2),
            endpoint: "https://127.0.0.1:50051".to_string(),
        }],
        config.clone(),
    );
    assert!(
        https_err.is_err(),
        "https:// endpoint must be rejected on plaintext socket"
    );

    // 2. Bind two local endpoints and verify optimistic, migration, telemetry, heartbeat, and round GVT
    let mut node_b = GrpcSocketTransport::bind(
        LpId(2),
        "127.0.0.1:0".parse().unwrap(),
        vec![],
        config.clone(),
    )
    .unwrap();
    let port_b = node_b.bound_addr().port();

    let node_a = GrpcSocketTransport::bind(
        LpId(1),
        "127.0.0.1:0".parse().unwrap(),
        vec![GrpcPeer {
            lp_id: LpId(2),
            endpoint: format!("http://127.0.0.1:{port_b}"),
        }],
        config,
    )
    .unwrap();

    // Send anti-message envelope with non-zero incarnation
    let env = KairoOptimisticEnvelopeV1 {
        envelope_schema_version: 1,
        protocol_id: GRPC_PROTOCOL_ID.to_string(),
        protocol_version: 1,
        authority_mode: OptimisticAuthorityMode::Scoped,
        simulation_namespace: 0x42,
        ownership_epoch: 3,
        emission_epoch: 3,
        source_lp: 1,
        dest_lp: 2,
        tick: 77,
        ancestry_frames: vec![AncestryFrame::Root {
            root_source_lp: 1,
            root_sequence: 9,
        }],
        incarnation: 4,
        kind: WireMessageKind::Anti,
        event_payload: b"anti-cancel".to_vec(),
    };
    node_a.send_optimistic_envelope(LpId(2), &env).unwrap();

    let mig = GrpcMigrationRequest {
        entity: EntityId {
            index: 12,
            generation: 5,
        },
        source_lp: LpId(1),
        dest_lp: LpId(2),
        migration_id: "mig-12".to_string(),
        components: vec![GrpcComponentBlob {
            component_type_id: "state".to_string(),
            payload: vec![1, 2, 3, 4],
        }],
    };
    node_a.send_migration(LpId(2), &mig).unwrap();

    let telem = GrpcTelemetryBatch {
        source_lp: LpId(1),
        tick_start: Tick::from_ticks(10),
        tick_end: Tick::from_ticks(20),
        arrow_ipc_payload: b"ipc-batch".to_vec(),
    };
    node_a.send_telemetry(LpId(2), &telem).unwrap();

    let hb = WorkerHeartbeat {
        lp_id: LpId(1),
        elapsed_since_last_seen: Duration::from_millis(250),
    };
    node_a.send_heartbeat(LpId(2), &hb).unwrap();

    let mut opt_msgs = Vec::new();
    let mut migs = Vec::new();
    let mut telems = Vec::new();
    let mut hbs = Vec::new();

    for _ in 0..50 {
        opt_msgs.extend(node_b.recv_optimistic(LpId(2)).unwrap());
        migs.extend(node_b.recv_migrations());
        telems.extend(node_b.recv_telemetry());
        hbs.extend(node_b.recv_heartbeats());
        if !opt_msgs.is_empty() && !migs.is_empty() && !telems.is_empty() && !hbs.is_empty() {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }

    assert_eq!(opt_msgs.len(), 1);
    assert_eq!(opt_msgs[0].kind(), OptimisticMessageKind::Anti);
    assert_eq!(opt_msgs[0].incarnation(), 4);
    assert_eq!(opt_msgs[0].event().event_payload, b"anti-cancel");

    assert_eq!(migs, vec![mig]);
    assert_eq!(telems, vec![telem]);
    assert_eq!(hbs, vec![hb]);
}
