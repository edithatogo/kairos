#![cfg(feature = "parallel-io")]

use kairo_ecs_arrow::{CheckpointFormat, CheckpointManifest, ParallelIoRecordBatch};
use kairo_ecs_types::{DispatchedEvent, EntityId, EventId, EventKind, SimTime};

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

#[cfg(feature = "hdf5")]
#[test]
fn hdf5_checkpoint_contract_records_format_and_checksum() {
    let batch = ParallelIoRecordBatch::from_dispatched("hdf5-run", [event(7, 20, 0)])
        .expect("record batch");
    let checkpoint = CheckpointManifest::from_batch("hdf5-0001", CheckpointFormat::Hdf5, &batch)
        .expect("hdf5 checkpoint");

    assert_eq!(checkpoint.format, CheckpointFormat::Hdf5);
    assert_eq!(checkpoint.records[0].sequence, 0);
    assert_ne!(checkpoint.checksum, 0);
}

#[cfg(feature = "adios2")]
#[test]
fn adios2_checkpoint_contract_records_format_and_checksum() {
    let batch = ParallelIoRecordBatch::from_dispatched("adios2-run", [event(8, 30, 0)])
        .expect("record batch");
    let checkpoint =
        CheckpointManifest::from_batch("adios2-0001", CheckpointFormat::Adios2, &batch)
            .expect("adios2 checkpoint");

    assert_eq!(checkpoint.format, CheckpointFormat::Adios2);
    assert_eq!(checkpoint.records[0].time_ticks, 30);
    assert_ne!(checkpoint.checksum, 0);
}
