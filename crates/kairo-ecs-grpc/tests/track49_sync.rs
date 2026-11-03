// SPDX-License-Identifier: Apache-2.0
use kairo_ecs_grpc::*;
use kairo_ecs_pdes::{LpId, Tick};
use kairo_ecs_types::EntityId;
#[cfg(feature = "telemetry-ipc")]
use std::collections::BTreeSet;
use std::time::Duration;

fn pair() -> (TonicTransport, TonicTransport) {
    let b = TonicTransport::bind(
        LpId(2),
        "127.0.0.1:0".parse().unwrap(),
        vec![GrpcPeer {
            lp_id: LpId(1),
            endpoint: "http://127.0.0.1:1".into(),
        }],
        GrpcTransportConfig::default(),
    )
    .unwrap();
    let a = TonicTransport::bind(
        LpId(1),
        "127.0.0.1:0".parse().unwrap(),
        vec![GrpcPeer {
            lp_id: LpId(2),
            endpoint: format!("http://{}", b.bound_addr()),
        }],
        GrpcTransportConfig::default(),
    )
    .unwrap();
    (a, b)
}
fn snapshot() -> GrpcEntityMigrationSnapshot {
    GrpcEntityMigrationSnapshot {
        entity: EntityId::new(17, 9),
        source_lp: LpId(1),
        dest_lp: LpId(2),
        migration_id: "entity17-transfer3".into(),
        components: vec![GrpcVersionedComponentBlob {
            component_type_id: "entity-state-with-pending-payloads".into(),
            generation: 97,
            payload: vec![8, 9, 10],
        }],
        pending_events: vec![
            GrpcPendingEventMetadata {
                source_lp: LpId(1),
                dest_lp: LpId(2),
                tick: Tick::from_ticks((1u128 << 100) + 23),
                sequence: 51,
                payload_bytes: 3,
                anti_message: false,
            },
            GrpcPendingEventMetadata {
                source_lp: LpId(2),
                dest_lp: LpId(1),
                tick: Tick::from_ticks((1u128 << 100) + 24),
                sequence: 52,
                payload_bytes: 0,
                anti_message: true,
            },
            GrpcPendingEventMetadata {
                source_lp: LpId(1),
                dest_lp: LpId(1),
                tick: Tick::from_ticks((1u128 << 100) + 25),
                sequence: 53,
                payload_bytes: 2,
                anti_message: false,
            },
        ],
    }
}
#[test]
fn actual_snapshot_rpc_preserves_all_generations_pending_and_u128_without_application_claim() {
    let (a, b) = pair();
    let s = snapshot();
    let bytes = encode_migration_snapshot(&s).unwrap();
    assert_eq!(decode_migration_snapshot(&bytes).unwrap(), s);
    let mut unknown = bytes.clone();
    unknown.extend_from_slice(&[0x78, 0x01]);
    assert_eq!(
        decode_migration_snapshot(&unknown)
            .unwrap_err()
            .status
            .code(),
        tonic::Code::InvalidArgument
    );
    let ack = a.send_migration_snapshot(LpId(2), &s).unwrap();
    assert!(!ack.applied);
    assert!(ack.message.contains("not durable/model-applied"));
    let got = b.drain_migration_snapshots();
    assert_eq!(
        got,
        vec![ReceivedMigrationSnapshot {
            version: 2,
            snapshot: s.clone()
        }]
    );
    a.send_migration_snapshot(LpId(2), &s).unwrap();
    assert!(b.drain_migration_snapshots().is_empty());
    let mut conflict = s.clone();
    conflict.components[0].payload.push(11);
    let error = a.send_migration_snapshot(LpId(2), &conflict).unwrap_err();
    assert_eq!(error.status.code(), tonic::Code::AlreadyExists);
    assert_eq!(error.kind, GrpcFailureKind::Permanent);
    assert!(b.drain_migration_snapshots().is_empty());
    let mut bad = s.clone();
    bad.components.push(bad.components[0].clone());
    assert_eq!(
        a.send_migration_snapshot(LpId(2), &bad)
            .unwrap_err()
            .status
            .code(),
        tonic::Code::InvalidArgument
    );
    assert!(b.drain_migration_snapshots().is_empty());
}
// An independent live gRPC peer returns an explicit application status. A closed
// Windows TCP port may remain pending until the client deadline, so it cannot
// deterministically stand in for an observed Unavailable response.
#[derive(Clone)]
struct ExplicitStatus(std::sync::Arc<std::sync::atomic::AtomicUsize>, tonic::Code);
impl tonic::server::NamedService for ExplicitStatus {
    const NAME: &'static str = "kairo.ecs.simulation.v1.SimulationTransport";
}
impl tonic::codegen::Service<tonic::codegen::http::Request<tonic::body::Body>> for ExplicitStatus {
    type Response = tonic::codegen::http::Response<tonic::body::Body>;
    type Error = std::convert::Infallible;
    type Future = std::future::Ready<Result<Self::Response, Self::Error>>;
    fn poll_ready(
        &mut self,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }
    fn call(&mut self, request: tonic::codegen::http::Request<tonic::body::Body>) -> Self::Future {
        assert_eq!(
            request.uri().path(),
            "/kairo.ecs.simulation.v1.SimulationTransport/MigrateSnapshot"
        );
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let mut status = tonic::Status::with_details(
            self.1,
            if self.1 == tonic::Code::Unknown {
                "transport error"
            } else {
                "qualification peer explicitly unavailable"
            },
            tonic::codegen::Bytes::from_static(b"qualification-unavailable-details"),
        );
        status
            .metadata_mut()
            .insert("x-qualification", "explicit-unavailable".parse().unwrap());
        std::future::ready(Ok(status.into_http()))
    }
}
#[test]
fn real_unavailable_and_blackhole_deadline_are_distinct_without_takeover_authority() {
    // Wire application statuses cannot carry a local transport source chain.
    // A server Unknown with the same misleading text must remain permanent.
    for code in [tonic::Code::Unavailable, tonic::Code::Unknown] {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let service = ExplicitStatus(std::sync::Arc::clone(&calls), code);
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let server = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async move {
                let listener = tokio::net::TcpListener::from_std(listener).unwrap();
                ready_tx.send(()).unwrap();
                tonic::transport::Server::builder()
                    .add_service(service)
                    .serve_with_incoming_shutdown(
                        tokio_stream::wrappers::TcpListenerStream::new(listener),
                        async {
                            let _ = shutdown_rx.await;
                        },
                    )
                    .await
                    .unwrap();
            });
        });
        ready_rx.recv().unwrap();
        let sender = TonicTransport::bind(
            LpId(1),
            "127.0.0.1:0".parse().unwrap(),
            vec![GrpcPeer {
                lp_id: LpId(2),
                endpoint: format!("http://{address}"),
            }],
            GrpcTransportConfig::default(),
        )
        .unwrap();
        let error = sender
            .send_migration_snapshot(LpId(2), &snapshot())
            .unwrap_err();
        assert_eq!(
            error.kind,
            if code == tonic::Code::Unavailable {
                GrpcFailureKind::Transient
            } else {
                GrpcFailureKind::Permanent
            }
        );
        assert_eq!(error.status.code(), code);
        assert_eq!(
            error.status.message(),
            if code == tonic::Code::Unknown {
                "transport error"
            } else {
                "qualification peer explicitly unavailable"
            }
        );
        assert_eq!(error.status.details(), b"qualification-unavailable-details");
        assert_eq!(
            error.status.metadata().get("x-qualification").unwrap(),
            "explicit-unavailable"
        );
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
        let error = sender
            .send_migration_snapshot(LpId(99), &snapshot())
            .unwrap_err();
        assert_eq!(error.kind, GrpcFailureKind::Permanent);
        assert_eq!(error.status.code(), tonic::Code::InvalidArgument);
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
        drop(sender);
        shutdown_tx.send(()).unwrap();
        server.join().unwrap();
    }
    // Keep the listener held without an application handler. This is a real
    // connection/HTTP2 blackhole and must expire at the existing short deadline.
    let blackhole = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let sender = TonicTransport::bind(
        LpId(1),
        "127.0.0.1:0".parse().unwrap(),
        vec![GrpcPeer {
            lp_id: LpId(2),
            endpoint: format!("http://{}", blackhole.local_addr().unwrap()),
        }],
        GrpcTransportConfig {
            request_timeout: Duration::from_millis(120),
            ..Default::default()
        },
    )
    .unwrap();
    let error = sender
        .send_migration_snapshot(LpId(2), &snapshot())
        .unwrap_err();
    assert_eq!(error.kind, GrpcFailureKind::PartitionSuspected);
    assert_eq!(error.status.code(), tonic::Code::DeadlineExceeded);
    drop(sender);
    drop(blackhole);
}
#[test]
fn accepted_connection_closed_during_rpc_is_transient_without_rewriting_status() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let peer = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut preface = [0; 24];
        socket.read_exact(&mut preface).unwrap();
        assert_eq!(&preface, b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n");
        socket.write_all(&[0, 0, 0, 4, 0, 0, 0, 0, 0]).unwrap();
        loop {
            let mut header = [0; 9];
            socket.read_exact(&mut header).unwrap();
            let len = (usize::from(header[0]) << 16)
                | (usize::from(header[1]) << 8)
                | usize::from(header[2]);
            assert!(len <= 65536);
            let mut payload = vec![0; len];
            socket.read_exact(&mut payload).unwrap();
            if header[3] == 4 && header[4] & 1 == 0 {
                socket.write_all(&[0, 0, 0, 4, 1, 0, 0, 0, 0]).unwrap();
            }
            if header[3] == 1 {
                assert_ne!(&header[5..9], &[0, 0, 0, 0]);
                socket.shutdown(std::net::Shutdown::Both).unwrap();
                return;
            }
        }
    });
    let sender = TonicTransport::bind(
        LpId(1),
        "127.0.0.1:0".parse().unwrap(),
        vec![GrpcPeer {
            lp_id: LpId(2),
            endpoint: format!("http://{address}"),
        }],
        GrpcTransportConfig::default(),
    )
    .unwrap();
    let error = sender
        .send_migration_snapshot(LpId(2), &snapshot())
        .unwrap_err();
    peer.join().unwrap();
    assert_eq!(error.status.code(), tonic::Code::Unknown);
    assert_eq!(error.status.message(), "transport error");
    assert!(error.status.details().is_empty());
    assert!(error.status.metadata().is_empty());
    assert!(std::error::Error::source(&error.status)
        .unwrap()
        .is::<tonic::transport::Error>());
    assert_eq!(error.kind, GrpcFailureKind::Transient);
}
#[test]
fn unknown_text_and_unrelated_io_are_not_connectivity_evidence() {
    for io in [
        None,
        Some(std::io::ErrorKind::BrokenPipe),
        Some(std::io::ErrorKind::PermissionDenied),
    ] {
        let mut status = tonic::Status::with_details(
            tonic::Code::Unknown,
            "transport error",
            tonic::codegen::Bytes::from_static(b"original-unknown-details"),
        );
        status
            .metadata_mut()
            .insert("x-original", "retained".parse().unwrap());
        if let Some(kind) = io {
            status.set_source(std::sync::Arc::new(std::io::Error::new(
                kind,
                "transport error",
            )));
        }
        let error = GrpcCallFailure::from(status);
        assert_eq!(error.kind, GrpcFailureKind::Permanent);
        assert_eq!(error.status.code(), tonic::Code::Unknown);
        assert_eq!(error.status.message(), "transport error");
        assert_eq!(error.status.details(), b"original-unknown-details");
        assert_eq!(
            error.status.metadata().get("x-original").unwrap(),
            "retained"
        );
        assert_eq!(
            std::error::Error::source(&error.status).is_some(),
            io.is_some()
        );
    }
}
#[test]
fn unknown_with_typed_tonic_configuration_error_remains_permanent() {
    let configuration_error =
        tonic::transport::Endpoint::from_shared("http://[".to_owned()).unwrap_err();
    let mut status = tonic::Status::unknown("transport error");
    status.set_source(std::sync::Arc::new(configuration_error));
    let failure = GrpcCallFailure::from(status);
    assert_eq!(failure.kind, GrpcFailureKind::Permanent);
    assert_eq!(failure.status.code(), tonic::Code::Unknown);
    assert!(std::error::Error::source(&failure.status)
        .unwrap()
        .is::<tonic::transport::Error>());
}
#[test]
fn independent_protoc_canonical_wire_matches_versioned_prost_schema() {
    use prost::Message;
    let wire = envelope::WireMigrationSnapshotRequest {
        version: 2,
        snapshot: Some(envelope::WireMigrationRequest {
            entity_index: 17,
            entity_generation: 9,
            source_lp: 1,
            dest_lp: 2,
            migration_id: "x".into(),
            components: vec![envelope::WireComponentBlob {
                component_type_id: "c".into(),
                generation: 97,
                payload: vec![8, 9, 10],
            }],
            pending_events: vec![envelope::WirePendingEvent {
                source_lp: 1,
                dest_lp: 2,
                tick: 23u128.to_be_bytes().to_vec(),
                sequence: 51,
                payload_bytes: 3,
                anti_message: false,
            }],
        }),
    };
    // Independently produced with protoc36.2 --encode using simulation.proto.
    let golden = [
        8, 2, 18, 51, 8, 17, 16, 9, 24, 1, 32, 2, 42, 1, 120, 50, 10, 10, 1, 99, 16, 97, 26, 3, 8,
        9, 10, 58, 26, 8, 1, 16, 2, 26, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 23, 32,
        51, 40, 3,
    ];
    assert_eq!(wire.encode_to_vec(), golden);
    assert_eq!(
        envelope::WireMigrationSnapshotRequest::decode(golden.as_slice()).unwrap(),
        wire
    );
}

#[cfg(feature = "telemetry-ipc")]
mod ipc {
    use super::*;
    use kairo_ecs_arrow::polars_interop::{
        dataframe_to_records, read_ipc_stream, records_to_dataframe, write_ipc_stream,
    };
    use kairo_ecs_arrow::{EventLogRecord, EventStatus, SCHEMA_VERSION_V2};
    use kairo_ecs_types::EventId;
    fn batch(id: &str, seq: u64, source: u32) -> VersionedTelemetryBatch {
        let time = (1u128 << 100) + u128::from(seq);
        let row = EventLogRecord {
            schema_version: SCHEMA_VERSION_V2,
            run_id: "track49".into(),
            event_id: EventId::new(seq, 3),
            entity_id: Some(EntityId::new(17, 9)),
            time_ticks: time,
            time_scale: "ticks".into(),
            priority: 0,
            sequence: seq,
            event_kind: "migration".into(),
            status: EventStatus::Dispatched,
            payload_ref: Some(format!("{id}/payload")),
        };
        let mut df = records_to_dataframe(&[row]).unwrap();
        let mut data = Vec::new();
        write_ipc_stream(&mut df, &mut data).unwrap();
        VersionedTelemetryBatch {
            version: 2,
            batch_id: id.into(),
            run_id: "track49".into(),
            source_lp: LpId(source),
            tick_start: Tick::from_ticks(time),
            tick_end: Tick::from_ticks(time),
            arrow_ipc_payload: data,
        }
    }
    #[test]
    fn actual_stream_and_standard_ipc_merge_are_ordered_exact_and_duplicate_safe() {
        let (a, b) = pair();
        let high = batch("high", 9, 1);
        let low = batch("low", 3, 1);
        let ack = a
            .send_versioned_telemetry(LpId(2), &[high.clone(), low.clone()])
            .unwrap();
        assert_eq!(ack.accepted_batches, 2);
        assert_eq!(
            a.send_versioned_telemetry(LpId(2), &[high.clone(), low.clone()])
                .unwrap()
                .accepted_batches,
            0
        );
        let got = b.drain_versioned_telemetry();
        assert_eq!(got, vec![high.clone(), low.clone()]);
        let expected = BTreeSet::from([LpId(1), LpId(2)]);
        let merged = merge_telemetry_batches(&got, &expected).unwrap();
        assert_eq!(merged.missing_sources, vec![LpId(2)]);
        assert_eq!(
            merged
                .records
                .iter()
                .map(|r| r.sequence)
                .collect::<Vec<_>>(),
            vec![3, 9]
        );
        assert_eq!(merged.row_sources, vec![LpId(1), LpId(1)]);
        let decoded = dataframe_to_records(
            &read_ipc_stream(std::io::Cursor::new(&merged.arrow_ipc_payload)).unwrap(),
        )
        .unwrap();
        assert_eq!(decoded, merged.records);
        let mut conflict = high.clone();
        conflict.arrow_ipc_payload = low.arrow_ipc_payload.clone();
        conflict.tick_start = low.tick_start;
        conflict.tick_end = low.tick_end;
        assert_eq!(
            a.send_versioned_telemetry(LpId(2), &[batch("new", 7, 1), conflict])
                .unwrap_err()
                .status
                .code(),
            tonic::Code::AlreadyExists
        );
        assert!(b.drain_versioned_telemetry().is_empty());
        let mut malformed = batch("bad", 8, 1);
        malformed.arrow_ipc_payload = b"ARROW1\0\0not-standard-IPC".to_vec();
        assert!(a.send_versioned_telemetry(LpId(2), &[malformed]).is_err());
        assert!(b.drain_versioned_telemetry().is_empty());
    }
    #[test]
    fn raw_stream_invalid_tail_is_atomic_and_legacy_checked_range_is_real_rpc() {
        use kairo_ecs_grpc::envelope::WireTelemetryBatchV2;
        use kairo_ecs_grpc::service::simulation_transport_client::SimulationTransportClient;
        let (a, b) = pair();
        let valid = batch("atomic-prefix", 5, 1);
        let wire = WireTelemetryBatchV2 {
            version: 2,
            batch_id: valid.batch_id.clone(),
            run_id: valid.run_id.clone(),
            source_lp: 1,
            arrow_ipc_payload: valid.arrow_ipc_payload.clone(),
            tick_start: valid.tick_start.ticks().to_be_bytes().to_vec(),
            tick_end: valid.tick_end.ticks().to_be_bytes().to_vec(),
        };
        let mut malformed = wire.clone();
        malformed.batch_id = "truncated-tail".into();
        malformed.arrow_ipc_payload.pop();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let mut client = runtime
            .block_on(SimulationTransportClient::connect(format!(
                "http://{}",
                b.bound_addr()
            )))
            .unwrap();
        let error = runtime
            .block_on(client.stream_telemetry_v2(tokio_stream::iter(vec![wire, malformed])))
            .unwrap_err();
        assert_eq!(error.code(), tonic::Code::InvalidArgument);
        assert!(b.drain_versioned_telemetry().is_empty());
        assert_eq!(
            a.send_versioned_telemetry(LpId(2), std::slice::from_ref(&valid))
                .unwrap()
                .accepted_batches,
            1
        );
        b.drain_versioned_telemetry();
        let expected = BTreeSet::from([LpId(1)]);
        let merged = merge_telemetry_batches(&[valid], &expected).unwrap();
        let mut records = merged.records;
        records[0].time_ticks = 5;
        let mut df = records_to_dataframe(&records).unwrap();
        let mut payload = Vec::new();
        write_ipc_stream(&mut df, &mut payload).unwrap();
        let legacy = GrpcTelemetryBatch {
            source_lp: LpId(1),
            tick_start: Tick::from_ticks(5),
            tick_end: Tick::from_ticks(5),
            arrow_ipc_payload: payload,
        };
        let ack = a
            .send_telemetry(LpId(2), std::slice::from_ref(&legacy))
            .unwrap();
        assert!(!ack.applied);
        let got = b.drain_versioned_telemetry();
        assert_eq!(got.len(), 1);
        assert!(got[0].batch_id.starts_with("legacy-v1-"));
        a.send_telemetry(LpId(2), std::slice::from_ref(&legacy))
            .unwrap();
        assert!(b.drain_versioned_telemetry().is_empty());
        let too_wide = GrpcTelemetryBatch {
            tick_end: Tick::from_ticks(1u128 << 100),
            ..legacy
        };
        assert_eq!(
            a.send_telemetry(LpId(2), &[too_wide])
                .unwrap_err()
                .status
                .code(),
            tonic::Code::InvalidArgument
        );
    }
    #[test]
    fn valid_framed_deep_and_wide_schemas_are_rejected_before_upstream_decode() {
        use kairo_ecs_grpc::envelope::WireTelemetryBatchV2;
        use kairo_ecs_grpc::service::simulation_transport_client::SimulationTransportClient;
        // Generated once with polars-arrow-format0.2.1 / planus1.1.1 owned
        // schema builders and independently read through MessageRef. Both are
        // valid Schema+EOS streams:64 list levels, and128 flat Int64 fields.
        let fixtures=["ffffffff880b000004000000f2ffffff140000000400010000000a000b0008000a000400f8ffffff0c0000000800080000000400010000000400000008f9ffff400b0000340b000008000000010c0000010000000400000024f9ffff100b0000080b000008000000010c0000010000000400000040f9ffffe40a0000dc0a000008000000010c000001000000040000005cf9ffffb80a0000b00a000008000000010c0000010000000400000078f9ffff8c0a0000840a000008000000010c0000010000000400000094f9ffff600a0000580a000008000000010c00000100000004000000b0f9ffff340a00002c0a000008000000010c00000100000004000000ccf9ffff080a0000000a000008000000010c00000100000004000000e8f9ffffdc090000d409000008000000010c0000010000000400000004faffffb0090000a809000008000000010c0000010000000400000020faffff840900007c09000008000000010c000001000000040000003cfaffff580900005009000008000000010c0000010000000400000058faffff2c0900002409000008000000010c0000010000000400000074faffff00090000f808000008000000010c0000010000000400000090faffffd4080000cc08000008000000010c00000100000004000000acfaffffa8080000a008000008000000010c00000100000004000000c8faffff7c0800007408000008000000010c00000100000004000000e4faffff500800004808000008000000010c0000010000000400000000fbffff240800001c08000008000000010c000001000000040000001cfbfffff8070000f007000008000000010c0000010000000400000038fbffffcc070000c407000008000000010c0000010000000400000054fbffffa00700009807000008000000010c0000010000000400000070fbffff740700006c07000008000000010c000001000000040000008cfbffff480700004007000008000000010c00000100000004000000a8fbffff1c0700001407000008000000010c00000100000004000000c4fbfffff0060000e806000008000000010c00000100000004000000e0fbffffc4060000bc06000008000000010c00000100000004000000fcfbffff980600009006000008000000010c0000010000000400000018fcffff6c0600006406000008000000010c0000010000000400000034fcffff400600003806000008000000010c0000010000000400000050fcffff140600000c06000008000000010c000001000000040000006cfcffffe8050000e005000008000000010c0000010000000400000088fcffffbc050000b405000008000000010c00000100000004000000a4fcffff900500008805000008000000010c00000100000004000000c0fcffff640500005c05000008000000010c00000100000004000000dcfcffff380500003005000008000000010c00000100000004000000f8fcffff0c0500000405000008000000010c0000010000000400000014fdffffe0040000d804000008000000010c0000010000000400000030fdffffb4040000ac04000008000000010c000001000000040000004cfdffff880400008004000008000000010c0000010000000400000068fdffff5c0400005404000008000000010c0000010000000400000084fdffff300400002804000008000000010c00000100000004000000a0fdffff04040000fc03000008000000010c00000100000004000000bcfdffffd8030000d003000008000000010c00000100000004000000d8fdffffac030000a403000008000000010c00000100000004000000f4fdffff800300007803000008000000010c0000010000000400000010feffff540300004c03000008000000010c000001000000040000002cfeffff280300002003000008000000010c0000010000000400000048fefffffc020000f402000008000000010c0000010000000400000064feffffd0020000c802000008000000010c0000010000000400000080feffffa40200009c02000008000000010c000001000000040000009cfeffff780200007002000008000000010c00000100000004000000b8feffff4c0200004402000008000000010c00000100000004000000d4feffff200200001802000008000000010c00000100000004000000f0fefffff4010000ec01000008000000010c000001000000040000000cffffffc8010000c001000008000000010c0000010000000400000028ffffff9c0100009401000008000000010c0000010000000400000044ffffff700100006801000008000000010c0000010000000400000060ffffff440100003c01000008000000010c000001000000040000007cffffff180100001001000008000000010c0000010000000400000098ffffffec000000e400000008000000010c00000100000004000000b4ffffffc0000000b800000008000000010c00000100000004000000d0ffffff940000008c00000008000000010c00000100000004000000ecffffff680000006000000018000000010c000010001200040010001100080000000c000100000004000000f0ffffff2c00000014000000010200000c000e0004000c000d000800f4ffffff40000000010000000800090004000800040000006c656166000000000cfcffff060000006c6973742d3000001cfcffff060000006c6973742d3100002cfcffff060000006c6973742d3200003cfcffff060000006c6973742d3300004cfcffff060000006c6973742d3400005cfcffff060000006c6973742d3500006cfcffff060000006c6973742d3600007cfcffff060000006c6973742d3700008cfcffff060000006c6973742d3800009cfcffff060000006c6973742d390000acfcffff070000006c6973742d313000bcfcffff070000006c6973742d313100ccfcffff070000006c6973742d313200dcfcffff070000006c6973742d313300ecfcffff070000006c6973742d313400fcfcffff070000006c6973742d3135000cfdffff070000006c6973742d3136001cfdffff070000006c6973742d3137002cfdffff070000006c6973742d3138003cfdffff070000006c6973742d3139004cfdffff070000006c6973742d3230005cfdffff070000006c6973742d3231006cfdffff070000006c6973742d3232007cfdffff070000006c6973742d3233008cfdffff070000006c6973742d3234009cfdffff070000006c6973742d323500acfdffff070000006c6973742d323600bcfdffff070000006c6973742d323700ccfdffff070000006c6973742d323800dcfdffff070000006c6973742d323900ecfdffff070000006c6973742d333000fcfdffff070000006c6973742d3331000cfeffff070000006c6973742d3332001cfeffff070000006c6973742d3333002cfeffff070000006c6973742d3334003cfeffff070000006c6973742d3335004cfeffff070000006c6973742d3336005cfeffff070000006c6973742d3337006cfeffff070000006c6973742d3338007cfeffff070000006c6973742d3339008cfeffff070000006c6973742d3430009cfeffff070000006c6973742d343100acfeffff070000006c6973742d343200bcfeffff070000006c6973742d343300ccfeffff070000006c6973742d343400dcfeffff070000006c6973742d343500ecfeffff070000006c6973742d343600fcfeffff070000006c6973742d3437000cffffff070000006c6973742d3438001cffffff070000006c6973742d3439002cffffff070000006c6973742d3530003cffffff070000006c6973742d3531004cffffff070000006c6973742d3532005cffffff070000006c6973742d3533006cffffff070000006c6973742d3534007cffffff070000006c6973742d3535008cffffff070000006c6973742d3536009cffffff070000006c6973742d353700acffffff070000006c6973742d353800bcffffff070000006c6973742d353900ccffffff070000006c6973742d363000dcffffff070000006c6973742d363100ecffffff070000006c6973742d363200fcffffff04000400070000006c6973742d36330000000000ffffffff00000000", "ffffffff2018000004000000f2ffffff140000000400010000000a000b0008000a000400f8ffffff0c000000080008000000040080000000b017000084170000581700002c17000000170000d4160000a81600007c1600005016000024160000f4150000c415000094150000641500003415000004150000d4140000a4140000741400004414000014140000e4130000b4130000841300005413000024130000f4120000c412000094120000641200003412000004120000d4110000a4110000741100004411000014110000e4100000b4100000841000005410000024100000f40f0000c40f0000940f0000640f0000340f0000040f0000d40e0000a40e0000740e0000440e0000140e0000e40d0000b40d0000840d0000540d0000240d0000f40c0000c40c0000940c0000640c0000340c0000040c0000d40b0000a40b0000740b0000440b0000140b0000e40a0000b40a0000840a0000540a0000240a0000f4090000c409000094090000640900003409000004090000d4080000a4080000740800004408000014080000e4070000b4070000840700005407000024070000f4060000c406000094060000640600003406000004060000d4050000a4050000740500004405000014050000e4040000b4040000840400005404000024040000f4030000c403000094030000640300003403000004030000d4020000a4020000740200004402000014020000e4010000b4010000840100005401000024010000f4000000c40000009400000064000000340000000400000040eaffff18000000080000000102000038eaffff4000000001000000090000006669656c642d3132370000006ceaffff18000000080000000102000064eaffff4000000001000000090000006669656c642d31323600000098eaffff18000000080000000102000090eaffff4000000001000000090000006669656c642d313235000000c4eaffff180000000800000001020000bceaffff4000000001000000090000006669656c642d313234000000f0eaffff180000000800000001020000e8eaffff4000000001000000090000006669656c642d3132330000001cebffff18000000080000000102000014ebffff4000000001000000090000006669656c642d31323200000048ebffff18000000080000000102000040ebffff4000000001000000090000006669656c642d31323100000074ebffff1800000008000000010200006cebffff4000000001000000090000006669656c642d313230000000a0ebffff18000000080000000102000098ebffff4000000001000000090000006669656c642d313139000000ccebffff180000000800000001020000c4ebffff4000000001000000090000006669656c642d313138000000f8ebffff180000000800000001020000f0ebffff4000000001000000090000006669656c642d31313700000024ecffff1800000008000000010200001cecffff4000000001000000090000006669656c642d31313600000050ecffff18000000080000000102000048ecffff4000000001000000090000006669656c642d3131350000007cecffff18000000080000000102000074ecffff4000000001000000090000006669656c642d313134000000a8ecffff180000000800000001020000a0ecffff4000000001000000090000006669656c642d313133000000d4ecffff180000000800000001020000ccecffff4000000001000000090000006669656c642d31313200000000edffff180000000800000001020000f8ecffff4000000001000000090000006669656c642d3131310000002cedffff18000000080000000102000024edffff4000000001000000090000006669656c642d31313000000058edffff18000000080000000102000050edffff4000000001000000090000006669656c642d31303900000084edffff1800000008000000010200007cedffff4000000001000000090000006669656c642d313038000000b0edffff180000000800000001020000a8edffff4000000001000000090000006669656c642d313037000000dcedffff180000000800000001020000d4edffff4000000001000000090000006669656c642d31303600000008eeffff18000000080000000102000000eeffff4000000001000000090000006669656c642d31303500000034eeffff1800000008000000010200002ceeffff4000000001000000090000006669656c642d31303400000060eeffff18000000080000000102000058eeffff4000000001000000090000006669656c642d3130330000008ceeffff18000000080000000102000084eeffff4000000001000000090000006669656c642d313032000000b8eeffff180000000800000001020000b0eeffff4000000001000000090000006669656c642d313031000000e4eeffff180000000800000001020000dceeffff4000000001000000090000006669656c642d31303000000010efffff18000000080000000102000008efffff4000000001000000080000006669656c642d3939000000003cefffff18000000080000000102000034efffff4000000001000000080000006669656c642d39380000000068efffff18000000080000000102000060efffff4000000001000000080000006669656c642d39370000000094efffff1800000008000000010200008cefffff4000000001000000080000006669656c642d393600000000c0efffff180000000800000001020000b8efffff4000000001000000080000006669656c642d393500000000ecefffff180000000800000001020000e4efffff4000000001000000080000006669656c642d39340000000018f0ffff18000000080000000102000010f0ffff4000000001000000080000006669656c642d39330000000044f0ffff1800000008000000010200003cf0ffff4000000001000000080000006669656c642d39320000000070f0ffff18000000080000000102000068f0ffff4000000001000000080000006669656c642d3931000000009cf0ffff18000000080000000102000094f0ffff4000000001000000080000006669656c642d393000000000c8f0ffff180000000800000001020000c0f0ffff4000000001000000080000006669656c642d383900000000f4f0ffff180000000800000001020000ecf0ffff4000000001000000080000006669656c642d38380000000020f1ffff18000000080000000102000018f1ffff4000000001000000080000006669656c642d3837000000004cf1ffff18000000080000000102000044f1ffff4000000001000000080000006669656c642d38360000000078f1ffff18000000080000000102000070f1ffff4000000001000000080000006669656c642d383500000000a4f1ffff1800000008000000010200009cf1ffff4000000001000000080000006669656c642d383400000000d0f1ffff180000000800000001020000c8f1ffff4000000001000000080000006669656c642d383300000000fcf1ffff180000000800000001020000f4f1ffff4000000001000000080000006669656c642d38320000000028f2ffff18000000080000000102000020f2ffff4000000001000000080000006669656c642d38310000000054f2ffff1800000008000000010200004cf2ffff4000000001000000080000006669656c642d38300000000080f2ffff18000000080000000102000078f2ffff4000000001000000080000006669656c642d373900000000acf2ffff180000000800000001020000a4f2ffff4000000001000000080000006669656c642d373800000000d8f2ffff180000000800000001020000d0f2ffff4000000001000000080000006669656c642d37370000000004f3ffff180000000800000001020000fcf2ffff4000000001000000080000006669656c642d37360000000030f3ffff18000000080000000102000028f3ffff4000000001000000080000006669656c642d3735000000005cf3ffff18000000080000000102000054f3ffff4000000001000000080000006669656c642d37340000000088f3ffff18000000080000000102000080f3ffff4000000001000000080000006669656c642d373300000000b4f3ffff180000000800000001020000acf3ffff4000000001000000080000006669656c642d373200000000e0f3ffff180000000800000001020000d8f3ffff4000000001000000080000006669656c642d3731000000000cf4ffff18000000080000000102000004f4ffff4000000001000000080000006669656c642d37300000000038f4ffff18000000080000000102000030f4ffff4000000001000000080000006669656c642d36390000000064f4ffff1800000008000000010200005cf4ffff4000000001000000080000006669656c642d36380000000090f4ffff18000000080000000102000088f4ffff4000000001000000080000006669656c642d363700000000bcf4ffff180000000800000001020000b4f4ffff4000000001000000080000006669656c642d363600000000e8f4ffff180000000800000001020000e0f4ffff4000000001000000080000006669656c642d36350000000014f5ffff1800000008000000010200000cf5ffff4000000001000000080000006669656c642d36340000000040f5ffff18000000080000000102000038f5ffff4000000001000000080000006669656c642d3633000000006cf5ffff18000000080000000102000064f5ffff4000000001000000080000006669656c642d36320000000098f5ffff18000000080000000102000090f5ffff4000000001000000080000006669656c642d363100000000c4f5ffff180000000800000001020000bcf5ffff4000000001000000080000006669656c642d363000000000f0f5ffff180000000800000001020000e8f5ffff4000000001000000080000006669656c642d3539000000001cf6ffff18000000080000000102000014f6ffff4000000001000000080000006669656c642d35380000000048f6ffff18000000080000000102000040f6ffff4000000001000000080000006669656c642d35370000000074f6ffff1800000008000000010200006cf6ffff4000000001000000080000006669656c642d353600000000a0f6ffff18000000080000000102000098f6ffff4000000001000000080000006669656c642d353500000000ccf6ffff180000000800000001020000c4f6ffff4000000001000000080000006669656c642d353400000000f8f6ffff180000000800000001020000f0f6ffff4000000001000000080000006669656c642d35330000000024f7ffff1800000008000000010200001cf7ffff4000000001000000080000006669656c642d35320000000050f7ffff18000000080000000102000048f7ffff4000000001000000080000006669656c642d3531000000007cf7ffff18000000080000000102000074f7ffff4000000001000000080000006669656c642d353000000000a8f7ffff180000000800000001020000a0f7ffff4000000001000000080000006669656c642d343900000000d4f7ffff180000000800000001020000ccf7ffff4000000001000000080000006669656c642d34380000000000f8ffff180000000800000001020000f8f7ffff4000000001000000080000006669656c642d3437000000002cf8ffff18000000080000000102000024f8ffff4000000001000000080000006669656c642d34360000000058f8ffff18000000080000000102000050f8ffff4000000001000000080000006669656c642d34350000000084f8ffff1800000008000000010200007cf8ffff4000000001000000080000006669656c642d343400000000b0f8ffff180000000800000001020000a8f8ffff4000000001000000080000006669656c642d343300000000dcf8ffff180000000800000001020000d4f8ffff4000000001000000080000006669656c642d34320000000008f9ffff18000000080000000102000000f9ffff4000000001000000080000006669656c642d34310000000034f9ffff1800000008000000010200002cf9ffff4000000001000000080000006669656c642d34300000000060f9ffff18000000080000000102000058f9ffff4000000001000000080000006669656c642d3339000000008cf9ffff18000000080000000102000084f9ffff4000000001000000080000006669656c642d333800000000b8f9ffff180000000800000001020000b0f9ffff4000000001000000080000006669656c642d333700000000e4f9ffff180000000800000001020000dcf9ffff4000000001000000080000006669656c642d33360000000010faffff18000000080000000102000008faffff4000000001000000080000006669656c642d3335000000003cfaffff18000000080000000102000034faffff4000000001000000080000006669656c642d33340000000068faffff18000000080000000102000060faffff4000000001000000080000006669656c642d33330000000094faffff1800000008000000010200008cfaffff4000000001000000080000006669656c642d333200000000c0faffff180000000800000001020000b8faffff4000000001000000080000006669656c642d333100000000ecfaffff180000000800000001020000e4faffff4000000001000000080000006669656c642d33300000000018fbffff18000000080000000102000010fbffff4000000001000000080000006669656c642d32390000000044fbffff1800000008000000010200003cfbffff4000000001000000080000006669656c642d32380000000070fbffff18000000080000000102000068fbffff4000000001000000080000006669656c642d3237000000009cfbffff18000000080000000102000094fbffff4000000001000000080000006669656c642d323600000000c8fbffff180000000800000001020000c0fbffff4000000001000000080000006669656c642d323500000000f4fbffff180000000800000001020000ecfbffff4000000001000000080000006669656c642d32340000000020fcffff18000000080000000102000018fcffff4000000001000000080000006669656c642d3233000000004cfcffff18000000080000000102000044fcffff4000000001000000080000006669656c642d32320000000078fcffff18000000080000000102000070fcffff4000000001000000080000006669656c642d323100000000a4fcffff1800000008000000010200009cfcffff4000000001000000080000006669656c642d323000000000d0fcffff180000000800000001020000c8fcffff4000000001000000080000006669656c642d313900000000fcfcffff180000000800000001020000f4fcffff4000000001000000080000006669656c642d31380000000028fdffff18000000080000000102000020fdffff4000000001000000080000006669656c642d31370000000054fdffff1800000008000000010200004cfdffff4000000001000000080000006669656c642d31360000000080fdffff18000000080000000102000078fdffff4000000001000000080000006669656c642d313500000000acfdffff180000000800000001020000a4fdffff4000000001000000080000006669656c642d313400000000d8fdffff180000000800000001020000d0fdffff4000000001000000080000006669656c642d31330000000004feffff180000000800000001020000fcfdffff4000000001000000080000006669656c642d31320000000030feffff18000000080000000102000028feffff4000000001000000080000006669656c642d3131000000005cfeffff18000000080000000102000054feffff4000000001000000080000006669656c642d31300000000088feffff18000000080000000102000080feffff4000000001000000070000006669656c642d3900b0feffff180000000800000001020000a8feffff4000000001000000070000006669656c642d3800d8feffff180000000800000001020000d0feffff4000000001000000070000006669656c642d370000ffffff180000000800000001020000f8feffff4000000001000000070000006669656c642d360028ffffff18000000080000000102000020ffffff4000000001000000070000006669656c642d350050ffffff18000000080000000102000048ffffff4000000001000000070000006669656c642d340078ffffff18000000080000000102000070ffffff4000000001000000070000006669656c642d3300a0ffffff18000000080000000102000098ffffff4000000001000000070000006669656c642d3200c8ffffff180000000800000001020000c0ffffff4000000001000000070000006669656c642d3100f0ffffff2c00000014000000010200000c000e0004000c000d000800f4ffffff40000000010000000800090004000800070000006669656c642d300000000000ffffffff00000000"];
        let (a, b) = pair();
        let valid = batch("after-rejected-schemas", 5, 1);
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let mut client = runtime
            .block_on(SimulationTransportClient::connect(format!(
                "http://{}",
                b.bound_addr()
            )))
            .unwrap();
        for (index, hex) in fixtures.iter().enumerate() {
            assert!(hex.len().is_multiple_of(2));
            let payload = hex
                .as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
                .collect::<Vec<_>>();
            assert!(payload.len() < MAX_GRPC_MESSAGE_BYTES / 2);
            let wire = WireTelemetryBatchV2 {
                version: 2,
                batch_id: format!("invalid-schema-{index}"),
                run_id: "track49".into(),
                source_lp: 1,
                arrow_ipc_payload: payload,
                tick_start: valid.tick_start.ticks().to_be_bytes().to_vec(),
                tick_end: valid.tick_end.ticks().to_be_bytes().to_vec(),
            };
            let error = runtime
                .block_on(client.stream_telemetry_v2(tokio_stream::iter(vec![wire])))
                .unwrap_err();
            assert_eq!(error.code(), tonic::Code::InvalidArgument);
            assert_eq!(
                error.message(),
                "noncanonical telemetry schema profile before decode"
            );
            assert!(b.drain_versioned_telemetry().is_empty());
        }
        assert_eq!(
            a.send_versioned_telemetry(LpId(2), std::slice::from_ref(&valid))
                .unwrap()
                .accepted_batches,
            1
        );
    }
}
