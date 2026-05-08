use kairo_ecs_arrow::{
    event_log_schema_fingerprint, EventLogBatch, EventLogRecord, EVENT_LOG_FIELDS,
    EVENT_LOG_SCHEMA_MAJOR, EVENT_LOG_STREAM, SCHEMA_VERSION,
};
use kairo_ecs_types::{DispatchedEvent, EntityId, EventId, EventKind, SimTime};

#[test]
fn event_log_schema_is_stable_for_binding_consumers() {
    let fields: Vec<_> = EVENT_LOG_FIELDS
        .iter()
        .map(|field| (field.name, field.data_type, field.nullable))
        .collect();

    assert_eq!(EVENT_LOG_STREAM, "kairo_ecs.event_log.v1");
    assert_eq!(
        fields,
        vec![
            ("schema_version", "UInt16", false),
            ("run_id", "Utf8", false),
            ("event_id", "FixedSizeBinary(12)", false),
            ("entity_id", "FixedSizeBinary(12)", true),
            ("time_ticks", "FixedSizeBinary(16)", false),
            ("time_scale", "Utf8", false),
            ("priority", "Int32", false),
            ("sequence", "UInt64", false),
            ("event_kind", "Utf8", false),
            ("status", "Utf8", false),
            ("payload_ref", "Utf8", true),
        ]
    );
}

#[test]
fn event_log_schema_fingerprint_is_stable() {
    assert_eq!(EVENT_LOG_SCHEMA_MAJOR, 1);
    assert_eq!(SCHEMA_VERSION, 1);
    assert_eq!(
        event_log_schema_fingerprint(),
        "kairo_ecs.event_log.v1;major=1;fields=schema_version:UInt16:required|run_id:Utf8:required|event_id:FixedSizeBinary(12):required|entity_id:FixedSizeBinary(12):nullable|time_ticks:FixedSizeBinary(16):required|time_scale:Utf8:required|priority:Int32:required|sequence:UInt64:required|event_kind:Utf8:required|status:Utf8:required|payload_ref:Utf8:nullable"
    );
}

#[test]
fn checked_in_schema_json_matches_runtime_contract() {
    let schema_json = include_str!("../../../schemas/arrow/event_log_v1.schema.json");

    assert_ordered(
        schema_json,
        &[
            r#""stream": "kairo_ecs.event_log.v1""#,
            r#""schema_version": 1"#,
            r#""major": 1"#,
            r#""name": "schema_version", "type": "UInt16", "nullable": false"#,
            r#""name": "run_id", "type": "Utf8", "nullable": false"#,
            r#""name": "event_id", "type": "FixedSizeBinary(12)", "nullable": false"#,
            r#""name": "entity_id", "type": "FixedSizeBinary(12)", "nullable": true"#,
            r#""name": "time_ticks", "type": "FixedSizeBinary(16)", "nullable": false"#,
            r#""name": "time_scale", "type": "Utf8", "nullable": false"#,
            r#""name": "priority", "type": "Int32", "nullable": false"#,
            r#""name": "sequence", "type": "UInt64", "nullable": false"#,
            r#""name": "event_kind", "type": "Utf8", "nullable": false"#,
            r#""name": "status", "type": "Utf8", "nullable": false"#,
            r#""name": "payload_ref", "type": "Utf8", "nullable": true"#,
        ],
    );
}

#[test]
fn event_log_smoke_roundtrip_preserves_ordering_fields() {
    let event = DispatchedEvent::new(
        EventId::new(11, 2),
        SimTime::from_ticks(u128::MAX - 1),
        -10,
        42,
        Some(EntityId::new(99, 7)),
        EventKind::custom(123),
    );
    let batch = EventLogBatch::new(vec![EventLogRecord::dispatched("compat-run", event)])
        .expect("valid event-log batch");

    let decoded =
        EventLogBatch::from_smoke_bytes(&batch.to_smoke_bytes()).expect("smoke bytes decode");

    assert_eq!(decoded, batch);
    assert_eq!(decoded.records()[0].time_ticks, u128::MAX - 1);
    assert_eq!(decoded.records()[0].priority, -10);
    assert_eq!(decoded.records()[0].sequence, 42);
}

fn assert_ordered(haystack: &str, needles: &[&str]) {
    let mut cursor = 0;
    for needle in needles {
        let Some(position) = haystack[cursor..].find(needle) else {
            panic!("schema JSON did not contain expected fragment after byte {cursor}: {needle}");
        };
        cursor += position + needle.len();
    }
}
