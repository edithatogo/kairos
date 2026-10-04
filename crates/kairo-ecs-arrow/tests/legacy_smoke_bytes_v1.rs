use kairo_ecs_arrow::{EventLogBatch, EventLogRecord};
use kairo_ecs_types::{DispatchedEvent, EntityId, EventId, EventKind, SimTime};

// Independently frozen from the pre-feature baseline (4c065e7): 296 bytes,
// SHA-256 3928adcb2a84bfb0521c267cc834534693ee2acaa8ab7a080523dc20491163ac.
// This is Kairos' legacy custom smoke representation, not Arrow IPC or Parquet.
const LEGACY_SMOKE_V1: &[u8] = b"stream=kairo_ecs.event_log.v1;schema_version=1\nschema_version\trun_id\tevent_id_hex\tentity_id_hex\ttime_ticks_le_hex\ttime_scale\tpriority\tsequence\tevent_kind\tstatus\tpayload_ref\n1\trun-1\t010000000000000002000000\t050000000000000006000000\t0a000000000000000000000000000000\tticks\t-3\t4\tcustom:7\tdispatched\t\n";

fn legacy_sample() -> EventLogBatch {
    let event = DispatchedEvent::new(
        EventId::new(1, 2),
        SimTime::from_ticks(10),
        -3,
        4,
        Some(EntityId::new(5, 6)),
        EventKind::custom(7),
    );
    EventLogBatch::new(vec![EventLogRecord::dispatched("run-1", event)])
        .expect("valid legacy event log")
}

#[test]
fn legacy_smoke_bytes_match_frozen_custom_format_and_roundtrip() {
    assert_eq!(LEGACY_SMOKE_V1.len(), 296);

    let emitted = legacy_sample().to_smoke_bytes();
    assert_eq!(emitted, LEGACY_SMOKE_V1);
    assert!(LEGACY_SMOKE_V1.starts_with(b"stream=kairo_ecs.event_log.v1;schema_version=1\n"));
    assert!(!LEGACY_SMOKE_V1.starts_with(b"ARROW1"));
    assert!(!LEGACY_SMOKE_V1.starts_with(b"PAR1"));

    let decoded = EventLogBatch::from_smoke_bytes(LEGACY_SMOKE_V1)
        .expect("frozen legacy custom bytes decode");
    assert_eq!(decoded.to_smoke_bytes(), LEGACY_SMOKE_V1);
}
