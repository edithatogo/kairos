use kairo_ecs_arrow::*;
use std::fs;
use std::thread;

#[test]
fn test_concurrent_directory_creation_and_hermetic_isolation() {
    let thread_count = 8;
    let mut handles = Vec::new();

    for thread_idx in 0..thread_count {
        let handle = thread::spawn(move || {
            let temp_dir = tempfile::tempdir().expect("Failed to create isolated tempdir");
            let dir_path = temp_dir.path().to_path_buf();
            assert!(dir_path.exists());

            // Write thread-specific payload
            let file_path = dir_path.join("data.txt");
            let payload = format!("payload from thread {thread_idx}");
            fs::write(&file_path, payload.as_bytes()).unwrap();

            assert_eq!(fs::read_to_string(&file_path).unwrap(), payload);

            // Return path to verify cleanup on drop
            (temp_dir, file_path)
        });
        handles.push(handle);
    }

    for handle in handles {
        let (temp_dir, file_path) = handle.join().expect("Thread panicked");
        let dir_path = temp_dir.path().to_path_buf();
        assert!(file_path.exists());

        // Drop the TempDir and ensure automatic deletion
        drop(temp_dir);
        assert!(!dir_path.exists(), "TempDir must be cleaned up on drop");
        assert!(
            !file_path.exists(),
            "Contained file must be cleaned up on drop"
        );
    }
}

#[test]
fn test_atomic_publication_staging_and_cleanup_on_failure() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let target_file = temp_dir.path().join("part-00001.parquet");

    // 1. Staged write interrupted before persist
    {
        use std::io::Write;
        let mut staged = tempfile::NamedTempFile::new_in(temp_dir.path()).expect("staged file");
        staged.write_all(b"partial-data-before-crash").unwrap();
        // Drop without persist: simulate process crash / abort
        drop(staged);
    }

    // Destination file must NOT exist
    assert!(
        !target_file.exists(),
        "Target file must not exist after aborted staging"
    );

    // Directory must contain 0 files (staging scratch file was deleted on drop)
    let entries: Vec<_> = fs::read_dir(temp_dir.path()).unwrap().collect();
    assert_eq!(entries.len(), 0, "No orphaned scratch files should remain");

    // 2. Successful atomic write
    let valid_data = b"complete-durable-parquet-bytes";
    atomic_write_file(&target_file, valid_data, true).expect("atomic write must succeed");

    assert!(target_file.exists());
    assert_eq!(fs::read(&target_file).unwrap(), valid_data);
}

#[test]
fn test_no_clobber_protection_preserves_existing_output() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let target_file = temp_dir.path().join("protected_output.bin");

    let original_data = b"original-unmodified-data";
    atomic_write_file(&target_file, original_data, false).expect("initial write must succeed");
    assert_eq!(fs::read(&target_file).unwrap(), original_data);

    // Attempt overwrite with allow_overwrite = false (no-clobber)
    let new_data = b"new-clobbering-data";
    let res = atomic_write_file(&target_file, new_data, false);
    assert!(res.is_err(), "Must return error on no-clobber violation");

    // Verify original data was preserved intact
    assert_eq!(
        fs::read(&target_file).unwrap(),
        original_data,
        "Original file must remain unmodified after no-clobber rejection"
    );

    // Now overwrite with allow_overwrite = true
    atomic_write_file(&target_file, new_data, true).expect("overwrite must succeed");
    assert_eq!(
        fs::read(&target_file).unwrap(),
        new_data,
        "File must be updated when allow_overwrite is true"
    );
}

#[test]
fn test_checkpoint_writer_atomic_write_file_with_sha256() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let checkpoint_path = temp_dir.path().join("checkpoint.parquet");

    let schema = ArrowNativeSchema::from_event_log_fields();
    let batch = ArrowRecordBatch::from_u64_column(&[1, 2, 3], &[100, 200, 300]);

    ParquetZstdCheckpointWriter::write_checkpoint_file(
        &checkpoint_path,
        &schema,
        core::slice::from_ref(&batch),
        false,
    )
    .expect("write checkpoint file must succeed");

    assert!(checkpoint_path.exists());
    let bytes = fs::read(&checkpoint_path).unwrap();

    let (decoded_schema, decoded_batches) =
        ParquetZstdCheckpointReader::read_checkpoint(&bytes).expect("valid checkpoint read");
    assert_eq!(decoded_schema.fields.len(), schema.fields.len());
    assert_eq!(decoded_batches.len(), 1);

    // No-clobber rejection
    let clobber_res = ParquetZstdCheckpointWriter::write_checkpoint_file(
        &checkpoint_path,
        &schema,
        core::slice::from_ref(&batch),
        false,
    );
    assert!(
        clobber_res.is_err(),
        "Must reject clobbering existing checkpoint file"
    );
}

#[cfg(feature = "polars-native")]
mod polars_publication_tests {
    use super::*;
    use kairo_ecs_arrow::polars_interop::*;
    use kairo_ecs_types::{EntityId, EventId};

    fn sample_record(seq: u64, ticks: u128) -> EventLogRecord {
        EventLogRecord {
            schema_version: SCHEMA_VERSION_V2,
            run_id: "run-tempfile-verify".to_string(),
            event_id: EventId::new(seq, 1),
            entity_id: Some(EntityId::new(seq + 10, 1)),
            time_ticks: ticks,
            time_scale: "ticks".to_string(),
            priority: 0,
            sequence: seq,
            event_kind: "custom:tempfile".to_string(),
            status: EventStatus::Dispatched,
            payload_ref: None,
        }
    }

    #[test]
    fn test_durable_publication_writer_no_clobber_and_deterministic_identity() {
        let temp1 = tempfile::tempdir().unwrap();
        let temp2 = tempfile::tempdir().unwrap();

        let records: Vec<EventLogRecord> = (1..=30)
            .map(|i| sample_record(i, (i * 10) as u128))
            .collect();

        // Writer 1 in temp1
        let writer1 = DurableRunPublicationWriter::new(temp1.path(), "run-01", 10);
        let receipt1 = writer1.publish(&records).expect("publish 1");

        // Writer 2 in temp2 (identical records)
        let writer2 = DurableRunPublicationWriter::new(temp2.path(), "run-01", 10);
        let receipt2 = writer2.publish(&records).expect("publish 2");

        // Read manifests
        let manifest1_str = fs::read_to_string(&receipt1.manifest_path).unwrap();
        let manifest2_str = fs::read_to_string(&receipt2.manifest_path).unwrap();

        // 1. Bit-for-bit determinism: manifests must be identical
        assert_eq!(
            manifest1_str, manifest2_str,
            "Manifests must be identical for identical inputs"
        );
        assert_eq!(receipt1.watermark_sequence, receipt2.watermark_sequence);
        assert_eq!(receipt1.parts_count, 3);

        // 2. No-clobber protection test on writer1
        let no_clobber_writer = writer1.with_no_clobber();
        let clobber_err = no_clobber_writer.publish(&records);
        assert!(
            clobber_err.is_err(),
            "Must reject publication when parts already exist in no-clobber mode"
        );

        // Reader verification
        let reader = DurableRunPublicationReader::new(temp1.path(), "run-01");
        let read_records = reader.read_all_records().expect("read back");
        assert_eq!(read_records, records);
    }
}
