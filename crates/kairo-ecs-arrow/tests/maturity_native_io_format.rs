#![cfg(feature = "hdf5")]

use kairo_ecs_arrow::{
    CheckpointAdapter, CheckpointFormat, CheckpointManifest, Hdf5CheckpointAdapter,
    ParallelIoRecordBatch,
};
use kairo_ecs_types::{DispatchedEvent, EntityId, EventId, EventKind, SimTime};
use std::io::Read;

const HDF5_SIGNATURE: [u8; 8] = *b"\x89HDF\r\n\x1a\n";

fn checkpoint() -> CheckpointManifest {
    let event = DispatchedEvent::new(
        EventId::new(u64::MAX, 17),
        SimTime::from_ticks(u128::MAX),
        -7,
        u64::MAX - 1,
        Some(EntityId::new(u64::MAX - 2, u32::MAX)),
        EventKind::custom(5),
    );
    let batch = ParallelIoRecordBatch::from_dispatched("native-run", [event]).unwrap();
    CheckpointManifest::from_batch("native-checkpoint", CheckpointFormat::Hdf5, &batch).unwrap()
}

#[test]
fn adapter_writes_a_native_hdf5_checkpoint() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("queue.h5");
    let manifest = checkpoint();
    let receipt = Hdf5CheckpointAdapter
        .write_manifest(&path, &manifest)
        .unwrap();

    let mut signature = [0; 8];
    std::fs::File::open(&path)
        .unwrap()
        .read_exact(&mut signature)
        .unwrap();
    assert_eq!(signature, HDF5_SIGNATURE);
    assert!(!receipt.contract_only);
    assert_eq!(
        Hdf5CheckpointAdapter.restore_manifest(path).unwrap(),
        manifest.restore().unwrap()
    );
}
