#![cfg(feature = "polars-native")]

use kairo_ecs_arrow::polars_interop::*;
use kairo_ecs_arrow::*;
use kairo_ecs_types::{EntityId, EventId};
use std::fs;

fn sample_v2_record(seq: u64, time_ticks: u128, entity_id: Option<EntityId>) -> EventLogRecord {
    EventLogRecord {
        schema_version: SCHEMA_VERSION_V2,
        run_id: "run-durable-01".to_string(),
        event_id: EventId::new(seq, 1),
        entity_id,
        time_ticks,
        time_scale: "ticks".to_string(),
        priority: 0,
        sequence: seq,
        event_kind: "custom:durable".to_string(),
        status: EventStatus::Dispatched,
        payload_ref: None,
    }
}

#[test]
fn test_failing_manifest_last_publication_and_interruption_rejection() {
    let temp = tempfile::tempdir().expect("isolated tempdir");
    let temp_dir = temp.path();

    let records: Vec<EventLogRecord> = (1..=50)
        .map(|i| sample_v2_record(i, (i * 10) as u128, Some(EntityId::new(i, 1))))
        .collect();

    let writer = DurableRunPublicationWriter::new(temp_dir, "run-durable-01", 10);
    // Write parts
    let receipt = writer.publish(&records).expect("Publication must succeed");
    assert_eq!(receipt.total_rows, 50);
    assert_eq!(receipt.parts_count, 5); // 50 / 10 = 5 parts
    assert!(receipt.manifest_path.exists());

    // Reader must successfully read back completed publication
    let reader = DurableRunPublicationReader::new(temp_dir, "run-durable-01");
    let read_back = reader.read_all_records().expect("Read back must succeed");
    assert_eq!(read_back.len(), 50);
    assert_eq!(read_back, records);

    // Simulate interruption before manifest-last publication (delete manifest.json)
    fs::remove_file(&receipt.manifest_path).unwrap();
    let err = reader.read_all_records();
    assert!(err.is_err(), "Must reject unmanifested/interrupted run");
}

#[test]
fn test_failing_resume_watermark_no_data_duplication() {
    let temp = tempfile::tempdir().expect("isolated tempdir");
    let temp_dir = temp.path();

    let batch1: Vec<EventLogRecord> = (1..=20)
        .map(|i| sample_v2_record(i, (i * 10) as u128, None))
        .collect();

    let writer = DurableRunPublicationWriter::new(temp_dir, "run-resume-01", 10);
    let receipt1 = writer.publish(&batch1).unwrap();
    assert_eq!(receipt1.watermark_sequence, 20);

    // Check watermark
    let reader = DurableRunPublicationReader::new(temp_dir, "run-resume-01");
    assert_eq!(reader.get_watermark_sequence().unwrap(), 20);

    // Filter incoming stream by resume watermark
    let batch2: Vec<EventLogRecord> = (15..=30)
        .map(|i| sample_v2_record(i, (i * 10) as u128, None))
        .collect();

    let deduplicated =
        writer.filter_new_records_since_watermark(&batch2, receipt1.watermark_sequence);
    assert_eq!(deduplicated.len(), 10); // only sequences 21..=30
    assert_eq!(deduplicated[0].sequence, 21);
    assert_eq!(deduplicated[9].sequence, 30);
}

#[test]
fn test_failing_tampered_part_file_checksum_rejection() {
    let temp = tempfile::tempdir().expect("isolated tempdir");
    let temp_dir = temp.path();

    let records: Vec<EventLogRecord> = (1..=10)
        .map(|i| sample_v2_record(i, (i * 100) as u128, None))
        .collect();

    let writer = DurableRunPublicationWriter::new(temp_dir, "run-tamper", 10);
    let _receipt = writer.publish(&records).unwrap();

    // Tamper with part-00000.parquet
    let part_file = temp_dir.join("run-tamper").join("part-00000.parquet");
    let mut bytes = fs::read(&part_file).unwrap();
    bytes[10] ^= 0xFF;
    fs::write(&part_file, bytes).unwrap();

    let reader = DurableRunPublicationReader::new(temp_dir, "run-tamper");
    let res = reader.read_all_records();
    assert!(
        res.is_err(),
        "Must reject tampered part file with checksum mismatch"
    );
}

#[test]
fn test_failing_thread_budget_configuration() {
    // Budget configuration must set and report explicit thread pool limit
    assert_eq!(get_or_set_execution_thread_budget(4), 4);
}
