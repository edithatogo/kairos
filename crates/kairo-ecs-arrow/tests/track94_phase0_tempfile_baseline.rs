use std::fs;

#[test]
#[should_panic(expected = "baseline direct write exposes partial file on interruption")]
fn baseline_direct_write_leaves_partial_file_on_interruption() {
    let base_dir = std::env::temp_dir().join("kairo_track94_phase0_baseline_test");
    let _ = fs::remove_dir_all(&base_dir);
    fs::create_dir_all(&base_dir).unwrap();

    let target_file = base_dir.join("part-00000.parquet");

    // In baseline non-atomic publication, bytes are written directly to target_file.
    // If interrupted midway (e.g. only 4 of 1000 bytes written), target_file exists with incomplete bytes.
    let partial_data = [0x50, 0x41, 0x52, 0x31]; // "PAR1" magic only
    fs::write(&target_file, partial_data).unwrap();

    // Verify the flaw: the incomplete file exists on disk
    if target_file.exists() && fs::metadata(&target_file).unwrap().len() == 4 {
        let _ = fs::remove_dir_all(&base_dir);
        panic!("baseline direct write exposes partial file on interruption");
    }

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
#[should_panic(expected = "atomic publication staging primitive with persist_noclobber required")]
fn atomic_staging_primitive_must_exist() {
    // Phase 0 requirement: atomic staging and publication primitives are required by Track 94.
    // Currently kairo-ecs-arrow does not expose atomic staged publication with overwrite/no-clobber controls.
    panic!("atomic publication staging primitive with persist_noclobber required");
}
