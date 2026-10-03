#![cfg(all(feature = "ipc", feature = "parquet"))]

use arrow_array::builder::FixedSizeBinaryBuilder;
use arrow_array::{Array, FixedSizeBinaryArray, Int32Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use kairo_ecs_arrow_io::{
    IoLimits, read_ipc_file, read_ipc_stream, read_parquet, write_ipc_file, write_ipc_stream,
    write_parquet,
};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

const U128_MAX: u128 = u128::MAX;

#[derive(Clone)]
struct Row {
    signed: i32,
    text: Option<&'static str>,
    fixed: Option<[u8; 16]>,
    ticks: u128,
    label: &'static str,
}

fn metadata(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect()
}

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/interop_v1")
        .join(name);
    fs::read(path).expect("generate the reviewed PyArrow fixture bundle first")
}

fn expected_schema() -> SchemaRef {
    let fields = vec![
        Field::new("signed_i32", DataType::Int32, false),
        Field::new("nullable_text", DataType::Utf8, true),
        Field::new("nullable_fixed16", DataType::FixedSizeBinary(16), true)
            .with_metadata(metadata(&[("role", "opaque-synthetic-bytes")])),
        Field::new("ticks_u128_le", DataType::FixedSizeBinary(16), false).with_metadata(metadata(
            &[("unit", "ns"), ("encoding", "little-endian-u128")],
        )),
        Field::new("synthetic_label", DataType::Utf8, false),
    ];
    Arc::new(Schema::new_with_metadata(
        fields,
        metadata(&[
            ("kairos.fixture.name", "interop-probe.v1"),
            ("kairos.fixture.synthetic", "true"),
            ("kairos.fixture.contract", "recordbatch-v1"),
        ]),
    ))
}

fn empty_schema() -> SchemaRef {
    Arc::new(Schema::new_with_metadata(
        Vec::<Field>::new(),
        metadata(&[
            ("kairos.fixture.name", "interop-probe.v1"),
            ("kairos.fixture.synthetic", "true"),
            ("kairos.fixture.contract", "recordbatch-v1"),
        ]),
    ))
}

fn io_limits() -> IoLimits {
    IoLimits {
        max_input_bytes: 1 << 20,
        max_output_bytes: 1 << 20,
        max_batch_rows: 2,
        max_total_rows: 16,
        max_batches: 8,
        max_columns: 8,
    }
}

fn fixed_array(values: &[Option<[u8; 16]>]) -> FixedSizeBinaryArray {
    let mut builder = FixedSizeBinaryBuilder::with_capacity(values.len(), 16);
    for value in values {
        match value {
            Some(bytes) => builder.append_value(bytes).expect("fixed-size value"),
            None => builder.append_null(),
        }
    }
    builder.finish()
}

fn ticks_array(rows: &[Row]) -> FixedSizeBinaryArray {
    let mut builder = FixedSizeBinaryBuilder::with_capacity(rows.len(), 16);
    for row in rows {
        builder
            .append_value(&row.ticks.to_le_bytes())
            .expect("tick value is exactly sixteen bytes");
    }
    builder.finish()
}

fn record_batch(schema: SchemaRef, rows: &[Row]) -> RecordBatch {
    let signed = Int32Array::from(rows.iter().map(|row| row.signed).collect::<Vec<_>>());
    let text = StringArray::from(rows.iter().map(|row| row.text).collect::<Vec<_>>());
    let fixed = fixed_array(&rows.iter().map(|row| row.fixed).collect::<Vec<_>>());
    let ticks = ticks_array(rows);
    let labels = StringArray::from(rows.iter().map(|row| Some(row.label)).collect::<Vec<_>>());
    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(signed),
            Arc::new(text),
            Arc::new(fixed),
            Arc::new(ticks),
            Arc::new(labels),
        ],
    )
    .expect("fixture batch matches frozen schema")
}

fn expected_rows() -> Vec<Row> {
    vec![
        Row {
            signed: -7,
            text: None,
            fixed: None,
            ticks: 0,
            label: "synthetic-a",
        },
        Row {
            signed: 0,
            text: Some(""),
            fixed: Some([0; 16]),
            ticks: 1,
            label: "é-二",
        },
        Row {
            signed: 19,
            text: Some("München/東京"),
            fixed: Some(std::array::from_fn(|index| index as u8)),
            ticks: U128_MAX,
            label: "synthetic-c",
        },
    ]
}

fn batches() -> Vec<RecordBatch> {
    let schema = expected_schema();
    let rows = expected_rows();
    vec![
        record_batch(Arc::clone(&schema), &rows[..2]),
        record_batch(schema, &rows[2..]),
    ]
}

fn assert_rows(batches: &[RecordBatch], expected_batch_count: Option<usize>) {
    if let Some(expected) = expected_batch_count {
        assert_eq!(batches.len(), expected, "IPC framing batch count");
    }
    let schema = expected_schema();
    let mut signed_values = Vec::new();
    let mut text_values = Vec::new();
    let mut fixed_values = Vec::new();
    let mut ticks_values = Vec::new();
    let mut labels = Vec::new();
    let mut rows_seen = 0;
    for batch in batches {
        assert_eq!(batch.schema().as_ref(), schema.as_ref());
        let signed = batch
            .column(0)
            .as_any()
            .downcast_ref::<Int32Array>()
            .expect("signed Int32");
        let text = batch
            .column(1)
            .as_any()
            .downcast_ref::<StringArray>()
            .expect("nullable UTF-8");
        let fixed = batch
            .column(2)
            .as_any()
            .downcast_ref::<FixedSizeBinaryArray>()
            .expect("nullable fixed 16 bytes");
        let ticks = batch
            .column(3)
            .as_any()
            .downcast_ref::<FixedSizeBinaryArray>()
            .expect("required fixed 16-byte ticks");
        let label = batch
            .column(4)
            .as_any()
            .downcast_ref::<StringArray>()
            .expect("required UTF-8 label");
        for row in 0..batch.num_rows() {
            signed_values.push(signed.value(row));
            text_values.push(if text.is_null(row) {
                None
            } else {
                Some(text.value(row).to_owned())
            });
            fixed_values.push(if fixed.is_null(row) {
                None
            } else {
                Some(fixed.value(row).to_vec())
            });
            ticks_values.push(u128::from_le_bytes(
                ticks
                    .value(row)
                    .try_into()
                    .expect("exactly sixteen tick bytes"),
            ));
            labels.push(label.value(row).to_owned());
            rows_seen += 1;
        }
    }
    assert_eq!(rows_seen, 3);
    assert_eq!(signed_values, vec![-7, 0, 19]);
    assert_eq!(
        text_values,
        vec![None, Some(String::new()), Some("München/東京".into())]
    );
    assert_eq!(
        fixed_values,
        vec![None, Some(vec![0; 16]), Some((0u8..16).collect())]
    );
    assert_eq!(ticks_values, vec![0, 1, U128_MAX]);
    assert_eq!(labels, vec!["synthetic-a", "é-二", "synthetic-c"]);
}

#[test]
fn pyarrow_fixtures_are_read_and_rust_transports_are_written() {
    let schema = expected_schema();
    let file = read_ipc_file(&fixture("mixed.ipc_file"), Arc::clone(&schema), io_limits())
        .expect("read independent PyArrow IPC file");
    assert_rows(&file, Some(2));

    let stream = read_ipc_stream(
        &fixture("mixed.ipc_stream"),
        Arc::clone(&schema),
        io_limits(),
    )
    .expect("read independent PyArrow IPC stream");
    assert_rows(&stream, Some(2));

    let parquet = read_parquet(&fixture("mixed.parquet"), Arc::clone(&schema), io_limits())
        .expect("read independent PyArrow Parquet");
    assert_rows(&parquet, None);

    assert!(
        read_ipc_file(
            &fixture("typed_empty.ipc_file"),
            Arc::clone(&schema),
            io_limits(),
        )
        .expect("typed IPC file with no batches")
        .is_empty()
    );
    assert!(
        read_ipc_stream(
            &fixture("typed_empty.ipc_stream"),
            Arc::clone(&schema),
            io_limits(),
        )
        .expect("typed IPC stream with no batches")
        .is_empty()
    );
    let empty_parquet = read_parquet(
        &fixture("typed_empty.parquet"),
        Arc::clone(&schema),
        io_limits(),
    )
    .expect("typed empty Parquet");
    assert!(empty_parquet.iter().all(|batch| batch.num_rows() == 0));
    assert!(
        empty_parquet
            .iter()
            .all(|batch| batch.schema().as_ref() == schema.as_ref())
    );

    let zero_row_file = read_ipc_file(
        &fixture("typed_zero_row_batch.ipc_file"),
        Arc::clone(&schema),
        io_limits(),
    )
    .expect("typed IPC file with one zero-row batch");
    assert_eq!(zero_row_file.len(), 1);
    assert_eq!(zero_row_file[0].num_rows(), 0);
    let zero_row_stream = read_ipc_stream(
        &fixture("typed_zero_row_batch.ipc_stream"),
        Arc::clone(&schema),
        io_limits(),
    )
    .expect("typed IPC stream with one zero-row batch");
    assert_eq!(zero_row_stream.len(), 1);
    assert_eq!(zero_row_stream[0].num_rows(), 0);

    let no_fields = empty_schema();
    assert!(
        read_ipc_file(
            &fixture("empty_schema.ipc_file"),
            Arc::clone(&no_fields),
            io_limits(),
        )
        .expect("empty-schema IPC file")
        .is_empty()
    );
    assert!(
        read_ipc_stream(&fixture("empty_schema.ipc_stream"), no_fields, io_limits(),)
            .expect("empty-schema IPC stream")
            .is_empty()
    );

    // Same field types with changed schema metadata must be rejected before rows.
    let mismatched = Arc::new(Schema::new_with_metadata(
        schema.fields().iter().cloned().collect::<Vec<_>>(),
        metadata(&[
            ("kairos.fixture.name", "interop-probe.v1"),
            ("kairos.fixture.synthetic", "true"),
            ("kairos.fixture.contract", "wrong-version"),
        ]),
    ));
    assert!(read_ipc_file(&fixture("mixed.ipc_file"), mismatched, io_limits(),).is_err());

    let output_dir = PathBuf::from(
        std::env::var_os("KAIROS_INTEROP_OUTDIR")
            .expect("set KAIROS_INTEROP_OUTDIR to the reviewed Rust output directory"),
    );
    fs::create_dir_all(&output_dir).expect("create reviewed Rust output directory");
    let expected = batches();
    fs::write(
        output_dir.join("rust.ipc_file"),
        write_ipc_file(Arc::clone(&schema), &expected, io_limits()).expect("write Rust IPC file"),
    )
    .expect("save Rust IPC file");
    fs::write(
        output_dir.join("rust.ipc_stream"),
        write_ipc_stream(Arc::clone(&schema), &expected, io_limits())
            .expect("write Rust IPC stream"),
    )
    .expect("save Rust IPC stream");
    fs::write(
        output_dir.join("rust.parquet"),
        write_parquet(schema, &expected, io_limits()).expect("write Rust Parquet"),
    )
    .expect("save Rust Parquet");
}
