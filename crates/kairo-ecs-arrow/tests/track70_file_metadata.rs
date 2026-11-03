#![cfg(feature = "polars-native")]
use kairo_ecs_arrow::polars_interop as io;
use polars::prelude::*;
use std::collections::BTreeMap;
use std::io::Cursor;
fn frame() -> DataFrame {
    df!("id"=>[u64::MAX,0],"label"=>["x,\"y\nline","雪"],"value"=>[None,Some(1.5f64)]).unwrap()
}
fn metadata() -> BTreeMap<String, String> {
    [
        ("careops.schema".into(), "careops.tests.files.v2".into()),
        ("careops.run_id".into(), "metadata-run".into()),
        ("careops.time_unit".into(), "second".into()),
    ]
    .into_iter()
    .collect()
}
fn parquet_metadata(bytes: &[u8]) -> BTreeMap<String, String> {
    ParquetReader::new(Cursor::new(bytes))
        .get_metadata()
        .unwrap()
        .key_value_metadata
        .as_ref()
        .unwrap()
        .iter()
        .filter_map(|kv| kv.value.as_ref().map(|v| (kv.key.clone(), v.clone())))
        .collect()
}
#[test]
fn parquet_file_requires_declared_provenance_metadata() {
    let mut bytes = Vec::new();
    io::write_parquet_zstd_with_metadata(&mut frame(), &mut bytes, &metadata()).unwrap();
    let observed = parquet_metadata(&bytes);
    for (key, value) in metadata() {
        assert_eq!(observed.get(&key), Some(&value));
    }
}
#[test]
fn ipc_stream_requires_declared_schema_metadata() {
    let mut bytes = Vec::new();
    io::write_ipc_stream_with_metadata(&mut frame(), &mut bytes, &metadata()).unwrap();
    let mut reader = IpcStreamReader::new(Cursor::new(bytes));
    let observed = reader.custom_metadata().unwrap().unwrap_or_default();
    for (key, value) in metadata() {
        assert_eq!(
            observed.get(key.as_str()).map(|v| v.as_str()),
            Some(value.as_str())
        );
    }
}
#[test]
fn original_plain_writers_preserve_populated_values() {
    let df = frame();
    let mut bytes = Vec::new();
    io::write_parquet_zstd(&mut df.clone(), &mut bytes).unwrap();
    assert!(io::read_parquet(Cursor::new(bytes))
        .unwrap()
        .equals_missing(&df));
    let mut bytes = Vec::new();
    io::write_ipc_stream(&mut df.clone(), &mut bytes).unwrap();
    assert!(io::read_ipc_stream(Cursor::new(bytes))
        .unwrap()
        .equals_missing(&df));
}

use kairo_ecs_arrow::run_output::{validate_run_output, RunOutput, RunOutputConfig};
use kairo_ecs_arrow::{EventLogRecord, EventStatus, SCHEMA_VERSION_V2};
use kairo_ecs_types::EventId;
use std::io::Write;
use std::num::NonZeroUsize;
mod support;
fn config() -> RunOutputConfig {
    RunOutputConfig {
        run_id: "metadata-run".into(),
        replication_id: 0,
        seed: 7,
        time_scale: "second".into(),
        effective_input_hash: "ab".repeat(32),
        raw_input_hash: None,
        max_rows_per_part: NonZeroUsize::new(1).unwrap(),
    }
}
fn records() -> Vec<EventLogRecord> {
    (1..=2)
        .map(|i| EventLogRecord {
            schema_version: SCHEMA_VERSION_V2,
            run_id: "metadata-run".into(),
            event_id: EventId::new(i, 0),
            entity_id: None,
            time_ticks: u128::from(i),
            time_scale: "second".into(),
            priority: 0,
            sequence: i,
            event_kind: "capture".into(),
            status: EventStatus::Dispatched,
            payload_ref: None,
        })
        .collect()
}
#[test]
fn empty_metadata_matches_original_library_encoding_bytes() {
    let df = frame();
    let mut original = Vec::new();
    ParquetWriter::new(&mut original)
        .with_compression(ParquetCompression::Zstd(None))
        .finish(&mut df.clone())
        .unwrap();
    let mut updated = Vec::new();
    io::write_parquet_zstd(&mut df.clone(), &mut updated).unwrap();
    assert_eq!(original, updated);
    updated.clear();
    io::write_parquet_zstd_with_metadata(&mut df.clone(), &mut updated, &BTreeMap::new()).unwrap();
    assert_eq!(original, updated);
    original.clear();
    IpcStreamWriter::new(&mut original)
        .finish(&mut df.clone())
        .unwrap();
    updated.clear();
    io::write_ipc_stream_with_metadata(&mut df.clone(), &mut updated, &BTreeMap::new()).unwrap();
    assert_eq!(original, updated);
}
#[test]
fn zero_row_tables_preserve_typed_schema_and_file_metadata() {
    let df = frame().clear();
    let mut bytes = Vec::new();
    io::write_parquet_zstd_with_metadata(&mut df.clone(), &mut bytes, &metadata()).unwrap();
    assert_eq!(
        parquet_metadata(&bytes).get("careops.schema"),
        metadata().get("careops.schema")
    );
    assert!(io::read_parquet(Cursor::new(bytes))
        .unwrap()
        .equals_missing(&df));
    let mut bytes = Vec::new();
    io::write_ipc_stream_with_metadata(&mut df.clone(), &mut bytes, &metadata()).unwrap();
    let mut reader = IpcStreamReader::new(Cursor::new(bytes));
    assert!(reader.custom_metadata().unwrap().is_some());
    assert!(reader.finish().unwrap().equals_missing(&df));
}
#[test]
fn reserved_empty_and_oversized_metadata_refuse_before_output_or_empty_batch_mutation() {
    for bad in [
        BTreeMap::from([("ARROW:schema".into(), "forged".into())]),
        BTreeMap::from([("".into(), "invalid".into())]),
        BTreeMap::from([("large".into(), "a".repeat(io::MAX_FILE_METADATA_BYTES))]),
    ] {
        let mut bytes = vec![42u8];
        assert!(
            io::write_parquet_zstd_with_metadata(&mut frame().clear(), &mut bytes, &bad).is_err()
        );
        assert_eq!(bytes, [42]);
        assert!(
            io::write_ipc_stream_with_metadata(&mut frame().clear(), &mut bytes, &bad).is_err()
        );
        assert_eq!(bytes, [42]);
        let root = support::protected_tempdir().unwrap();
        let mut output = RunOutput::create(config(), root.path()).unwrap();
        assert!(output.write_batch_with_metadata(&[], &bad).is_err());
        assert!(output
            .write_batch_to_with_metadata(&[], &mut bytes, &bad)
            .is_err());
        assert!(output.parts().is_empty());
        assert!(output.checkpoint_boundary().is_none());
        assert_eq!(bytes, [42]);
    }
}
struct FullAfter {
    bytes: Vec<u8>,
    limit: usize,
}
impl Write for FullAfter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let count = bytes.len().min(self.limit.saturating_sub(self.bytes.len()));
        if count == 0 {
            return Err(std::io::Error::from(std::io::ErrorKind::StorageFull));
        }
        self.bytes.extend_from_slice(&bytes[..count]);
        Ok(count)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
#[test]
fn actual_partial_sink_failure_with_metadata_records_no_acknowledgment() {
    let root = support::protected_tempdir().unwrap();
    let mut output = RunOutput::create(config(), root.path()).unwrap();
    let mut sink = FullAfter {
        bytes: Vec::new(),
        limit: 128,
    };
    assert!(output
        .write_batch_to_with_metadata(&records(), &mut sink, &metadata())
        .is_err());
    assert_eq!(sink.bytes.len(), 128);
    assert!(output.parts().is_empty());
    assert!(output.checkpoint_boundary().is_none());
    let mut retry = Vec::new();
    let receipt = output
        .write_batch_to_with_metadata(&records()[..1], &mut retry, &metadata())
        .unwrap();
    assert_eq!(receipt[0].file_hash, format!("{:016x}", fnv(&retry)));
    assert_eq!(
        parquet_metadata(&retry).get("careops.schema"),
        metadata().get("careops.schema")
    );
}
#[test]
fn durable_receipts_attest_actual_metadata_and_explicit_independent_fixture() {
    let owned = support::protected_tempdir().unwrap();
    let supplied = std::env::var_os("KAIROS_METADATA_FIXTURE_ROOT").map(std::path::PathBuf::from);
    let root = supplied.as_deref().unwrap_or(owned.path());
    if supplied.is_some() {
        let mut builder = std::fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder
            .create(root)
            .expect("fresh explicit owned fixture directory");
        let mut file = std::fs::File::create(root.join("table.parquet")).unwrap();
        io::write_parquet_zstd_with_metadata(&mut frame(), &mut file, &metadata()).unwrap();
        let mut file = std::fs::File::create(root.join("table.arrow")).unwrap();
        io::write_ipc_stream_with_metadata(&mut frame(), &mut file, &metadata()).unwrap();
    }
    let mut output = RunOutput::create(config(), root).unwrap();
    let mut declared = metadata();
    declared.insert("careops.schema".into(), "kairo_ecs.event_log.v2".into());
    let receipts = output
        .write_batch_with_metadata(&records(), &declared)
        .unwrap();
    assert_eq!(receipts.len(), 2);
    assert_eq!(output.checkpoint_boundary().unwrap().sequence_watermark, 2);
    let manifest = output.complete().unwrap();
    assert_eq!(manifest.parts, receipts);
    let directory = root.join("metadata-run-rep0");
    validate_run_output(&directory).unwrap();
    for part in receipts {
        let bytes = std::fs::read(directory.join(part.file_name)).unwrap();
        assert_eq!(part.file_hash, format!("{:016x}", fnv(&bytes)));
        for (key, value) in &declared {
            assert_eq!(parquet_metadata(&bytes).get(key), Some(value));
        }
    }
}

// Independent standard FNV-1a64 oracle; publication's algorithm label remains
// noncryptographic. The known vector guards this test-side implementation.
fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}
#[test]
fn metadata_bytes_reach_the_actual_bounded_writer() {
    assert_eq!(format!("{:016x}", fnv(b"hello")), "a430d84680aabd0b");
    let mut plain = Vec::new();
    io::write_parquet_zstd(&mut frame(), &mut plain).unwrap();
    let mut sink = FullAfter {
        bytes: Vec::new(),
        limit: plain.len(),
    };
    assert!(io::write_parquet_zstd_with_metadata(&mut frame(), &mut sink, &metadata()).is_err());
    assert_eq!(sink.bytes.len(), plain.len());
    let mut plain = Vec::new();
    io::write_ipc_stream(&mut frame(), &mut plain).unwrap();
    let mut sink = FullAfter {
        bytes: Vec::new(),
        limit: plain.len(),
    };
    assert!(io::write_ipc_stream_with_metadata(&mut frame(), &mut sink, &metadata()).is_err());
    assert_eq!(sink.bytes.len(), plain.len());
}
