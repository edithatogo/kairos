#![cfg(feature = "polars-native")]

use kairo_ecs_arrow::polars_interop::*;
use kairo_ecs_arrow::*;
use kairo_ecs_types::{EntityId, EventId};

fn sample_v2_record(time_ticks: u128, entity_id: Option<EntityId>) -> EventLogRecord {
    EventLogRecord {
        schema_version: SCHEMA_VERSION_V2,
        run_id: "test-run-phase1".to_string(),
        event_id: EventId::new(1, 1),
        entity_id,
        time_ticks,
        time_scale: "ticks".to_string(),
        priority: 0,
        sequence: 1,
        event_kind: "custom:test".to_string(),
        status: EventStatus::Dispatched,
        payload_ref: None,
    }
}

#[test]
#[should_panic(expected = "Track 70 Phase 1: records_to_dataframe not yet implemented")]
fn test_failing_table_schema_v2_polars_table_creation() {
    let records = vec![
        sample_v2_record(100, Some(EntityId::new(42, 1))),
        sample_v2_record(200, None),
    ];

    // In Task 1.1, this fails until Task 1.2 implements the true DataFrame builder
    let _df = records_to_dataframe(&records).expect("DataFrame construction should succeed");
}

#[test]
#[should_panic(expected = "Track 70 Phase 1: records_to_dataframe not yet implemented")]
fn test_failing_extreme_ticks_full_u128_range_and_limb_subtraction() {
    // Test extreme ticks spanning across high and low limbs:
    // u128::MAX = hi: u64::MAX, lo: u64::MAX
    // (1 << 64) = hi: 1, lo: 0
    let records = vec![
        sample_v2_record(0, None),
        sample_v2_record((1u128 << 64) - 1, None),
        sample_v2_record(1u128 << 64, None),
        sample_v2_record(u128::MAX, None),
    ];

    let _df = records_to_dataframe(&records).expect("Extreme tick records should build");
}

#[test]
fn test_failing_invalid_input_rejection_for_v1_records() {
    let mut v1_record = sample_v2_record(100, None);
    v1_record.schema_version = 1; // v1 records must be rejected by v2 DataFrame builder

    let res = records_to_dataframe(&[v1_record]);
    assert!(
        matches!(res, Err(ArrowError::UnsupportedSchemaVersion(1))),
        "Must reject non-v2 records with UnsupportedSchemaVersion"
    );
}

#[test]
#[should_panic(expected = "Track 70 Phase 1: write_parquet_zstd not yet implemented")]
fn test_failing_independent_parquet_reader_roundtrip() {
    let mut df = polars::prelude::DataFrame::empty();

    let mut buf = Vec::new();
    write_parquet_zstd(&mut df, &mut buf).unwrap();

    let _roundtrip_df = read_parquet(std::io::Cursor::new(buf)).unwrap();
}

#[test]
#[should_panic(expected = "Track 70 Phase 1: write_ipc_stream not yet implemented")]
fn test_failing_independent_ipc_reader_roundtrip() {
    let mut df = polars::prelude::DataFrame::empty();

    let mut buf = Vec::new();
    write_ipc_stream(&mut df, &mut buf).unwrap();

    let _roundtrip_df = read_ipc_stream(std::io::Cursor::new(buf)).unwrap();
}
