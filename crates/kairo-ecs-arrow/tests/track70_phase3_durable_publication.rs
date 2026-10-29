#![cfg(feature = "polars-native")]

mod support;
use support::protected_tempdir;

use kairo_ecs_arrow::polars_interop::*;
use kairo_ecs_arrow::*;
use kairo_ecs_types::{EntityId, EventId};
use std::fs;

#[cfg(unix)]
#[test]
fn shared_writable_root_is_rejected_before_run_creation() {
    use std::os::unix::fs::PermissionsExt;
    let root = protected_tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o777)).unwrap();
    let writer = DurableRunPublicationWriter::new(root.path(), "rejected", 10);
    assert!(writer.publish(&[]).is_err());
    assert!(!root.path().join("rejected").exists());
}

#[cfg(target_os = "macos")]
#[test]
fn root_acl_write_grant_is_rejected_before_run_creation() {
    let root = protected_tempdir().unwrap();
    let status = std::process::Command::new("/bin/chmod")
        .args(["+a", "everyone allow add_file,delete_child"])
        .arg(root.path())
        .status()
        .unwrap();
    assert!(status.success());
    let writer = DurableRunPublicationWriter::new(root.path(), "rejected", 10);
    assert!(writer.publish(&[]).is_err());
    assert!(!root.path().join("rejected").exists());
}

#[cfg(unix)]
#[test]
fn published_run_and_files_are_private() {
    use std::os::unix::fs::PermissionsExt;
    let root = protected_tempdir().unwrap();
    let writer = DurableRunPublicationWriter::new(root.path(), "private", 10);
    let receipt = writer.publish(&[sample_v2_record(1, 10, None)]).unwrap();
    assert_eq!(
        fs::metadata(root.path().join("private"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    for file in [
        receipt.manifest_path,
        root.path().join("private/part-00000.parquet"),
    ] {
        assert_eq!(
            fs::metadata(file).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

#[test]
fn absent_output_root_is_not_created_implicitly() {
    let root = protected_tempdir().unwrap();
    let absent = root.path().join("absent");
    let writer = DurableRunPublicationWriter::new(&absent, "rejected", 10);
    assert!(writer.publish(&[]).is_err());
    assert!(!absent.exists());
}

#[cfg(unix)]
#[test]
fn symlink_output_root_is_rejected_before_run_creation() {
    let root = protected_tempdir().unwrap();
    let target = protected_tempdir().unwrap();
    let link = root.path().join("link");
    std::os::unix::fs::symlink(target.path(), &link).unwrap();
    let writer = DurableRunPublicationWriter::new(link, "rejected", 10);
    assert!(writer.publish(&[]).is_err());
    assert!(!target.path().join("rejected").exists());
}

#[test]
fn manifest_json_is_strict_and_roundtrips_escaped_strings() {
    let manifest = DurableManifest {
        run_id: "quoted\"id\\suffix".into(),
        schema_stream: "v2".into(),
        total_rows: 0,
        watermark_sequence: 0,
        completed: true,
        parts: vec![],
    };
    let json = manifest.to_json();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["run_id"], manifest.run_id);
    assert_eq!(DurableManifest::from_json(&json).unwrap(), manifest);
    assert!(DurableManifest::from_json(r#"{"run_id":"x","schema_stream":"v2","total_rows":1,"watermark_sequence":0,"completed":true}"#).is_err());
}

#[test]
fn completed_run_cannot_be_overwritten_even_when_requested() {
    let temp = protected_tempdir().unwrap();
    let writer = DurableRunPublicationWriter::new(temp.path(), "run", 10).with_overwrite(true);
    let records = vec![sample_v2_record(1, 10, None)];
    let receipt = writer.publish(&records).unwrap();
    let before = fs::read(&receipt.manifest_path).unwrap();
    assert!(writer.publish(&[]).is_err());
    assert_eq!(before, fs::read(&receipt.manifest_path).unwrap());
}

#[test]
fn manifest_reader_rejects_traversal_and_inconsistent_totals() {
    let temp = protected_tempdir().unwrap();
    let dir = temp.path().join("run");
    fs::create_dir(&dir).unwrap();
    let reader = DurableRunPublicationReader::new(temp.path(), "run");
    let invalid = DurableManifest {
        run_id: "run".into(),
        schema_stream: EVENT_LOG_STREAM_V2.into(),
        total_rows: 1,
        watermark_sequence: 1,
        completed: true,
        parts: vec![DurablePartMetadata {
            filename: "../outside".into(),
            row_count: 1,
            checksum: 0,
            max_sequence: 1,
        }],
    };
    fs::write(dir.join("manifest.json"), invalid.to_json()).unwrap();
    assert!(reader.read_manifest().is_err());
    assert!(reader.read_all_records().is_err());
    let mut invalid = invalid;
    invalid.parts.clear();
    fs::write(dir.join("manifest.json"), invalid.to_json()).unwrap();
    assert!(reader.read_all_records().is_err());
}

#[cfg(unix)]
#[test]
fn reader_rejects_symlink_part() {
    use std::os::unix::fs::symlink;
    let temp = protected_tempdir().unwrap();
    let dir = temp.path().join("run");
    fs::create_dir(&dir).unwrap();
    fs::write(temp.path().join("outside"), b"data").unwrap();
    symlink(temp.path().join("outside"), dir.join("part.parquet")).unwrap();
    let manifest = DurableManifest {
        run_id: "run".into(),
        schema_stream: EVENT_LOG_STREAM_V2.into(),
        total_rows: 1,
        watermark_sequence: 1,
        completed: true,
        parts: vec![DurablePartMetadata {
            filename: "part.parquet".into(),
            row_count: 1,
            checksum: calculate_checksum(b"data"),
            max_sequence: 1,
        }],
    };
    fs::write(dir.join("manifest.json"), manifest.to_json()).unwrap();
    assert!(DurableRunPublicationReader::new(temp.path(), "run")
        .read_all_records()
        .is_err());
}

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
    let temp = protected_tempdir().expect("isolated tempdir");
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
    let temp = protected_tempdir().expect("isolated tempdir");
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
    let temp = protected_tempdir().expect("isolated tempdir");
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
    assert!(reader.get_watermark_sequence().is_err());
    fs::remove_file(part_file).unwrap();
    assert!(reader.get_watermark_sequence().is_err());
}

#[test]
fn test_failing_thread_budget_configuration() {
    // Budget configuration must set and report explicit thread pool limit
    assert_eq!(get_or_set_execution_thread_budget(4), 4);
}
