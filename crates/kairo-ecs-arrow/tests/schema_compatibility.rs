use kairo_ecs_arrow::{
    ArrowType, EventLogBatch, EventLogRecord, EVENT_LOG_FIELDS, EVENT_LOG_STREAM, SCHEMA_VERSION,
};
use kairo_ecs_types::{DispatchedEvent, EventId, EventKind, SimTime};

#[test]
fn event_log_schema_keeps_v1_contract_order() {
    let fields: Vec<(&str, ArrowType, bool)> = EVENT_LOG_FIELDS
        .iter()
        .map(|field| (field.name, field.data_type, field.nullable))
        .collect();

    assert_eq!(EVENT_LOG_STREAM, "kairo_ecs.event_log.v1");
    assert_eq!(SCHEMA_VERSION, 1);
    assert_eq!(
        fields,
        vec![
            ("schema_version", ArrowType::UInt16, false),
            ("run_id", ArrowType::Utf8, false),
            ("event_id", ArrowType::FixedSizeBinary(12), false),
            ("entity_id", ArrowType::FixedSizeBinary(12), true),
            ("time_ticks", ArrowType::FixedSizeBinary(16), false),
            ("time_scale", ArrowType::Utf8, false),
            ("priority", ArrowType::Int32, false),
            ("sequence", ArrowType::UInt64, false),
            ("event_kind", ArrowType::Utf8, false),
            ("status", ArrowType::Utf8, false),
            ("payload_ref", ArrowType::Utf8, true),
        ]
    );
}

#[test]
fn event_log_v1_roundtrip_preserves_fixed_tick_ordering_fields() {
    let event = DispatchedEvent {
        id: EventId {
            index: 100,
            generation: 4,
        },
        at: SimTime::from_ticks(10),
        priority: 1,
        sequence: 2,
        entity: None,
        kind: EventKind::Custom(2),
    };
    let batch = EventLogBatch::new(vec![EventLogRecord::dispatched(
        "deterministic-ordering",
        event,
    )])
    .expect("valid batch");

    let decoded = EventLogBatch::from_smoke_bytes(&batch.to_smoke_bytes()).expect("roundtrip");

    assert_eq!(decoded, batch);
    assert_eq!(decoded.records()[0].time_ticks, 10);
    assert_eq!(decoded.records()[0].priority, 1);
    assert_eq!(decoded.records()[0].sequence, 2);
}
