use kairo_ecs_arrow::{atomic_write_file, ArrowNativeSchema, ParquetZstdCheckpointWriter};

#[test]
fn bare_relative_publication_syncs_current_directory() {
    const CHILD: &str = "KAIROS_RELATIVE_PUBLICATION_CHILD";
    if std::env::var_os(CHILD).is_some() {
        atomic_write_file("trace.parquet", b"first", false).unwrap();
        assert_eq!(std::fs::read("trace.parquet").unwrap(), b"first");
        assert!(atomic_write_file("trace.parquet", b"clobber", false).is_err());
        assert_eq!(std::fs::read("trace.parquet").unwrap(), b"first");
        atomic_write_file("trace.parquet", b"replacement", true).unwrap();
        assert_eq!(std::fs::read("trace.parquet").unwrap(), b"replacement");
        let schema = ArrowNativeSchema::from_event_log_fields();
        ParquetZstdCheckpointWriter::write_checkpoint_file(
            "checkpoint.parquet",
            &schema,
            &[],
            false,
        )
        .unwrap();
        assert_eq!(
            std::fs::read("checkpoint.parquet").unwrap(),
            ParquetZstdCheckpointWriter::write_checkpoint(&schema, &[])
        );
        return;
    }
    // Isolate the current directory in a child instead of changing process-global
    // state underneath concurrently executing tests.
    let directory = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "bare_relative_publication_syncs_current_directory",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .current_dir(directory.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
