#![cfg(feature = "parallel-io")]

use kairo_ecs_arrow::{
    Adios2CheckpointAdapter, ArrowError, CheckpointAdapter, CheckpointFormat, CheckpointManifest,
    Hdf5CheckpointAdapter, ParallelIoRecordBatch,
};
use kairo_ecs_types::{DispatchedEvent, EntityId, EventId, EventKind, SimTime};
use std::{fs, path::PathBuf};

fn event(index: u64, ticks: u128, sequence: u64) -> DispatchedEvent {
    DispatchedEvent::new(
        EventId::new(index, 0),
        SimTime::from_ticks(ticks),
        0,
        sequence,
        Some(EntityId::new(index + 100, 0)),
        EventKind::custom(7),
    )
}

fn temp_path(label: &str) -> PathBuf {
    tempfile::Builder::new()
        .prefix(&format!("kairo-ecs-arrow-{label}-"))
        .suffix(".contract")
        .tempfile()
        .expect("tempfile creation")
        .into_temp_path()
        .to_path_buf()
}

#[test]
fn record_batch_declares_arrow_schema_and_contiguous_blocks() {
    let batch = ParallelIoRecordBatch::from_dispatched(
        "run-51",
        [event(1, 10, 0), event(2, 11, 1), event(3, 12, 2)],
    )
    .expect("record batch");

    assert_eq!(batch.row_count(), 3);
    assert_eq!(
        batch.schema_fingerprint(),
        kairo_ecs_arrow::event_log_schema_fingerprint()
    );
    assert_eq!(batch.contiguous_blocks().len(), 1);
    assert_eq!(batch.contiguous_blocks()[0].row_count, 3);
    assert!(batch.contiguous_blocks()[0].byte_len > 0);
}

#[test]
fn record_batch_writes_contiguous_block_contract_to_path() {
    let batch =
        ParallelIoRecordBatch::from_dispatched("write-run", [event(1, 10, 0), event(2, 11, 1)])
            .expect("record batch");
    let path = temp_path("record-batch");

    let manifest = batch
        .write_contiguous_blocks_to_path(&path)
        .expect("contiguous write");
    let payload = fs::read(&path).expect("written payload");
    let _ = fs::remove_file(&path);

    assert_eq!(payload, batch.to_record_batch_bytes());
    assert_eq!(manifest.row_count, 2);
    assert_eq!(manifest.encoded_len, payload.len());
    assert_eq!(manifest.blocks.as_slice(), batch.contiguous_blocks());
    assert_eq!(manifest.blocks[0].offset_bytes, 0);
    assert_eq!(manifest.blocks[0].byte_len as usize, payload.len());
    assert_ne!(manifest.checksum, 0);
}

#[test]
fn checkpoint_manifest_round_trips_and_restores_final_tick() {
    let batch =
        ParallelIoRecordBatch::from_dispatched("restart-run", [event(1, 10, 0), event(2, 14, 1)])
            .expect("record batch");
    let checkpoint =
        CheckpointManifest::from_batch("checkpoint-0001", CheckpointFormat::LocalContract, &batch)
            .expect("checkpoint");

    let encoded = checkpoint.to_contract_bytes();
    let decoded = CheckpointManifest::from_contract_bytes(&encoded).expect("decoded checkpoint");
    let restored = decoded.restore().expect("restored checkpoint");

    assert_eq!(decoded, checkpoint);
    assert_eq!(restored.run_id, "restart-run");
    assert_eq!(restored.records.len(), 2);
    assert_eq!(restored.final_tick, 14);
}

#[test]
fn checkpoint_manifest_file_writer_round_trips_and_rejects_bad_block_layout() {
    let batch =
        ParallelIoRecordBatch::from_dispatched("file-run", [event(3, 20, 0), event(4, 25, 1)])
            .expect("record batch");
    let checkpoint =
        CheckpointManifest::from_batch("checkpoint-file", CheckpointFormat::LocalContract, &batch)
            .expect("checkpoint");
    let path = temp_path("checkpoint");

    checkpoint
        .write_contract_file(&path)
        .expect("checkpoint manifest write");
    let decoded = CheckpointManifest::read_contract_file(&path).expect("checkpoint manifest read");
    let _ = fs::remove_file(&path);

    assert_eq!(decoded, checkpoint);

    let mut invalid = decoded;
    invalid.blocks[0].offset_bytes = 1;
    assert!(matches!(
        invalid.validate_for_restore(),
        Err(ArrowError::InvalidBlockLayout(_))
    ));
}

#[test]
fn checkpoint_checksum_rejects_corrupted_record_fields() {
    let batch = ParallelIoRecordBatch::from_dispatched("checksum-run", [event(4, 18, 0)])
        .expect("record batch");
    let checkpoint = CheckpointManifest::from_batch(
        "checkpoint-checksum",
        CheckpointFormat::LocalContract,
        &batch,
    )
    .expect("checkpoint");

    let mut encoded = String::from_utf8(checkpoint.to_contract_bytes()).expect("utf8 checkpoint");
    encoded = encoded.replace("\t0\t0\tcustom:7\t", "\t5\t0\tcustom:7\t");

    assert!(CheckpointManifest::from_contract_bytes(encoded.as_bytes()).is_err());
}

#[cfg(not(feature = "hdf5"))]
#[test]
fn hdf5_adapter_returns_disabled_without_feature() {
    let batch = ParallelIoRecordBatch::from_dispatched("hdf5-disabled", [event(7, 20, 0)])
        .expect("record batch");
    let checkpoint =
        CheckpointManifest::from_batch("hdf5-disabled", CheckpointFormat::Hdf5, &batch)
            .expect("hdf5 checkpoint");
    let adapter = Hdf5CheckpointAdapter;
    let path = temp_path("hdf5-disabled");

    assert_eq!(
        adapter.write_manifest(&path, &checkpoint),
        Err(ArrowError::AdapterDisabled("hdf5"))
    );
    assert_eq!(
        adapter.restore_manifest(&path),
        Err(ArrowError::AdapterDisabled("hdf5"))
    );
}

#[cfg(feature = "hdf5")]
#[test]
fn hdf5_checkpoint_contract_records_format_and_checksum() {
    let batch = ParallelIoRecordBatch::from_dispatched("hdf5-run", [event(7, 20, 0)])
        .expect("record batch");
    let checkpoint = CheckpointManifest::from_batch("hdf5-0001", CheckpointFormat::Hdf5, &batch)
        .expect("hdf5 checkpoint");
    let adapter = Hdf5CheckpointAdapter;
    let path = temp_path("hdf5");

    let receipt = adapter
        .write_manifest(&path, &checkpoint)
        .expect("hdf5 contract write");
    let restored = adapter
        .restore_manifest(&path)
        .expect("hdf5 contract restore");
    let _ = fs::remove_file(&path);

    assert_eq!(receipt.format, CheckpointFormat::Hdf5);
    assert!(receipt.contract_only);
    assert_eq!(checkpoint.format, CheckpointFormat::Hdf5);
    assert_eq!(checkpoint.records[0].sequence, 0);
    assert_eq!(restored.final_tick, 20);
    assert_ne!(checkpoint.checksum, 0);
}

#[cfg(not(feature = "adios2"))]
#[test]
fn adios2_adapter_returns_disabled_without_feature() {
    let batch = ParallelIoRecordBatch::from_dispatched("adios2-disabled", [event(8, 30, 0)])
        .expect("record batch");
    let checkpoint =
        CheckpointManifest::from_batch("adios2-disabled", CheckpointFormat::Adios2, &batch)
            .expect("adios2 checkpoint");
    let adapter = Adios2CheckpointAdapter;
    let path = temp_path("adios2-disabled");

    assert_eq!(
        adapter.write_manifest(&path, &checkpoint),
        Err(ArrowError::AdapterDisabled("adios2"))
    );
    assert_eq!(
        adapter.restore_manifest(&path),
        Err(ArrowError::AdapterDisabled("adios2"))
    );
}

#[cfg(feature = "adios2")]
#[test]
fn adios2_checkpoint_contract_records_format_and_checksum() {
    let batch = ParallelIoRecordBatch::from_dispatched("adios2-run", [event(8, 30, 0)])
        .expect("record batch");
    let checkpoint =
        CheckpointManifest::from_batch("adios2-0001", CheckpointFormat::Adios2, &batch)
            .expect("adios2 checkpoint");
    let adapter = Adios2CheckpointAdapter;
    let path = temp_path("adios2");

    let receipt = adapter
        .write_manifest(&path, &checkpoint)
        .expect("adios2 contract write");
    let restored = adapter
        .restore_manifest(&path)
        .expect("adios2 contract restore");
    let _ = fs::remove_file(&path);

    assert_eq!(receipt.format, CheckpointFormat::Adios2);
    assert!(receipt.contract_only);
    assert_eq!(checkpoint.format, CheckpointFormat::Adios2);
    assert_eq!(checkpoint.records[0].time_ticks, 30);
    assert_eq!(restored.final_tick, 30);
    assert_ne!(checkpoint.checksum, 0);
}

#[test]
fn invalid_replacement_preserves_the_last_restorable_checkpoint() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("restart");
    let batch = ParallelIoRecordBatch::from_dispatched("recovery", [event(1, 17, 0)]).unwrap();
    let checkpoint =
        CheckpointManifest::from_batch("known-good", CheckpointFormat::LocalContract, &batch)
            .unwrap();
    checkpoint.write_contract_file(&path).unwrap();
    let mut invalid = checkpoint.clone();
    invalid.checksum ^= 1;
    assert!(invalid.write_contract_file(&path).is_err());
    assert_eq!(
        CheckpointManifest::read_contract_file(&path).unwrap(),
        checkpoint
    );
}

#[cfg(unix)]
#[test]
fn replacement_keeps_group_read_mode_and_the_previous_inode_intact() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("restart");
    let previous = dir.path().join("previous");
    let batch = ParallelIoRecordBatch::from_dispatched("mode", [event(1, 1, 0)]).unwrap();
    let first =
        CheckpointManifest::from_batch("first", CheckpointFormat::LocalContract, &batch).unwrap();
    first.write_contract_file(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
    fs::hard_link(&path, &previous).unwrap();
    let second =
        CheckpointManifest::from_batch("second", CheckpointFormat::LocalContract, &batch).unwrap();
    second.write_contract_file(&path).unwrap();
    assert_eq!(
        CheckpointManifest::read_contract_file(previous).unwrap(),
        first
    );
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o640
    );
    assert_eq!(
        CheckpointManifest::read_contract_file(path).unwrap(),
        second
    );
}

#[test]
fn lossy_v1_candidates_cannot_displace_a_valid_generation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("restart");
    let batch =
        ParallelIoRecordBatch::from_dispatched("recovery", [event(1, 17, 0), event(2, 18, 1)])
            .unwrap();
    let good =
        CheckpointManifest::from_batch("good", CheckpointFormat::LocalContract, &batch).unwrap();
    good.write_contract_file(&path).unwrap();

    let mut multi = good.clone();
    let bytes = multi.blocks[0].byte_len;
    multi.blocks[0].row_count = 1;
    multi.blocks[0].byte_len = 1;
    multi.blocks.push(kairo_ecs_arrow::ContiguousBlock {
        offset_bytes: 1,
        byte_len: bytes - 1,
        row_count: 1,
    });
    assert!(multi.validate_for_restore().is_ok());
    assert!(multi.write_contract_file(&path).is_err());

    let mut record = kairo_ecs_arrow::EventLogRecord::dispatched("recovery", event(3, 19, 2));
    record.payload_ref = Some("payload\r".into());
    let batch = ParallelIoRecordBatch::from_records(vec![record]).unwrap();
    let lossy =
        CheckpointManifest::from_batch("lossy", CheckpointFormat::LocalContract, &batch).unwrap();
    assert!(lossy.validate_for_restore().is_ok());
    assert!(lossy.write_contract_file(&path).is_err());
    assert_eq!(CheckpointManifest::read_contract_file(path).unwrap(), good);
}

#[test]
fn trusted_digest_detects_upper_tick_corruption_in_legacy_v1() {
    let batch = ParallelIoRecordBatch::from_dispatched("digest", [event(1, 17, 0)]).unwrap();
    let checkpoint =
        CheckpointManifest::from_batch("previous", CheckpointFormat::LocalContract, &batch)
            .unwrap();
    let bytes = checkpoint.to_contract_bytes();
    let trusted = kairo_ecs_arrow::compute_sha256(&bytes);
    let changed_tick = 17_u128 + (1_u128 << 64);
    let tampered = String::from_utf8(bytes)
        .unwrap()
        .replace("\t17\t", &format!("\t{changed_tick}\t"))
        .into_bytes();
    // Existing v1 readers accept legacy checksums. Authenticate the same bytes
    // that will be parsed when the storage contract requires full integrity.
    let legacy = CheckpointManifest::from_contract_bytes(&tampered).unwrap();
    assert_eq!(legacy.records[0].time_ticks, changed_tick);
    assert_ne!(kairo_ecs_arrow::compute_sha256(&tampered), trusted);
}
