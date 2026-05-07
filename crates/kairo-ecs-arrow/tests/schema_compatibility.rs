use kairo_ecs_arrow::{EventLogBatch, EventLogRecord, EVENT_LOG_FIELDS, EVENT_LOG_STREAM};
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
