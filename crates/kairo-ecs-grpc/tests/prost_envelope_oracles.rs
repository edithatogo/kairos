use kairo_ecs_grpc::{
    AncestryFrame, GrpcSocketTransport, GrpcTransportConfig, KairoOptimisticEnvelopeV1,
    OptimisticAuthorityMode, RejectionCode, WireAncestryFrame, ENVELOPE_PROTOCOL_VERSION_V1,
    ENVELOPE_PROTOCOL_VERSION_V2, ENVELOPE_SCHEMA_VERSION_LEGACY_V1,
    ENVELOPE_SCHEMA_VERSION_PROTOBUF_V2, GRPC_PROTOCOL_ID, MAX_ANCESTRY_DEPTH, MAX_ANCESTRY_NODES,
    MAX_ENCODED_ENVELOPE_BYTES, MAX_PAYLOAD_BYTES,
};
use kairo_ecs_pdes::{
    LogicalEventId, LpId, OptimisticAuthority, OptimisticEventOrderKey, OptimisticMessage,
    OptimisticMessageKind, RemoteEvent,
};
use kairo_ecs_types::SimTime;
use prost::Message;
use std::time::Duration;

fn sample_valid_envelope() -> KairoOptimisticEnvelopeV1 {
    KairoOptimisticEnvelopeV1 {
        envelope_schema_version: ENVELOPE_SCHEMA_VERSION_PROTOBUF_V2,
        protocol_id: GRPC_PROTOCOL_ID.to_string(),
        protocol_version: ENVELOPE_PROTOCOL_VERSION_V2,
        authority_mode: OptimisticAuthorityMode::Scoped,
        simulation_namespace: 0xfeed_face_cafe_beef_0123_4567_89ab_cdef_u128,
        ownership_epoch: 101,
        emission_epoch: 202,
        source_lp: 3,
        dest_lp: 7,
        tick: (u64::MAX as u128) + 500, // full 128-bit tick exceeding u64
        ancestry_frames: vec![AncestryFrame::Root {
            root_source_lp: 3,
            root_sequence: 42,
        }],
        incarnation: 5,
        kind: kairo_ecs_grpc::WireMessageKind::Positive,
        event_payload: b"prost-wire-payload-v1".to_vec(),
    }
}

#[test]
fn oracle_prost_envelope_roundtrip_preserves_full_identity_and_tick_range() {
    let env = sample_valid_envelope();
    env.validate(GRPC_PROTOCOL_ID)
        .expect("envelope must validate");

    let encoded = env.encode().expect("envelope must encode to protobuf");
    assert!(
        encoded.len() <= MAX_ENCODED_ENVELOPE_BYTES,
        "encoded len must be bounded"
    );

    let decoded = KairoOptimisticEnvelopeV1::decode(&encoded).expect("must decode valid protobuf");
    assert_eq!(env, decoded, "decoded envelope must match original");
    assert_eq!(decoded.tick, (u64::MAX as u128) + 500);
    assert_eq!(
        decoded.simulation_namespace,
        0xfeed_face_cafe_beef_0123_4567_89ab_cdef_u128
    );
}

#[test]
fn oracle_prost_envelope_roundtrip_with_deep_ancestry_chain() {
    let mut frames = Vec::with_capacity(MAX_ANCESTRY_NODES);
    frames.push(AncestryFrame::Root {
        root_source_lp: 1,
        root_sequence: 1,
    });

    for i in 1..MAX_ANCESTRY_NODES {
        frames.push(AncestryFrame::Output {
            parent_tick: (i * 10) as u128,
            parent_source_lp: 1,
            ordinal: i as u32,
        });
    }

    let env = KairoOptimisticEnvelopeV1 {
        envelope_schema_version: ENVELOPE_SCHEMA_VERSION_PROTOBUF_V2,
        protocol_id: GRPC_PROTOCOL_ID.to_string(),
        protocol_version: ENVELOPE_PROTOCOL_VERSION_V2,
        authority_mode: OptimisticAuthorityMode::Scoped,
        simulation_namespace: 0x42,
        ownership_epoch: 1,
        emission_epoch: 1,
        source_lp: 1,
        dest_lp: 2,
        tick: ((MAX_ANCESTRY_NODES * 10) + 10) as u128,
        ancestry_frames: frames,
        incarnation: 0,
        kind: kairo_ecs_grpc::WireMessageKind::Positive,
        event_payload: b"deep-chain-data".to_vec(),
    };

    env.validate(GRPC_PROTOCOL_ID)
        .expect("chain up to max allowed depth must validate");

    let encoded = env.encode().expect("must encode deep chain");
    let decoded = KairoOptimisticEnvelopeV1::decode(&encoded).expect("must decode deep chain");
    assert_eq!(env, decoded);
    assert_eq!(decoded.ancestry_frames.len(), MAX_ANCESTRY_NODES);
}

#[test]
fn oracle_prost_envelope_rejects_ancestry_depth_exceeded() {
    // Case 1: 129 output frames without root frame triggers AncestryDepthExceeded
    let mut depth_exceeded_frames = Vec::with_capacity(MAX_ANCESTRY_DEPTH + 1);
    for i in 1..=MAX_ANCESTRY_DEPTH + 1 {
        depth_exceeded_frames.push(AncestryFrame::Output {
            parent_tick: (i * 10) as u128,
            parent_source_lp: 1,
            ordinal: i as u32,
        });
    }

    let env_depth = KairoOptimisticEnvelopeV1 {
        envelope_schema_version: ENVELOPE_SCHEMA_VERSION_PROTOBUF_V2,
        protocol_id: GRPC_PROTOCOL_ID.to_string(),
        protocol_version: ENVELOPE_PROTOCOL_VERSION_V2,
        authority_mode: OptimisticAuthorityMode::Scoped,
        simulation_namespace: 0x42,
        ownership_epoch: 1,
        emission_epoch: 1,
        source_lp: 1,
        dest_lp: 2,
        tick: 2000,
        ancestry_frames: depth_exceeded_frames,
        incarnation: 0,
        kind: kairo_ecs_grpc::WireMessageKind::Positive,
        event_payload: vec![],
    };

    assert_eq!(
        env_depth.validate(GRPC_PROTOCOL_ID),
        Err(RejectionCode::AncestryDepthExceeded {
            depth: MAX_ANCESTRY_DEPTH + 1,
            limit: MAX_ANCESTRY_DEPTH,
        })
    );

    // Case 2: Exceeding MAX_ANCESTRY_NODES (130 frames) triggers EnvelopeTooLarge
    let mut node_count_exceeded = Vec::with_capacity(MAX_ANCESTRY_NODES + 1);
    node_count_exceeded.push(AncestryFrame::Root {
        root_source_lp: 1,
        root_sequence: 1,
    });
    for i in 1..=MAX_ANCESTRY_NODES {
        node_count_exceeded.push(AncestryFrame::Output {
            parent_tick: (i * 10) as u128,
            parent_source_lp: 1,
            ordinal: i as u32,
        });
    }

    let env_nodes = KairoOptimisticEnvelopeV1 {
        envelope_schema_version: ENVELOPE_SCHEMA_VERSION_PROTOBUF_V2,
        protocol_id: GRPC_PROTOCOL_ID.to_string(),
        protocol_version: ENVELOPE_PROTOCOL_VERSION_V2,
        authority_mode: OptimisticAuthorityMode::Scoped,
        simulation_namespace: 0x42,
        ownership_epoch: 1,
        emission_epoch: 1,
        source_lp: 1,
        dest_lp: 2,
        tick: 2000,
        ancestry_frames: node_count_exceeded,
        incarnation: 0,
        kind: kairo_ecs_grpc::WireMessageKind::Positive,
        event_payload: vec![],
    };

    assert_eq!(
        env_nodes.validate(GRPC_PROTOCOL_ID),
        Err(RejectionCode::EnvelopeTooLarge(
            "ancestry_frames",
            MAX_ANCESTRY_NODES + 1,
            MAX_ANCESTRY_NODES,
        ))
    );

    assert_eq!(
        env_nodes.encode(),
        Err(RejectionCode::EnvelopeTooLarge(
            "ancestry_frames",
            MAX_ANCESTRY_NODES + 1,
            MAX_ANCESTRY_NODES,
        ))
    );
}

#[test]
fn oracle_prost_envelope_rejects_oversized_payload() {
    let mut env = sample_valid_envelope();
    env.event_payload = vec![0xaa; MAX_PAYLOAD_BYTES + 1];

    assert!(matches!(
        env.validate(GRPC_PROTOCOL_ID),
        Err(RejectionCode::EnvelopeTooLarge("event_payload", _, _))
    ));

    assert!(matches!(
        env.encode(),
        Err(RejectionCode::EnvelopeTooLarge("event_payload", _, _))
    ));
}

#[test]
fn oracle_prost_envelope_rejects_oversized_total_envelope() {
    let oversized_bytes = vec![0u8; MAX_ENCODED_ENVELOPE_BYTES + 1];
    let result = KairoOptimisticEnvelopeV1::decode(&oversized_bytes);
    assert!(matches!(
        result,
        Err(RejectionCode::EnvelopeTooLarge("total_envelope", _, _))
    ));
}

#[test]
fn oracle_prost_envelope_rejects_truncated_or_malformed_protobuf_bytes() {
    let env = sample_valid_envelope();
    let encoded = env.encode().expect("encode ok");

    // Truncated buffer
    let truncated = &encoded[..encoded.len() / 2];
    assert!(matches!(
        KairoOptimisticEnvelopeV1::decode(truncated),
        Err(RejectionCode::MalformedFraming(_))
    ));

    // Empty buffer
    assert!(matches!(
        KairoOptimisticEnvelopeV1::decode(&[]),
        Err(RejectionCode::MalformedFraming(_))
    ));

    // Corrupted tag
    let mut corrupted = encoded.clone();
    corrupted[0] = 0xff;
    corrupted[1] = 0xff;
    assert!(matches!(
        KairoOptimisticEnvelopeV1::decode(&corrupted),
        Err(RejectionCode::MalformedFraming(_))
    ));
}

#[test]
fn oracle_prost_envelope_rejects_unsupported_schema_version() {
    let mut wire = sample_valid_envelope()
        .to_wire()
        .expect("to_wire should succeed");
    wire.envelope_schema_version = 3; // version 3 is unsupported (v1 legacy and v2 protobuf are supported)

    let mut buf = Vec::new();
    wire.encode(&mut buf).expect("wire encode ok");

    assert_eq!(
        KairoOptimisticEnvelopeV1::decode(&buf),
        Err(RejectionCode::UnsupportedEnvelopeVersion(3))
    );
}

#[test]
fn oracle_prost_envelope_rejects_invalid_authority_mode() {
    let mut wire = sample_valid_envelope()
        .to_wire()
        .expect("to_wire should succeed");
    wire.authority_mode = 99; // invalid authority mode

    let mut buf = Vec::new();
    wire.encode(&mut buf).expect("wire encode ok");

    assert_eq!(
        KairoOptimisticEnvelopeV1::decode(&buf),
        Err(RejectionCode::AuthorityModeMismatch)
    );
}

#[test]
fn oracle_prost_envelope_rejects_invalid_message_kind() {
    let mut wire = sample_valid_envelope()
        .to_wire()
        .expect("to_wire should succeed");
    wire.kind = 99; // invalid kind

    let mut buf = Vec::new();
    wire.encode(&mut buf).expect("wire encode ok");

    assert!(matches!(
        KairoOptimisticEnvelopeV1::decode(&buf),
        Err(RejectionCode::MalformedFraming(_))
    ));
}

#[test]
fn oracle_prost_envelope_rejects_invalid_timestamp_byte_length() {
    let mut wire = sample_valid_envelope()
        .to_wire()
        .expect("to_wire should succeed");
    wire.tick = vec![0u8; 8]; // 8 bytes instead of 16 for u128

    let mut buf = Vec::new();
    wire.encode(&mut buf).expect("wire encode ok");

    assert!(matches!(
        KairoOptimisticEnvelopeV1::decode(&buf),
        Err(RejectionCode::MalformedFraming(_))
    ));
}

#[test]
fn oracle_prost_envelope_rejects_missing_oneof_frame() {
    let mut wire = sample_valid_envelope()
        .to_wire()
        .expect("to_wire should succeed");
    wire.ancestry_frames = vec![WireAncestryFrame { frame: None }];

    let mut buf = Vec::new();
    wire.encode(&mut buf).expect("wire encode ok");

    assert!(matches!(
        KairoOptimisticEnvelopeV1::decode(&buf),
        Err(RejectionCode::MalformedFraming(_))
    ));
}

#[test]
fn oracle_prost_optimistic_message_full_roundtrip() {
    let root_id = LogicalEventId::root(LpId(5), 100);
    let k1 = OptimisticEventOrderKey::try_from_parts(SimTime::from_ticks(10), LpId(5), root_id)
        .expect("order key");
    let child_id = LogicalEventId::child(&k1, 0).expect("child logical id");

    let event = RemoteEvent {
        source_lp: LpId(5),
        dest_lp: LpId(6),
        tick: SimTime::from_ticks(20),
        event_payload: b"order-child-data".to_vec(),
    };

    let authority = OptimisticAuthority::Scoped {
        simulation_namespace: 0x7777,
        ownership_epoch: 12,
    };

    let msg = OptimisticMessage::try_from_authority_parts(
        event,
        child_id,
        authority,
        3,
        OptimisticMessageKind::Positive,
    )
    .expect("construct native message");

    let env =
        KairoOptimisticEnvelopeV1::from_optimistic_message(&msg, GRPC_PROTOCOL_ID, LpId(6), 12)
            .expect("convert to envelope");

    let wire_bytes = env.encode().expect("encode to protobuf bytes");
    let recovered_env =
        KairoOptimisticEnvelopeV1::decode(&wire_bytes).expect("decode from protobuf bytes");
    assert_eq!(env, recovered_env);

    let recovered_msg = recovered_env
        .to_optimistic_message(GRPC_PROTOCOL_ID)
        .expect("convert back to native message");

    assert_eq!(recovered_msg.event().source_lp, LpId(5));
    assert_eq!(recovered_msg.event().dest_lp, LpId(6));
    assert_eq!(recovered_msg.event().tick.ticks(), 20);
    assert_eq!(recovered_msg.incarnation(), 3);
    assert_eq!(recovered_msg.kind(), OptimisticMessageKind::Positive);
}

#[test]
fn oracle_legacy_v1_and_protobuf_v2_dual_compatibility() {
    // 1. Construct and encode legacy v1 envelope
    let legacy_env = KairoOptimisticEnvelopeV1 {
        envelope_schema_version: ENVELOPE_SCHEMA_VERSION_LEGACY_V1,
        protocol_id: GRPC_PROTOCOL_ID.to_string(),
        protocol_version: ENVELOPE_PROTOCOL_VERSION_V1,
        authority_mode: OptimisticAuthorityMode::Scoped,
        simulation_namespace: 0xfeed_cafe_dead_beef_u128,
        ownership_epoch: 11,
        emission_epoch: 22,
        source_lp: 4,
        dest_lp: 8,
        tick: 1_000_000_u128,
        ancestry_frames: vec![AncestryFrame::Root {
            root_source_lp: 4,
            root_sequence: 99,
        }],
        incarnation: 1,
        kind: kairo_ecs_grpc::WireMessageKind::Positive,
        event_payload: b"legacy-v1-dual-test".to_vec(),
    };
    legacy_env
        .validate(GRPC_PROTOCOL_ID)
        .expect("legacy v1 must validate");

    let legacy_bytes = legacy_env
        .encode_legacy_v1()
        .expect("must encode legacy v1");
    // Verify legacy wire format begins with big-endian u32 1 = [0, 0, 0, 1]
    assert_eq!(&legacy_bytes[0..4], &[0, 0, 0, 1]);

    // Decode through unified decode entrypoint
    let decoded_legacy =
        KairoOptimisticEnvelopeV1::decode(&legacy_bytes).expect("unified decode handles legacy v1");
    assert_eq!(legacy_env, decoded_legacy);
    assert_eq!(
        decoded_legacy.envelope_schema_version,
        ENVELOPE_SCHEMA_VERSION_LEGACY_V1
    );

    // 2. Construct and encode protobuf v2 envelope
    let proto_env = sample_valid_envelope();
    let proto_bytes = proto_env.encode().expect("must encode protobuf v2");
    // Protobuf wire format field 1 (tag 1, varint wire type = (1 << 3) | 0 = 0x08)
    assert_ne!(&proto_bytes[0..4], &[0, 0, 0, 1]);

    // Decode through unified decode entrypoint
    let decoded_proto =
        KairoOptimisticEnvelopeV1::decode(&proto_bytes).expect("unified decode handles proto v2");
    assert_eq!(proto_env, decoded_proto);
    assert_eq!(
        decoded_proto.envelope_schema_version,
        ENVELOPE_SCHEMA_VERSION_PROTOBUF_V2
    );
}

#[test]
fn oracle_socket_receiver_records_envelope_decode_failures() {
    use std::io::Write;
    use std::net::TcpStream;

    let transport = GrpcSocketTransport::bind(
        LpId(99),
        "127.0.0.1:0".parse().unwrap(),
        vec![],
        GrpcTransportConfig::default(),
    )
    .expect("bind socket transport");

    assert_eq!(transport.envelope_decode_failure_count(), 0);
    assert!(transport.envelope_decode_failures().is_empty());

    let addr = transport.bound_addr();

    // Connect raw TCP stream and send an invalid MSG_TYPE_OPTIMISTIC (3) frame
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(500))
        .expect("connect to bound transport");

    let malformed_payload = b"not-a-valid-envelope";
    let len = (malformed_payload.len() + 1) as u32;
    // Frame header: 4-byte big-endian len, 1-byte msg_type = 3 (MSG_TYPE_OPTIMISTIC)
    stream
        .write_all(&len.to_be_bytes())
        .expect("write frame len");
    stream.write_all(&[3u8]).expect("write msg_type");
    stream.write_all(malformed_payload).expect("write payload");
    stream.flush().expect("flush stream");

    // Allow socket thread to process the inbound frame
    let mut recorded = false;
    for _ in 0..50 {
        if transport.envelope_decode_failure_count() > 0 {
            recorded = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    assert!(
        recorded,
        "socket transport must record decode failure rather than silently discarding"
    );
    let failures = transport.envelope_decode_failures();
    assert_eq!(failures.len(), 1);
    assert!(
        failures[0].contains("Optimistic envelope decode error"),
        "failure message should capture decode error: {}",
        failures[0]
    );

    // Send 70 additional malformed frames to verify bounded retention and monotonic counter
    for i in 0..70 {
        let payload = format!("malformed-frame-flood-{i}");
        let payload_bytes = payload.as_bytes();
        let frame_len = (payload_bytes.len() + 1) as u32;
        stream
            .write_all(&frame_len.to_be_bytes())
            .expect("write frame len");
        stream.write_all(&[3u8]).expect("write msg_type");
        stream.write_all(payload_bytes).expect("write payload");
        stream.flush().expect("flush stream");
    }

    // Wait for frames to be processed
    for _ in 0..50 {
        if transport.envelope_decode_failure_count() >= 71 {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    let total_count = transport.envelope_decode_failure_count();
    assert!(
        total_count >= 71,
        "total failure count must track all rejections monotonically: {total_count}"
    );

    let retained = transport.envelope_decode_failures();
    assert_eq!(
        retained.len(),
        kairo_ecs_grpc::MAX_RETAINED_DECODE_FAILURES,
        "retained errors must be capped at MAX_RETAINED_DECODE_FAILURES (64)"
    );

    // Drain buffer and ensure retention resets while monotonic counter is preserved
    let drained = transport.drain_envelope_decode_failures();
    assert_eq!(drained.len(), kairo_ecs_grpc::MAX_RETAINED_DECODE_FAILURES);
    assert_eq!(transport.envelope_decode_failures().len(), 0);
    assert_eq!(transport.envelope_decode_failure_count(), total_count);
}

#[derive(serde::Deserialize)]
struct ConformanceFixture {
    cases: Vec<ConformanceCase>,
}

#[derive(serde::Deserialize)]
struct ConformanceCase {
    case_id: String,
    envelope_schema_version: u32,
    protocol_id: String,
    protocol_version: u32,
    authority_mode: u32,
    simulation_namespace: u128,
    ownership_epoch: u64,
    emission_epoch: u64,
    source_lp: u32,
    dest_lp: u32,
    tick: u128,
    ancestry_frames: Vec<ConformanceAncestryFrame>,
    incarnation: u64,
    kind: u32,
    event_payload_hex: String,
    independent_protobuf_wire_hex: String,
}

#[derive(serde::Deserialize)]
struct ConformanceAncestryFrame {
    #[serde(rename = "type")]
    frame_type: String,
    #[serde(default)]
    root_source_lp: u32,
    #[serde(default)]
    root_sequence: u64,
    #[serde(default)]
    parent_tick: u128,
    #[serde(default)]
    parent_source_lp: u32,
    #[serde(default)]
    ordinal: u32,
}

fn decode_hex_bytes(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("valid hex byte"))
        .collect()
}

#[test]
fn oracle_independent_protobuf_peer_conformance() {
    let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../conformance/fixtures/optimistic_envelope_v2_conformance.json");
    let fixture_content =
        std::fs::read_to_string(&fixture_path).expect("must read conformance fixture");
    let fixture: ConformanceFixture =
        serde_json::from_str(&fixture_content).expect("parse conformance fixture JSON");

    assert!(
        !fixture.cases.is_empty(),
        "fixture must contain conformance cases"
    );

    for case in fixture.cases {
        let peer_wire_bytes = decode_hex_bytes(&case.independent_protobuf_wire_hex);
        let expected_payload = decode_hex_bytes(&case.event_payload_hex);

        // 1. Decode peer-generated protobuf wire bytes through production decoder
        let decoded = KairoOptimisticEnvelopeV1::decode(&peer_wire_bytes)
            .unwrap_or_else(|e| panic!("failed to decode case {}: {e:?}", case.case_id));

        // 2. Validate semantic field agreement between published schema and Rust domain
        assert_eq!(
            decoded.envelope_schema_version, case.envelope_schema_version,
            "schema version in case {}",
            case.case_id
        );
        assert_eq!(
            decoded.protocol_id, case.protocol_id,
            "protocol_id in case {}",
            case.case_id
        );
        assert_eq!(
            decoded.protocol_version, case.protocol_version as u16,
            "protocol_version in case {}",
            case.case_id
        );
        match case.authority_mode {
            0 => assert_eq!(
                decoded.authority_mode,
                OptimisticAuthorityMode::LocalPreview
            ),
            1 => assert_eq!(decoded.authority_mode, OptimisticAuthorityMode::Scoped),
            _ => panic!("unexpected authority_mode"),
        }
        assert_eq!(
            decoded.simulation_namespace, case.simulation_namespace,
            "namespace in case {}",
            case.case_id
        );
        assert_eq!(
            decoded.ownership_epoch, case.ownership_epoch,
            "ownership_epoch in case {}",
            case.case_id
        );
        assert_eq!(
            decoded.emission_epoch, case.emission_epoch,
            "emission_epoch in case {}",
            case.case_id
        );
        assert_eq!(
            decoded.source_lp, case.source_lp,
            "source_lp in case {}",
            case.case_id
        );
        assert_eq!(
            decoded.dest_lp, case.dest_lp,
            "dest_lp in case {}",
            case.case_id
        );
        assert_eq!(decoded.tick, case.tick, "tick in case {}", case.case_id);
        assert_eq!(
            decoded.incarnation, case.incarnation,
            "incarnation in case {}",
            case.case_id
        );
        assert_eq!(
            decoded.event_payload, expected_payload,
            "event_payload in case {}",
            case.case_id
        );

        match case.kind {
            0 => assert_eq!(decoded.kind, kairo_ecs_grpc::WireMessageKind::Positive),
            1 => assert_eq!(decoded.kind, kairo_ecs_grpc::WireMessageKind::Anti),
            _ => panic!("unexpected kind in fixture"),
        }

        assert_eq!(
            decoded.ancestry_frames.len(),
            case.ancestry_frames.len(),
            "ancestry length in case {}",
            case.case_id
        );

        for (actual_frame, expected_frame) in decoded
            .ancestry_frames
            .iter()
            .zip(case.ancestry_frames.iter())
        {
            match (actual_frame, expected_frame.frame_type.as_str()) {
                (
                    AncestryFrame::Root {
                        root_source_lp,
                        root_sequence,
                    },
                    "root",
                ) => {
                    assert_eq!(*root_source_lp, expected_frame.root_source_lp);
                    assert_eq!(*root_sequence, expected_frame.root_sequence);
                }
                (
                    AncestryFrame::Output {
                        parent_tick,
                        parent_source_lp,
                        ordinal,
                    },
                    "output",
                ) => {
                    assert_eq!(*parent_tick, expected_frame.parent_tick);
                    assert_eq!(*parent_source_lp, expected_frame.parent_source_lp);
                    assert_eq!(*ordinal, expected_frame.ordinal);
                }
                _ => panic!("ancestry frame variant mismatch in case {}", case.case_id),
            }
        }

        // 3. Bidirectional roundtrip: re-encode via production encoder and verify byte-for-byte canonical equality
        let encoded_bytes = decoded
            .encode()
            .unwrap_or_else(|e| panic!("failed to encode case {}: {e:?}", case.case_id));

        assert_eq!(
            encoded_bytes, peer_wire_bytes,
            "production protobuf wire encoding must match independent peer wire bytes byte-for-byte in case {}",
            case.case_id
        );
    }
}
