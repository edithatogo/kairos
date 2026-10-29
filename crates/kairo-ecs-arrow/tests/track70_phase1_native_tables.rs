#![cfg(feature = "polars-native")]

use kairo_ecs_arrow::polars_interop::*;
use kairo_ecs_arrow::*;
use kairo_ecs_types::{EntityId, EventId};
use polars::prelude::*;
use std::io::Cursor;

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
fn test_table_schema_v2_polars_table_creation_and_reconstruction() {
    let records = vec![
        sample_v2_record(100, Some(EntityId::new(42, 1))),
        sample_v2_record(200, None),
    ];

    let df = records_to_dataframe(&records).expect("DataFrame construction should succeed");
    assert_eq!(df.height(), 2);
    assert_eq!(df.width(), 14);

    // Verify each column name and DataType
    assert_eq!(
        df.column("schema_version").unwrap().dtype(),
        &DataType::UInt16
    );
    assert_eq!(df.column("run_id").unwrap().dtype(), &DataType::String);
    assert_eq!(
        df.column("event_id_index").unwrap().dtype(),
        &DataType::UInt64
    );
    assert_eq!(
        df.column("event_id_gen").unwrap().dtype(),
        &DataType::UInt32
    );
    assert_eq!(
        df.column("entity_id_index").unwrap().dtype(),
        &DataType::UInt64
    );
    assert_eq!(
        df.column("entity_id_gen").unwrap().dtype(),
        &DataType::UInt32
    );
    assert_eq!(
        df.column("time_ticks_hi").unwrap().dtype(),
        &DataType::UInt64
    );
    assert_eq!(
        df.column("time_ticks_lo").unwrap().dtype(),
        &DataType::UInt64
    );
    assert_eq!(df.column("time_scale").unwrap().dtype(), &DataType::String);
    assert_eq!(df.column("priority").unwrap().dtype(), &DataType::Int32);
    assert_eq!(df.column("sequence").unwrap().dtype(), &DataType::UInt64);
    assert_eq!(df.column("event_kind").unwrap().dtype(), &DataType::String);
    assert_eq!(df.column("status").unwrap().dtype(), &DataType::String);
    assert_eq!(df.column("payload_ref").unwrap().dtype(), &DataType::String);

    // Verify nullability / null counts
    assert_eq!(df.column("entity_id_index").unwrap().null_count(), 1);
    assert_eq!(df.column("entity_id_gen").unwrap().null_count(), 1);
    assert_eq!(df.column("payload_ref").unwrap().null_count(), 2);

    // Roundtrip back to EventLogRecord
    let reconstructed = dataframe_to_records(&df).expect("Reconstruction must succeed");
    assert_eq!(reconstructed, records);
}

#[test]
fn test_extreme_ticks_full_u128_range_and_limb_subtraction() {
    let records = vec![
        sample_v2_record(0, None),
        sample_v2_record((1u128 << 64) - 1, None),
        sample_v2_record(1u128 << 64, None),
        sample_v2_record(u128::MAX, None),
    ];

    let df = records_to_dataframe(&records).expect("Extreme tick records should build");
    let hi_col = df.column("time_ticks_hi").unwrap().u64().unwrap();
    let lo_col = df.column("time_ticks_lo").unwrap().u64().unwrap();

    // Record 0: 0
    assert_eq!(hi_col.get(0), Some(0));
    assert_eq!(lo_col.get(0), Some(0));

    // Record 1: (1 << 64) - 1
    assert_eq!(hi_col.get(1), Some(0));
    assert_eq!(lo_col.get(1), Some(u64::MAX));

    // Record 2: 1 << 64
    assert_eq!(hi_col.get(2), Some(1));
    assert_eq!(lo_col.get(2), Some(0));

    // Record 3: u128::MAX
    assert_eq!(hi_col.get(3), Some(u64::MAX));
    assert_eq!(lo_col.get(3), Some(u64::MAX));

    // Test checked limb subtraction with borrow
    // Case 1: (1, 10) - (0, 20) -> borrow 1 from high limb -> (0, u64::MAX - 9)
    let (diff_hi, diff_lo) =
        subtract_tick_limbs(1, 10, 0, 20).expect("Borrow subtraction must succeed");
    assert_eq!(diff_hi, 0);
    assert_eq!(diff_lo, u64::MAX - 9);

    // Case 2: (5, 100) - (2, 50) -> no borrow -> (3, 50)
    let (diff_hi2, diff_lo2) =
        subtract_tick_limbs(5, 100, 2, 50).expect("Normal subtraction must succeed");
    assert_eq!(diff_hi2, 3);
    assert_eq!(diff_lo2, 50);

    // Case 3: underflow: (0, 10) - (1, 0)
    let err = subtract_tick_limbs(0, 10, 1, 0);
    assert!(err.is_err());
}

#[test]
fn test_invalid_input_rejection_for_v1_records() {
    let mut v1_record = sample_v2_record(100, None);
    v1_record.schema_version = 1;

    let res = records_to_dataframe(&[v1_record]);
    assert!(
        matches!(res, Err(ArrowError::UnsupportedSchemaVersion(1))),
        "Must reject non-v2 records with UnsupportedSchemaVersion"
    );

    // Mixed records: first is valid v2, second is v1 -> reports offending version 1
    let v2_record = sample_v2_record(200, None);
    let mut bad_record = sample_v2_record(300, None);
    bad_record.schema_version = 99;

    let mixed_res = records_to_dataframe(&[v2_record, bad_record]);
    assert!(
        matches!(mixed_res, Err(ArrowError::UnsupportedSchemaVersion(99))),
        "Must report exact version of the offending record"
    );
}

#[test]
fn test_independent_parquet_reader_roundtrip() {
    let records = vec![
        sample_v2_record(500, Some(EntityId::new(7, 2))),
        sample_v2_record(1500, None),
    ];
    let mut df = records_to_dataframe(&records).unwrap();

    let mut buf = Vec::new();
    write_parquet_zstd(&mut df, &mut buf).expect("write_parquet_zstd must succeed");

    // Independent Parquet format validation:
    // 1. Header magic PAR1 at 0..4
    assert!(
        buf.len() >= 12,
        "Parquet buffer must have header, footer length, and footer magic"
    );
    assert_eq!(&buf[0..4], b"PAR1", "Parquet header magic must be PAR1");

    // 2. Footer magic PAR1 at len - 4..len
    let len = buf.len();
    assert_eq!(
        &buf[len - 4..len],
        b"PAR1",
        "Parquet footer magic must be PAR1"
    );

    // 3. Read Thrift footer length
    let footer_len = u32::from_le_bytes(buf[len - 8..len - 4].try_into().unwrap()) as usize;
    assert!(footer_len > 0, "Footer length must be positive");
    assert!(
        footer_len + 8 <= len,
        "Footer length must fit within buffer"
    );

    // 4. Inspect footer bytes for Schema v2 column strings
    let footer_bytes = &buf[len - 8 - footer_len..len - 8];
    let footer_str = String::from_utf8_lossy(footer_bytes);
    assert!(
        footer_str.contains("time_ticks_hi"),
        "Footer must declare time_ticks_hi"
    );
    assert!(
        footer_str.contains("time_ticks_lo"),
        "Footer must declare time_ticks_lo"
    );
    assert!(
        footer_str.contains("entity_id_index"),
        "Footer must declare entity_id_index"
    );

    // 5. Read back via ParquetReader and assert logical equality
    let roundtrip_df = read_parquet(Cursor::new(buf)).expect("read_parquet must succeed");
    assert_eq!(roundtrip_df.height(), 2);
    let roundtrip_records = dataframe_to_records(&roundtrip_df).unwrap();
    assert_eq!(roundtrip_records, records);
}

#[test]
fn test_independent_ipc_reader_roundtrip() {
    let records = vec![
        sample_v2_record(1000, Some(EntityId::new(99, 5))),
        sample_v2_record(2000, None),
    ];
    let mut df = records_to_dataframe(&records).unwrap();

    let mut buf = Vec::new();
    write_ipc_stream(&mut df, &mut buf).expect("write_ipc_stream must succeed");

    // Stream encapsulation starts with the Arrow continuation marker, not
    // the ARROW1 file magic emitted by the previous implementation.
    assert_eq!(&buf[..4], &[0xff; 4]);
    assert!(!buf.starts_with(b"ARROW1"));

    let roundtrip_df = read_ipc_stream(Cursor::new(buf)).expect("read_ipc_stream must succeed");
    assert_eq!(roundtrip_df.height(), 2);
    let roundtrip_records = dataframe_to_records(&roundtrip_df).unwrap();
    assert_eq!(roundtrip_records, records);
}

#[test]
fn previously_written_ipc_files_have_an_explicit_reader() {
    let records = vec![sample_v2_record(1000, None)];
    let mut df = records_to_dataframe(&records).unwrap();
    let mut bytes = Vec::new();
    IpcWriter::new(&mut bytes).finish(&mut df).unwrap();
    assert!(bytes.starts_with(b"ARROW1"));
    let recovered = read_ipc_file(Cursor::new(bytes)).unwrap();
    assert_eq!(dataframe_to_records(&recovered).unwrap(), records);
}

#[test]
fn test_polars_tabular_transforms_and_aggregations() {
    let mut r1 = sample_v2_record(100, Some(EntityId::new(42, 1)));
    r1.status = EventStatus::Dispatched;
    let mut r2 = sample_v2_record(200, Some(EntityId::new(42, 1)));
    r2.status = EventStatus::Cancelled;
    let mut r3 = sample_v2_record(300, Some(EntityId::new(99, 1)));
    r3.status = EventStatus::Dispatched;

    let df = records_to_dataframe(&[r1, r2, r3]).unwrap();

    // 1. Filter by event kind
    let filtered_kind = filter_by_event_kind(&df, "custom:test").unwrap();
    assert_eq!(filtered_kind.height(), 3);

    let empty_kind = filter_by_event_kind(&df, "nonexistent").unwrap();
    assert_eq!(empty_kind.height(), 0);

    // 2. Filter by entity
    let filtered_entity = filter_by_entity(&df, 42, 1).unwrap();
    assert_eq!(filtered_entity.height(), 2);

    let filtered_entity_99 = filter_by_entity(&df, 99, 1).unwrap();
    assert_eq!(filtered_entity_99.height(), 1);

    // 3. Aggregate counts by status
    let agg = aggregate_counts_by_status(&df).unwrap();
    assert_eq!(agg.height(), 2);
}
