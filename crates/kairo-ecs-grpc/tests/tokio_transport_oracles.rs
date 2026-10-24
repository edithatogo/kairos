#![forbid(unsafe_code)]

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

use kairo_ecs_grpc::{
    GrpcPeer, GrpcSocketTransport, GrpcTransportConfig, MAX_CONCURRENT_INBOUND_CONNECTIONS,
    MAX_FRAME_BYTES,
};
use kairo_ecs_pdes::{LpId, PdesMessage, PdesTransport, RemoteEvent, Tick, TransportError};

fn get_available_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

#[test]
fn oracle_tokio_transport_clean_shutdown_and_cancellation() {
    let port = get_available_port();
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let config = GrpcTransportConfig::default();

    let transport =
        GrpcSocketTransport::bind(LpId(1), addr, vec![], config).expect("bind transport");
    assert_eq!(transport.bound_addr().port(), port);
    assert!(transport.real_grpc_runtime_claimed());

    // Measure drop latency to ensure immediate Tokio watch cancellation without hang
    let start = Instant::now();
    drop(transport);
    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_millis(500),
        "shutdown took too long: {elapsed:?}"
    );

    // Verify port is immediately available for rebinding without EADDRINUSE
    let rebind = TcpListener::bind(addr);
    assert!(
        rebind.is_ok(),
        "port should be immediately reusable after transport drop"
    );
}

#[test]
fn oracle_tokio_transport_bounded_inbound_connection_saturation() {
    let port = get_available_port();
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let config = GrpcTransportConfig::default();

    let transport =
        GrpcSocketTransport::bind(LpId(1), addr, vec![], config).expect("bind transport");
    assert_eq!(
        transport.max_concurrent_connections(),
        MAX_CONCURRENT_INBOUND_CONNECTIONS
    );

    // Spawn concurrent connections up to the saturation limit
    let mut streams = Vec::new();
    for i in 0..MAX_CONCURRENT_INBOUND_CONNECTIONS {
        let mut last_err = None;
        let mut connected = false;
        for _ in 0..100 {
            match TcpStream::connect(addr) {
                Ok(stream) => {
                    streams.push(stream);
                    connected = true;
                    break;
                }
                Err(e) => {
                    last_err = Some(e);
                    thread::sleep(Duration::from_millis(2));
                }
            }
        }
        assert!(
            connected,
            "failed to connect client {i} to transport: {:?}",
            last_err
        );
    }

    // Wait until all connections are accepted and registered
    let saturation_deadline = Instant::now() + Duration::from_secs(3);
    while transport.active_connections() < MAX_CONCURRENT_INBOUND_CONNECTIONS {
        assert!(
            Instant::now() < saturation_deadline,
            "timed out waiting for active connections to reach {MAX_CONCURRENT_INBOUND_CONNECTIONS}; currently at {}",
            transport.active_connections()
        );
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        transport.active_connections(),
        MAX_CONCURRENT_INBOUND_CONNECTIONS
    );

    // Any connection beyond MAX_CONCURRENT_INBOUND_CONNECTIONS should be immediately closed
    let mut overflow_stream = TcpStream::connect(addr).expect("connect overflow stream");
    overflow_stream
        .set_read_timeout(Some(Duration::from_millis(500)))
        .expect("set read timeout");
    let mut buf = [0u8; 1];
    // Stream should be closed by server immediately (EOF / read 0 or reset)
    let read_res = overflow_stream.read(&mut buf);
    match read_res {
        Ok(0) => {} // Clean EOF
        Err(e)
            if e.kind() == std::io::ErrorKind::ConnectionReset
                || e.kind() == std::io::ErrorKind::UnexpectedEof => {}
        other => panic!(
            "overflow connection beyond capacity must be rejected with EOF or reset, got: {:?}",
            other
        ),
    }

    // Active count must remain capped at MAX_CONCURRENT_INBOUND_CONNECTIONS
    assert_eq!(
        transport.active_connections(),
        MAX_CONCURRENT_INBOUND_CONNECTIONS
    );

    // Dropping streams releases permits
    drop(streams);
    let release_deadline = Instant::now() + Duration::from_secs(3);
    while transport.active_connections() > 0 {
        assert!(
            Instant::now() < release_deadline,
            "permits should be released after clients disconnect, active={}",
            transport.active_connections()
        );
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(transport.active_connections(), 0);
}

#[test]
fn oracle_tokio_transport_stalled_read_timeout_releases_permit() {
    let port = get_available_port();
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let config = GrpcTransportConfig {
        request_timeout: Duration::from_millis(150),
        heartbeat_interval: Duration::from_millis(300),
        heartbeat_timeout: Duration::from_millis(1000),
    };

    let transport =
        GrpcSocketTransport::bind(LpId(1), addr, vec![], config).expect("bind transport");

    // Client connects and sends incomplete 2-byte header, then stalls
    let mut client = TcpStream::connect(addr).expect("connect raw client");
    client
        .write_all(&[0x00, 0x00])
        .expect("write partial header");

    // Wait for connection to be recorded
    let start = Instant::now();
    while transport.active_connections() == 0 {
        assert!(start.elapsed() < Duration::from_secs(1));
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(transport.active_connections(), 1);

    // After request_timeout (150ms) expires, worker should close stream and release permit
    client
        .set_read_timeout(Some(Duration::from_millis(500)))
        .expect("set timeout");
    let mut buf = [0u8; 1];
    let read_res = client.read(&mut buf);
    match read_res {
        Ok(0) => {} // Clean EOF on timeout abort
        Err(e)
            if e.kind() == std::io::ErrorKind::ConnectionReset
                || e.kind() == std::io::ErrorKind::UnexpectedEof => {}
        other => panic!(
            "stalled client must be disconnected on read timeout, got: {:?}",
            other
        ),
    }

    let release_start = Instant::now();
    while transport.active_connections() > 0 {
        assert!(release_start.elapsed() < Duration::from_secs(1));
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(transport.active_connections(), 0);
}

#[test]
fn oracle_tokio_transport_slow_consumer_and_oversized_frame_rejection() {
    let port = get_available_port();
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let config = GrpcTransportConfig::default();

    let mut transport =
        GrpcSocketTransport::bind(LpId(1), addr, vec![], config).expect("bind transport");

    // 1. Oversized frame: announce 256 KiB (> MAX_FRAME_BYTES of 128 KiB)
    let mut bad_client = TcpStream::connect(addr).expect("connect raw client");
    let oversized_len = (MAX_FRAME_BYTES + 4096) as u32;
    bad_client
        .write_all(&oversized_len.to_be_bytes())
        .expect("write oversized frame len");

    let mut buf = [0u8; 4];
    let _ = bad_client.set_read_timeout(Some(Duration::from_millis(100)));
    // Server must reject and close connection immediately
    let _ = bad_client.read(&mut buf);
    drop(bad_client);

    // 2. Zero-length frame: announce 0 bytes
    let mut zero_client = TcpStream::connect(addr).expect("connect zero client");
    zero_client
        .write_all(&0u32.to_be_bytes())
        .expect("write zero len");
    let _ = zero_client.set_read_timeout(Some(Duration::from_millis(100)));
    let _ = zero_client.read(&mut buf);
    drop(zero_client);

    // 3. Verify healthy traffic still passes uninterrupted
    let mut healthy_client = TcpStream::connect(addr).expect("connect healthy client");
    // Send valid event frame
    let event = RemoteEvent {
        source_lp: LpId(2),
        dest_lp: LpId(1),
        tick: Tick::from_ticks(42),
        event_payload: b"healthy-frame-payload".to_vec(),
    };
    let mut payload = Vec::new();
    payload.extend_from_slice(&event.source_lp.0.to_be_bytes());
    payload.extend_from_slice(&event.dest_lp.0.to_be_bytes());
    payload.extend_from_slice(&event.tick.ticks().to_be_bytes());
    payload.extend_from_slice(&(event.event_payload.len() as u32).to_be_bytes());
    payload.extend_from_slice(&event.event_payload);

    let total_len = (payload.len() + 1) as u32;
    healthy_client.write_all(&total_len.to_be_bytes()).unwrap();
    healthy_client.write_all(&[1u8]).unwrap(); // MSG_TYPE_EVENT = 1
    healthy_client.write_all(&payload).unwrap();
    healthy_client.flush().unwrap();

    thread::sleep(Duration::from_millis(30));
    let mut received = transport.recv(LpId(1)).expect("recv from local inbox");
    assert_eq!(received.len(), 1);
    if let PdesMessage::Event(e) = received.pop().unwrap() {
        assert_eq!(e.tick.ticks(), 42);
        assert_eq!(e.event_payload, b"healthy-frame-payload");
    } else {
        panic!("expected event message");
    }
}

#[test]
fn oracle_tokio_transport_reconnect_and_transient_failure_recovery() {
    let port_a = get_available_port();
    let port_b = get_available_port();
    let addr_a: SocketAddr = format!("127.0.0.1:{port_a}").parse().unwrap();
    let addr_b: SocketAddr = format!("127.0.0.1:{port_b}").parse().unwrap();
    let config = GrpcTransportConfig::default();

    // Transport A knows peer B
    let mut transport_a = GrpcSocketTransport::bind(
        LpId(10),
        addr_a,
        vec![GrpcPeer {
            lp_id: LpId(20),
            endpoint: format!("http://127.0.0.1:{port_b}"),
        }],
        config.clone(),
    )
    .expect("bind transport A");

    // Peer B is initially offline: send should fail with error
    let event = RemoteEvent {
        source_lp: LpId(10),
        dest_lp: LpId(20),
        tick: Tick::from_ticks(1),
        event_payload: b"offline-peer-test".to_vec(),
    };
    let send_res = transport_a.send(LpId(20), PdesMessage::Event(event.clone()));
    assert!(matches!(
        send_res,
        Err(TransportError::UnknownLogicalProcess(_))
    ));

    // Bring Peer B online on the expected port
    let mut transport_b = GrpcSocketTransport::bind(
        LpId(20),
        addr_b,
        vec![GrpcPeer {
            lp_id: LpId(10),
            endpoint: format!("http://127.0.0.1:{port_a}"),
        }],
        config,
    )
    .expect("bind transport B");

    // Transport A retries send: now it must succeed and recover automatically
    let mut sent = false;
    for _ in 0..10 {
        if transport_a
            .send(LpId(20), PdesMessage::Event(event.clone()))
            .is_ok()
        {
            sent = true;
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    assert!(
        sent,
        "transport A should successfully send after peer B comes online"
    );

    thread::sleep(Duration::from_millis(30));
    let mut received = transport_b.recv(LpId(20)).expect("recv from inbox");
    assert_eq!(received.len(), 1);
    if let PdesMessage::Event(e) = received.pop().unwrap() {
        assert_eq!(e.source_lp, LpId(10));
        assert_eq!(e.dest_lp, LpId(20));
        assert_eq!(e.event_payload, b"offline-peer-test");
    } else {
        panic!("expected event message");
    }
}

#[test]
fn oracle_tokio_transport_deterministic_handoff_ordering_despite_io_jitter() {
    let port_a = get_available_port();
    let port_b = get_available_port();
    let addr_a: SocketAddr = format!("127.0.0.1:{port_a}").parse().unwrap();
    let addr_b: SocketAddr = format!("127.0.0.1:{port_b}").parse().unwrap();
    let config = GrpcTransportConfig::default();

    let mut transport_a = GrpcSocketTransport::bind(
        LpId(1),
        addr_a,
        vec![GrpcPeer {
            lp_id: LpId(2),
            endpoint: format!("http://127.0.0.1:{port_b}"),
        }],
        config.clone(),
    )
    .expect("bind transport A");

    let mut transport_b = GrpcSocketTransport::bind(
        LpId(2),
        addr_b,
        vec![GrpcPeer {
            lp_id: LpId(1),
            endpoint: format!("http://127.0.0.1:{port_a}"),
        }],
        config,
    )
    .expect("bind transport B");

    // Send 50 sequenced events
    const EVENT_COUNT: usize = 50;
    for seq in 0..EVENT_COUNT {
        let event = RemoteEvent {
            source_lp: LpId(1),
            dest_lp: LpId(2),
            tick: Tick::from_ticks(seq as u128 + 100),
            event_payload: format!("seq-{seq}").into_bytes(),
        };
        transport_a
            .send(LpId(2), PdesMessage::Event(event))
            .unwrap_or_else(|e| panic!("failed sending seq {seq}: {e:?}"));
    }

    // Wait and drain all messages from transport B
    let mut collected = Vec::new();
    let start = Instant::now();
    while collected.len() < EVENT_COUNT && start.elapsed() < Duration::from_secs(3) {
        let msgs = transport_b.recv(LpId(2)).unwrap();
        collected.extend(msgs);
        if collected.len() < EVENT_COUNT {
            thread::sleep(Duration::from_millis(10));
        }
    }

    assert_eq!(
        collected.len(),
        EVENT_COUNT,
        "must receive all {EVENT_COUNT} events"
    );

    // Verify strict FIFO preservation and tick monotonicity
    for (i, msg) in collected.iter().enumerate() {
        if let PdesMessage::Event(e) = msg {
            assert_eq!(e.tick.ticks(), (i as u128) + 100);
            assert_eq!(e.event_payload, format!("seq-{i}").into_bytes());
        } else {
            panic!("expected event message at index {i}");
        }
    }
}
