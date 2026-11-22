#![cfg(feature = "hdf5")]

use hdf5_metno::{types::TypeDescriptor, File};
use kairo_ecs_arrow::{
    ArrowError, CheckpointAdapter, CheckpointFormat, CheckpointManifest, EventLogRecord,
    EventStatus, Hdf5CheckpointAdapter, ParallelIoRecordBatch,
};
use kairo_ecs_types::{DispatchedEvent, EntityId, EventId, EventKind, SimTime};
use std::{
    fs,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    process::Command,
};

const HDF5_SIGNATURE: [u8; 8] = *b"\x89HDF\r\n\x1a\n";

fn checkpoint() -> CheckpointManifest {
    let first = DispatchedEvent::new(
        EventId::new(u64::MAX, 17),
        SimTime::from_ticks((3u128 << 64) | 9_223_372_036_854_775_813),
        -7,
        u64::MAX - 1,
        Some(EntityId::new(u64::MAX - 2, u32::MAX)),
        EventKind::custom(5),
    );
    let mut records = vec![EventLogRecord::dispatched("native-run", first)];
    for (index, status) in [
        EventStatus::Cancelled,
        EventStatus::Skipped,
        EventStatus::Error,
    ]
    .into_iter()
    .enumerate()
    {
        let mut row = records[0].clone();
        row.event_id = EventId::new(index as u64, index as u32);
        row.status = status;
        row.time_ticks = u128::MAX - index as u128;
        row.entity_id = None;
        row.event_kind = "étape\0de\tservice".into();
        row.payload_ref = Some("référence\n\0valide".into());
        records.push(row);
    }
    let batch = ParallelIoRecordBatch::from_records(records).unwrap();
    CheckpointManifest::from_batch("native-checkpoint", CheckpointFormat::Hdf5, &batch).unwrap()
}

fn write_fixture(path: &Path) -> CheckpointManifest {
    let manifest = checkpoint();
    let receipt = Hdf5CheckpointAdapter
        .write_manifest(path, &manifest)
        .unwrap();
    assert!(!receipt.contract_only);
    assert_eq!(receipt.row_count, manifest.row_count);
    manifest
}

#[test]
fn adapter_writes_a_native_hdf5_checkpoint() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("queue.h5");
    let manifest = write_fixture(&path);

    let mut signature = [0; 8];
    fs::File::open(&path)
        .unwrap()
        .read_exact(&mut signature)
        .unwrap();
    assert_eq!(signature, HDF5_SIGNATURE);
    assert_eq!(
        Hdf5CheckpointAdapter.restore_manifest(path).unwrap(),
        manifest.restore().unwrap()
    );
}

#[test]
fn legacy_contract_requires_the_explicit_legacy_reader() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("legacy.h5");
    let manifest = checkpoint();
    manifest.write_contract_file(&path).unwrap();
    assert_eq!(
        CheckpointManifest::read_contract_file(&path).unwrap(),
        manifest
    );
    assert!(matches!(
        Hdf5CheckpointAdapter.restore_manifest(path),
        Err(ArrowError::InvalidCheckpointFormat { .. })
    ));
}

fn overwrite_record_field(path: &Path, field_name: &str, bytes: &[u8]) {
    let offset = {
        let file = File::open(path).unwrap();
        let records = file.dataset("records").unwrap();
        let TypeDescriptor::Compound(ty) = records.dtype().unwrap().to_descriptor().unwrap() else {
            panic!("expected a compound native record dataset");
        };
        let field = ty
            .fields
            .iter()
            .find(|field| field.name == field_name)
            .unwrap();
        records.offset().unwrap() + field.offset as u64
    };
    let mut writable = fs::OpenOptions::new().write(true).open(path).unwrap();
    writable.seek(SeekFrom::Start(offset)).unwrap();
    writable.write_all(bytes).unwrap();
}

#[test]
fn rejects_corrupted_time_and_record_fields() {
    let directory = tempfile::tempdir().unwrap();
    let high_limb = 8_u64.to_le_bytes();
    let oversized_length = u64::MAX.to_le_bytes();
    let corruptions: [(&str, &[u8], ArrowError); 5] = [
        (
            "time_ticks_hi",
            &high_limb,
            ArrowError::InvalidNumber("checkpoint sha256"),
        ),
        (
            "kind_bytes",
            &oversized_length,
            ArrowError::InvalidNumber("text length"),
        ),
        (
            "entity_present",
            &[2],
            ArrowError::InvalidNumber("entity_present"),
        ),
        (
            "payload_present",
            &[2],
            ArrowError::InvalidNumber("payload_present"),
        ),
        ("status", &[255], ArrowError::InvalidStatus("255".into())),
    ];
    for (field, bytes, error) in corruptions {
        let path = directory.path().join(format!("{field}.h5"));
        write_fixture(&path);
        overwrite_record_field(&path, field, bytes);
        assert_eq!(Hdf5CheckpointAdapter.restore_manifest(path), Err(error));
    }
}

#[test]
fn rejects_wrong_schema_and_indirect_datasets() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("schema.h5");
    write_fixture(&path);
    {
        let native = File::open_rw(&path).unwrap();
        native.unlink("sha256").unwrap();
        native
            .new_dataset_builder()
            .with_data(&[0u8; 33])
            .create("sha256")
            .unwrap();
    }
    assert_eq!(
        Hdf5CheckpointAdapter.restore_manifest(&path),
        Err(ArrowError::UnexpectedFieldHeader)
    );
    write_fixture(&path);
    {
        let native = File::open_rw(&path).unwrap();
        native.link_soft("/text", "alias").unwrap();
    }
    assert!(matches!(
        Hdf5CheckpointAdapter.restore_manifest(path),
        Err(ArrowError::Io(message)) if message.contains("local hard links")
    ));
}

#[test]
fn invalid_replacement_preserves_the_previous_native_file() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("queue.h5");
    let original = write_fixture(&path);
    let snapshot = fs::read(&path).unwrap();
    let mut invalid = original.clone();
    invalid.blocks[0].offset_bytes = 1;

    assert!(matches!(
        Hdf5CheckpointAdapter.write_manifest(&path, &invalid),
        Err(ArrowError::InvalidBlockLayout(_))
    ));
    assert_eq!(fs::read(&path).unwrap(), snapshot);
    assert_eq!(
        Hdf5CheckpointAdapter.restore_manifest(path).unwrap(),
        original.restore().unwrap()
    );
}

#[cfg(unix)]
#[test]
fn rejects_non_utf8_path_before_publication() {
    use std::os::unix::ffi::OsStringExt;

    let directory = tempfile::tempdir().unwrap();
    let path = directory
        .path()
        .join(std::ffi::OsString::from_vec(vec![0xff]));
    let prior = checkpoint();

    // Darwin's default filesystem refuses this name. Linux additionally
    // exercises a prior checkpoint at the same path.
    #[cfg(target_os = "linux")]
    prior.write_contract_file(&path).unwrap();
    assert!(matches!(
        Hdf5CheckpointAdapter.write_manifest(&path, &prior),
        Err(ArrowError::InvalidArgument(
            "HDF5 checkpoint paths must be UTF-8"
        ))
    ));
    #[cfg(target_os = "linux")]
    assert_eq!(
        CheckpointManifest::read_contract_file(&path).unwrap(),
        prior
    );
    #[cfg(not(target_os = "linux"))]
    assert!(!path.exists());
}

#[test]
#[ignore = "requires independently installed HDF Group h5dump"]
fn h5dump_reads_the_adapter_output_and_rejects_the_legacy_bytes() {
    let reader = PathBuf::from(std::env::var_os("KAIROS_H5DUMP").expect("qualified h5dump path"));
    let directory = tempfile::tempdir().unwrap();
    let native = directory.path().join("physical.h5");
    let legacy = directory.path().join("legacy.h5");
    let manifest = write_fixture(&native);
    manifest.write_contract_file(&legacy).unwrap();

    let actual = Command::new(&reader)
        .args(["-d", "/records"])
        .arg(&native)
        .output()
        .unwrap();
    assert!(actual.status.success(), "{:?}", actual.stderr);
    let ddl = String::from_utf8(actual.stdout).unwrap();
    assert!(ddl.contains("H5T_STD_U64LE \"time_ticks_hi\""), "{ddl}");
    assert!(ddl.contains("H5T_STD_U64LE \"time_ticks_lo\""), "{ddl}");
    assert!(ddl.contains("H5T_STD_I32LE \"priority\""), "{ddl}");
    let rows = ddl.split_once("(0): {").unwrap().1;
    let first = rows.split_once('}').unwrap().0;
    let columns = first.split(',').map(str::trim).collect::<Vec<_>>();
    assert_eq!(&columns[2..4], ["3", "9223372036854775813"], "{ddl}");
    assert_eq!(columns[9], "-7", "{ddl}");
    assert!(rows.contains("18446744073709551615"), "{ddl}");
    println!("{ddl}");

    let rejected = Command::new(&reader).arg(legacy).output().unwrap();
    assert_eq!(rejected.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("unable to open file"));
}
