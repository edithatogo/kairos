//! Preparatory physical-v2 transport adapter for actual normalized C1.3 rows.
//!
//! Set `KAIROS_C13_PHYSICAL_INDIR` to a directory containing one IPC file,
//! IPC stream, and Parquet file per physical record type. Without it, the
//! committed physical-v2 fixture set provides a bounded transport smoke test.
//! This test establishes transport behavior only; it does not run or qualify
//! the C1.3 normalization, sorting, or validation pipeline.
#![cfg(all(feature = "ipc", feature = "parquet"))]

use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use arrow_array::RecordBatch;
use arrow_schema::SchemaRef;
use kairo_ecs_arrow_io::{
    read_ipc_file, read_ipc_stream, read_parquet, write_ipc_file, write_ipc_stream, write_parquet,
    IoLimits,
};

#[path = "support/calibration_physical_schema_v2.rs"]
mod frozen;

const FORMATS: [&str; 3] = ["ipc_file", "ipc_stream", "parquet"];
const OUTPUT_BATCH_SIZES: [usize; 3] = [1, 2, 3];

fn input_dir() -> (PathBuf, bool) {
    match env::var_os("KAIROS_C13_PHYSICAL_INDIR") {
        Some(path) => (PathBuf::from(path), false),
        None => (
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/calibration_physical_v2"),
            true,
        ),
    }
}

fn output_dir() -> PathBuf {
    env::var_os("KAIROS_C13_PHYSICAL_OUTDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../.artifacts/c13-transport/rust-output")
        })
}

fn limits(batch_rows: usize) -> IoLimits {
    IoLimits {
        max_batch_rows: batch_rows,
        max_total_rows: 1_000_000,
        max_batches: 1_000_000,
        ..IoLimits::default()
    }
}

fn input_path(directory: &Path, baseline: bool, record_type: &str, format: &str) -> PathBuf {
    let stem = if baseline {
        match record_type {
            "trace_event.v1" => "trace",
            "trace_exclusion.v1" => "exclusion",
            "outcome_observation.v1" => "outcome",
            _ => unreachable!("the frozen table list is exhaustive"),
        }
    } else {
        record_type
            .split('.')
            .next()
            .expect("record type has a stem")
    };
    directory.join(format!("{stem}.{format}"))
}

fn read(path: &Path, format: &str, schema: SchemaRef) -> Vec<RecordBatch> {
    let bytes = fs::read(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    match format {
        "ipc_file" => read_ipc_file(&bytes, schema, limits(100_000)),
        "ipc_stream" => read_ipc_stream(&bytes, schema, limits(100_000)),
        "parquet" => read_parquet(&bytes, schema, limits(100_000)),
        _ => unreachable!("known physical format"),
    }
    .unwrap_or_else(|error| panic!("decode {}: {error}", path.display()))
}

fn row_batches(batches: &[RecordBatch]) -> Vec<RecordBatch> {
    batches
        .iter()
        .flat_map(|batch| (0..batch.num_rows()).map(|index| batch.slice(index, 1)))
        .collect()
}

// Compare a transport-neutral multiset. Arrow array equality ignores backing
// values under nulls, which IPC/Parquet are free to canonicalize differently.
fn assert_same_rows(left: &[RecordBatch], right: &[RecordBatch], context: &str) {
    let mut unmatched = row_batches(right);
    for row in row_batches(left) {
        let Some(index) = unmatched.iter().position(|candidate| candidate == &row) else {
            panic!("row missing after transport: {context}");
        };
        unmatched.swap_remove(index);
    }
    assert!(unmatched.is_empty(), "unexpected extra rows: {context}");
}

fn write_and_read(
    directory: &Path,
    schema: &SchemaRef,
    batches: &[RecordBatch],
    format: &str,
    batch_size: usize,
) -> Vec<RecordBatch> {
    let io_limits = limits(batch_size);
    let bytes = match format {
        "ipc_file" => write_ipc_file(Arc::clone(schema), batches, io_limits),
        "ipc_stream" => write_ipc_stream(Arc::clone(schema), batches, io_limits),
        "parquet" => write_parquet(Arc::clone(schema), batches, io_limits),
        _ => unreachable!("known physical format"),
    }
    .expect("bounded Rust physical writer succeeds");

    let record_stem = schema
        .metadata()
        .get("record_type")
        .expect("frozen physical schema has record_type metadata");
    let path = directory.join(format!(
        "{record_stem}.reversed.limit-{batch_size}.{format}"
    ));
    fs::write(&path, &bytes).unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
    read(&path, format, Arc::clone(schema))
}

#[test]
fn actual_physical_tables_roundtrip_across_formats_orders_and_layouts() {
    let (input, baseline) = input_dir();
    let output = output_dir();
    fs::create_dir_all(&output).expect("create owned transport output directory");

    for (_, record_type) in frozen::TABLES {
        let schema = frozen::schema(record_type);
        let mut source_layouts = Vec::new();
        for format in FORMATS {
            let path = input_path(&input, baseline, record_type, format);
            let batches = read(&path, format, Arc::clone(&schema));
            source_layouts.push((format, batches));
        }

        for (format, batches) in &source_layouts {
            assert_same_rows(
                batches,
                &source_layouts[0].1,
                &format!("input payload mismatch for {record_type} in {format}"),
            );
        }

        let mut reversed = row_batches(&source_layouts[0].1);
        reversed.reverse();
        for batch_size in OUTPUT_BATCH_SIZES {
            // Keep each row as a RecordBatch so this adapter needs no extra
            // Arrow dependency. The caller-selected limit varies; Parquet's
            // writer uses it as the row-group cap. IPC input-layout variation
            // is exercised by supplying differently batched source files.
            let physical_batches = reversed.clone();
            for format in FORMATS {
                let decoded =
                    write_and_read(&output, &schema, &physical_batches, format, batch_size);
                assert_same_rows(
                    &decoded,
                    &source_layouts[0].1,
                    &format!("Rust {format} writer/reader changed {record_type} payload at limit {batch_size}")
                );
            }
        }
    }
}
