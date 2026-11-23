#![cfg(all(feature = "adios2", any(target_os = "linux", target_os = "macos")))]

use kairo_ecs_arrow::{
    Adios2CheckpointAdapter, ArrowError, CheckpointAdapter, CheckpointFormat, CheckpointManifest,
    EventLogRecord, EventStatus, ParallelIoRecordBatch,
};
use kairo_ecs_types::{DispatchedEvent, EntityId, EventId, EventKind, SimTime};
use std::{
    fs,
    os::unix::fs::{symlink, MetadataExt, PermissionsExt},
    path::Path,
    process::{Command, Output},
};

const HIGH: u64 = 0x2a48_b5c2_9134_f0c6;
const LOW: u64 = 9_223_372_036_854_775_813;

fn manifest() -> CheckpointManifest {
    let first = DispatchedEvent::new(
        EventId::new(u64::MAX, 17),
        SimTime::from_ticks((u128::from(HIGH) << 64) | u128::from(LOW)),
        -7,
        u64::MAX - 1,
        Some(EntityId::new(0, 0)),
        EventKind::custom(5),
    );
    let mut rows = vec![EventLogRecord::dispatched("native-run", first)];
    for (index, status) in [
        EventStatus::Cancelled,
        EventStatus::Skipped,
        EventStatus::Error,
    ]
    .into_iter()
    .enumerate()
    {
        let mut row = rows[0].clone();
        row.event_id = EventId::new(index as u64, index as u32);
        row.status = status;
        row.time_ticks = u128::MAX - index as u128;
        row.entity_id = None;
        row.event_kind = "étape\0de\tservice".into();
        row.payload_ref = Some("réf\n\0".into());
        rows.push(row);
    }
    let batch = ParallelIoRecordBatch::from_records(rows).unwrap();
    CheckpointManifest::from_batch("bp5-checkpoint", CheckpointFormat::Adios2, &batch).unwrap()
}

#[test]
fn native_generation_replacement_preserves_permissions_and_recovers() {
    let scratch = tempfile::tempdir().unwrap();
    let path = scratch.path().join("restart.bp");
    let original = manifest();
    let adapter = Adios2CheckpointAdapter;
    let receipt = adapter.write_manifest(&path, &original).unwrap();
    assert!(!receipt.contract_only);
    assert_eq!(receipt.row_count, original.row_count);
    assert_eq!(
        adapter.restore_manifest(&path).unwrap(),
        original.restore().unwrap()
    );

    let previous_inode = fs::metadata(&path).unwrap().ino();
    assert_eq!(fs::metadata(&path).unwrap().mode() & 0o777, 0o700);
    let data = path.join("data.0");
    assert_eq!(fs::metadata(&data).unwrap().mode() & 0o777, 0o600);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o750)).unwrap();
    fs::set_permissions(&data, fs::Permissions::from_mode(0o640)).unwrap();

    let mut invalid = original.clone();
    invalid.checksum ^= 1;
    assert!(adapter.write_manifest(&path, &invalid).is_err());
    assert_eq!(fs::metadata(&path).unwrap().ino(), previous_inode);
    assert_eq!(
        adapter.restore_manifest(&path).unwrap(),
        original.restore().unwrap()
    );

    let mut replacement = original.clone();
    replacement.records[0].time_ticks ^= 1 << 90;
    adapter.write_manifest(&path, &replacement).unwrap();
    assert_ne!(fs::metadata(&path).unwrap().ino(), previous_inode);
    assert_eq!(fs::metadata(&path).unwrap().mode() & 0o777, 0o750);
    assert_eq!(fs::metadata(&data).unwrap().mode() & 0o777, 0o640);
    assert_eq!(
        adapter.restore_manifest(&path).unwrap(),
        replacement.restore().unwrap()
    );
    assert_eq!(fs::read_dir(scratch.path()).unwrap().count(), 1);
}

#[test]
fn read_only_previous_generation_has_an_explicit_publication_outcome() {
    let scratch = tempfile::tempdir().unwrap();
    let path = scratch.path().join("restart.bp");
    let mut next = manifest();
    let adapter = Adios2CheckpointAdapter;
    adapter.write_manifest(&path, &next).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o555)).unwrap();
    next.records[0].time_ticks ^= 1 << 90;
    let result = adapter.write_manifest(&path, &next);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(
        adapter.restore_manifest(&path).unwrap(),
        next.restore().unwrap()
    );

    let stages: Vec<_> = fs::read_dir(scratch.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|entry| *entry != path)
        .collect();
    match result {
        Ok(_) => assert!(
            stages.is_empty(),
            "successful cleanup left a previous generation"
        ),
        Err(ArrowError::Io(reason)) => {
            assert!(reason.contains("was published but cleanup"));
            assert_eq!(stages.len(), 1);
            fs::set_permissions(&stages[0], fs::Permissions::from_mode(0o700)).unwrap();
        }
        Err(error) => panic!("unexpected result after publication: {error}"),
    }
}

#[test]
fn native_restore_rejects_corrupt_upper_time_limb() {
    let scratch = tempfile::tempdir().unwrap();
    let path = scratch.path().join("corrupt.bp");
    let adapter = Adios2CheckpointAdapter;
    adapter.write_manifest(&path, &manifest()).unwrap();
    let data = path.join("data.0");
    let mut bytes = fs::read(&data).unwrap();
    let offsets: Vec<_> = bytes
        .windows(8)
        .enumerate()
        .filter_map(|(index, value)| (value == HIGH.to_le_bytes()).then_some(index))
        .collect();
    assert_eq!(offsets.len(), 1);
    bytes[offsets[0]] ^= 1;
    fs::write(data, bytes).unwrap();
    assert_eq!(
        adapter.restore_manifest(&path),
        Err(ArrowError::InvalidNumber("checkpoint sha256"))
    );
}

#[test]
fn native_paths_reject_legacy_files_unrelated_directories_and_links() {
    let scratch = tempfile::tempdir().unwrap();
    let manifest = manifest();
    let adapter = Adios2CheckpointAdapter;
    let legacy = scratch.path().join("legacy.bp");
    // The older file stores a simpler subset of the record semantics.
    let batch = ParallelIoRecordBatch::from_dispatched(
        "legacy",
        [DispatchedEvent::new(
            EventId::new(0, 0),
            SimTime::from_ticks(1),
            0,
            0,
            None,
            EventKind::custom(1),
        )],
    )
    .unwrap();
    let old_format =
        CheckpointManifest::from_batch("legacy", CheckpointFormat::Adios2, &batch).unwrap();
    old_format.write_contract_file(&legacy).unwrap();
    assert_eq!(
        CheckpointManifest::read_contract_file(&legacy).unwrap(),
        old_format
    );
    assert!(matches!(
        adapter.restore_manifest(&legacy),
        Err(ArrowError::InvalidCheckpointFormat { .. })
    ));
    assert!(adapter.write_manifest(&legacy, &manifest).is_err());
    assert_eq!(
        CheckpointManifest::read_contract_file(&legacy).unwrap(),
        old_format
    );

    let unrelated = scratch.path().join("unrelated");
    fs::create_dir(&unrelated).unwrap();
    fs::write(unrelated.join("proof"), b"unrelated").unwrap();
    assert!(adapter.write_manifest(&unrelated, &manifest).is_err());
    assert_eq!(fs::read(unrelated.join("proof")).unwrap(), b"unrelated");

    let original = scratch.path().join("target.bp");
    adapter.write_manifest(&original, &manifest).unwrap();
    let link = scratch.path().join("link.bp");
    symlink(&original, &link).unwrap();
    let trailing_slash = format!("{}/", link.display());
    assert!(adapter.restore_manifest(&trailing_slash).is_err());
    assert!(adapter.write_manifest(&trailing_slash, &manifest).is_err());
    assert!(fs::symlink_metadata(link).unwrap().is_symlink());
    assert_eq!(
        adapter.restore_manifest(&original).unwrap(),
        manifest.restore().unwrap()
    );

    let member = original.join("data.0");
    let external = scratch.path().join("data");
    fs::rename(&member, &external).unwrap();
    symlink(&external, &member).unwrap();
    assert!(matches!(
        adapter.restore_manifest(&original),
        Err(ArrowError::InvalidCheckpointFormat { .. })
    ));
}

#[test]
fn native_restore_rejects_bad_framing_and_wrong_type() {
    const SCHEMA: &[u8] = b"kairo-ecs-adios2-bp5-checkpoint-v1";
    let scratch = tempfile::tempdir().unwrap();
    let bad_shape = scratch.path().join("bad-shape.bp");
    kairo_ecs_adios2::Session::acquire()
        .unwrap()
        .write(
            &bad_shape,
            &[
                (c"schema", SCHEMA.to_vec().into()),
                (c"counts", vec![1_u64].into()),
            ],
        )
        .unwrap();
    assert!(matches!(
        Adios2CheckpointAdapter.restore_manifest(&bad_shape),
        Err(ArrowError::Io(reason)) if reason.contains("column length")
    ));
    let wrong_type = scratch.path().join("wrong-type.bp");
    kairo_ecs_adios2::Session::acquire()
        .unwrap()
        .write(
            &wrong_type,
            &[
                (c"schema", SCHEMA.to_vec().into()),
                (c"counts", vec![0_i32; 6].into()),
            ],
        )
        .unwrap();
    assert!(matches!(
        Adios2CheckpointAdapter.restore_manifest(&wrong_type),
        Err(ArrowError::Io(reason)) if reason.contains("declared type")
    ));
}

fn bpls(path: &Path, variable: &str) -> Output {
    Command::new(std::env::var_os("KAIROS_BPLS").expect("select qualified ADIOS2 bpls"))
        .args(["-ld", "-n", "20"])
        .arg(path)
        .arg(variable)
        .output()
        .unwrap()
}

#[test]
#[ignore = "requires independently installed ADIOS2 bpls"]
fn bpls_reads_unequal_time_limbs_and_signed_priority() {
    let scratch = tempfile::tempdir().unwrap();
    let path = scratch.path().join("vendor.bp");
    Adios2CheckpointAdapter
        .write_manifest(&path, &manifest())
        .unwrap();
    for (column, kind, expected) in [
        ("time_ticks_hi", "uint64_t", HIGH.to_string()),
        ("time_ticks_lo", "uint64_t", LOW.to_string()),
        ("priority", "int32_t", "-7".to_string()),
    ] {
        let output = bpls(&path, column);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.contains(kind) && stdout.contains(column));
        assert!(stdout.split_whitespace().any(|value| value == expected));
        println!("{stdout}");
    }
    let legacy = scratch.path().join("labelled.bp");
    fs::write(&legacy, manifest().to_contract_bytes()).unwrap();
    assert!(!bpls(&legacy, "time_ticks_hi").status.success());
}
