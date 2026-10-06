#![cfg(all(feature = "ipc", feature = "parquet"))]

use arrow_array::{Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use kairo_ecs_arrow_io::{
    read_ipc_file, read_ipc_stream, read_parquet, write_ipc_file, write_ipc_stream, write_parquet,
    IoLimits,
};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

const META: [(&str, &str); 5] = [
    ("kairos.fixture.contract", "recordbatch-v1"),
    ("kairos.fixture.name", "calibration-raw-source.v1"),
    (
        "kairos.fixture.precision-scope",
        "reported-source-precision-only; not inferred from clock text",
    ),
    (
        "kairos.fixture.schema-scope",
        "raw-source-input-only; not normalized calibration schema",
    ),
    ("kairos.fixture.synthetic", "true"),
];

fn metadata(entries: &[(&str, &str)]) -> HashMap<String, String> {
    entries
        .iter()
        .map(|(k, v)| ((*k).into(), (*v).into()))
        .collect()
}

fn schema(profile: &str) -> SchemaRef {
    let fields: Vec<Field> = match profile {
        "wide" => vec![
            Field::new("source_row_id", DataType::Utf8, false),
            Field::new("encounter_id", DataType::Utf8, false),
            Field::new("event_occurred_at", DataType::Utf8, true),
            Field::new("event_recorded_at", DataType::Utf8, true),
            Field::new("triage_at", DataType::Utf8, true),
            Field::new("departure_admin_at", DataType::Utf8, true),
            Field::new("departure_physical_at", DataType::Utf8, true),
            Field::new("timezone_name", DataType::Utf8, true),
            Field::new("naive_local_at", DataType::Utf8, true),
            Field::new("occurrence_precision", DataType::Utf8, true),
            Field::new("triage_precision", DataType::Utf8, true),
            Field::new("canonical_rank", DataType::Utf8, false),
            Field::new("relative_elapsed_ns", DataType::Utf8, true),
            Field::new("cohort_denominator", DataType::Utf8, true),
            Field::new("triage_denominator", DataType::Utf8, true),
            Field::new("future_outcome", DataType::Utf8, true),
            Field::new("duplicate_external_id", DataType::Utf8, true),
        ],
        "long" => vec![
            Field::new("source_row_id", DataType::Utf8, false),
            Field::new("source_entity", DataType::Utf8, false),
            Field::new("field_name", DataType::Utf8, false),
            Field::new("field_value", DataType::Utf8, true),
        ],
        _ => panic!("unknown fixture profile"),
    };
    Arc::new(Schema::new_with_metadata(fields, metadata(&META)))
}

fn limits() -> IoLimits {
    IoLimits {
        max_input_bytes: 1 << 20,
        max_output_bytes: 1 << 20,
        max_batch_rows: 2,
        max_total_rows: 64,
        max_batches: 32,
        max_columns: 32,
    }
}

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/calibration_source_v1")
        .join(name);
    fs::read(path).expect("generate the independent PyArrow raw-source fixtures first")
}

fn assert_rows(batches: &[RecordBatch], expected: &[Vec<Option<&str>>]) {
    let mut actual = Vec::new();
    for batch in batches {
        for row in 0..batch.num_rows() {
            let mut values = Vec::new();
            for column in 0..batch.num_columns() {
                let array = batch
                    .column(column)
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .unwrap();
                values.push((!array.is_null(row)).then(|| array.value(row)));
            }
            actual.push(values);
        }
    }
    assert_eq!(actual, expected);
}

fn wide_rows() -> Vec<Vec<Option<&'static str>>> {
    vec![
        vec![
            Some("arrival-001"),
            Some("enc-001"),
            Some("2024-11-03T01:30:00.123456789123-04:00"),
            Some("2024-11-03T01:30:05.000000000001-04:00"),
            Some("2024-11-03T01:42:00-04:00"),
            Some("2024-11-03T03:10:00-05:00"),
            Some("2024-11-03T03:17:00-05:00"),
            Some("America/New_York"),
            Some("2024-11-03T01:30:00.123456789123"),
            Some("sub-nanosecond"),
            Some("second"),
            Some("18446744073709551616"),
            Some("18446744073709551616"),
            None,
            Some(""),
            Some("positive synthetic"),
            Some("dup-7"),
        ],
        vec![
            Some("arrival-002"),
            Some("enc-002"),
            Some("1900-01-01T00:00:00.000000001Z"),
            None,
            Some("2024-03-10T02:30:00"),
            Some("2024-03-10T02:31:00"),
            None,
            Some("America/New_York"),
            Some("2024-03-10T02:30:00"),
            Some("nanosecond"),
            Some("minute"),
            Some("7"),
            None,
            Some("0"),
            None,
            None,
            Some("dup-7"),
        ],
    ]
}

fn long_rows() -> Vec<Vec<Option<&'static str>>> {
    vec![
        vec![
            Some("arrival-001"),
            Some("arrival"),
            Some("source_row_id"),
            Some("arrival-001"),
        ],
        vec![
            Some("arrival-001"),
            Some("arrival"),
            Some("encounter_id"),
            Some("enc-001"),
        ],
        vec![
            Some("arrival-001"),
            Some("arrival"),
            Some("event_occurred_at"),
            Some("2024-11-03T01:30:00.123456789123-04:00"),
        ],
        vec![
            Some("arrival-001"),
            Some("arrival"),
            Some("event_recorded_at"),
            Some("2024-11-03T01:30:05.000000000001-04:00"),
        ],
        vec![
            Some("arrival-001"),
            Some("arrival"),
            Some("triage_at"),
            Some("2024-11-03T01:42:00-04:00"),
        ],
        vec![
            Some("arrival-001"),
            Some("arrival"),
            Some("departure_admin_at"),
            Some("2024-11-03T03:10:00-05:00"),
        ],
        vec![
            Some("arrival-001"),
            Some("arrival"),
            Some("departure_physical_at"),
            Some("2024-11-03T03:17:00-05:00"),
        ],
        vec![
            Some("arrival-001"),
            Some("arrival"),
            Some("timezone_name"),
            Some("America/New_York"),
        ],
        vec![
            Some("arrival-001"),
            Some("arrival"),
            Some("naive_local_at"),
            Some("2024-11-03T01:30:00.123456789123"),
        ],
        vec![
            Some("arrival-001"),
            Some("arrival"),
            Some("occurrence_precision"),
            Some("sub-nanosecond"),
        ],
        vec![
            Some("arrival-001"),
            Some("arrival"),
            Some("triage_precision"),
            Some("second"),
        ],
        vec![
            Some("arrival-001"),
            Some("arrival"),
            Some("canonical_rank"),
            Some("18446744073709551616"),
        ],
        vec![
            Some("arrival-001"),
            Some("arrival"),
            Some("relative_elapsed_ns"),
            Some("18446744073709551616"),
        ],
        vec![
            Some("arrival-001"),
            Some("arrival"),
            Some("cohort_denominator"),
            None,
        ],
        vec![
            Some("arrival-001"),
            Some("arrival"),
            Some("triage_denominator"),
            Some(""),
        ],
        vec![
            Some("arrival-001"),
            Some("arrival"),
            Some("future_outcome"),
            Some("positive synthetic"),
        ],
        vec![
            Some("arrival-001"),
            Some("arrival"),
            Some("duplicate_external_id"),
            Some("dup-7"),
        ],
        vec![
            Some("arrival-002"),
            Some("arrival"),
            Some("source_row_id"),
            Some("arrival-002"),
        ],
        vec![
            Some("arrival-002"),
            Some("arrival"),
            Some("encounter_id"),
            Some("enc-002"),
        ],
        vec![
            Some("arrival-002"),
            Some("arrival"),
            Some("event_occurred_at"),
            Some("1900-01-01T00:00:00.000000001Z"),
        ],
        vec![
            Some("arrival-002"),
            Some("arrival"),
            Some("event_recorded_at"),
            None,
        ],
        vec![
            Some("arrival-002"),
            Some("arrival"),
            Some("triage_at"),
            Some("2024-03-10T02:30:00"),
        ],
        vec![
            Some("arrival-002"),
            Some("arrival"),
            Some("departure_admin_at"),
            Some("2024-03-10T02:31:00"),
        ],
        vec![
            Some("arrival-002"),
            Some("arrival"),
            Some("departure_physical_at"),
            None,
        ],
        vec![
            Some("arrival-002"),
            Some("arrival"),
            Some("timezone_name"),
            Some("America/New_York"),
        ],
        vec![
            Some("arrival-002"),
            Some("arrival"),
            Some("naive_local_at"),
            Some("2024-03-10T02:30:00"),
        ],
        vec![
            Some("arrival-002"),
            Some("arrival"),
            Some("occurrence_precision"),
            Some("nanosecond"),
        ],
        vec![
            Some("arrival-002"),
            Some("arrival"),
            Some("triage_precision"),
            Some("minute"),
        ],
        vec![
            Some("arrival-002"),
            Some("arrival"),
            Some("canonical_rank"),
            Some("7"),
        ],
        vec![
            Some("arrival-002"),
            Some("arrival"),
            Some("relative_elapsed_ns"),
            None,
        ],
        vec![
            Some("arrival-002"),
            Some("arrival"),
            Some("cohort_denominator"),
            Some("0"),
        ],
        vec![
            Some("arrival-002"),
            Some("arrival"),
            Some("triage_denominator"),
            None,
        ],
        vec![
            Some("arrival-002"),
            Some("arrival"),
            Some("future_outcome"),
            None,
        ],
        vec![
            Some("arrival-002"),
            Some("arrival"),
            Some("duplicate_external_id"),
            Some("dup-7"),
        ],
    ]
}

fn assert_long_payload_matches_wide(
    wide: &[Vec<Option<&'static str>>],
    long: &[Vec<Option<&'static str>>],
) {
    const WIDE_FIELD_NAMES: [&str; 17] = [
        "source_row_id",
        "encounter_id",
        "event_occurred_at",
        "event_recorded_at",
        "triage_at",
        "departure_admin_at",
        "departure_physical_at",
        "timezone_name",
        "naive_local_at",
        "occurrence_precision",
        "triage_precision",
        "canonical_rank",
        "relative_elapsed_ns",
        "cohort_denominator",
        "triage_denominator",
        "future_outcome",
        "duplicate_external_id",
    ];
    for wide_row in wide {
        let source_id = wide_row[0].expect("wide row has a source id");
        let mut long_fields = HashMap::new();
        for long_row in long.iter().filter(|row| row[0] == Some(source_id)) {
            assert_eq!(long_row[1], Some("arrival"));
            let field_name = long_row[2].expect("long row has a field name");
            assert!(
                long_fields.insert(field_name, long_row[3]).is_none(),
                "duplicate {field_name} for {source_id}"
            );
        }
        assert_eq!(
            long_fields.len(),
            WIDE_FIELD_NAMES.len(),
            "long payload field count for {source_id}"
        );
        for (index, field_name) in WIDE_FIELD_NAMES.iter().enumerate() {
            assert_eq!(
                long_fields
                    .get(field_name)
                    .expect("long payload field present"),
                &wide_row[index],
                "wide/long value mismatch for {source_id}.{field_name}"
            );
        }
    }
}

fn output_dir() -> PathBuf {
    PathBuf::from(
        std::env::var_os("KAIROS_CALIBRATION_OUTDIR")
            .or_else(|| std::env::var_os("KAIROS_INTEROP_OUTDIR"))
            .expect(
                "set KAIROS_CALIBRATION_OUTDIR or the workflow's KAIROS_INTEROP_OUTDIR to the packet-bound output directory",
            ),
    )
}

fn read_format(
    bytes: &[u8],
    fmt: &str,
    expected: SchemaRef,
) -> Result<Vec<RecordBatch>, kairo_ecs_arrow_io::IoError> {
    match fmt {
        "ipc_file" => read_ipc_file(bytes, expected, limits()),
        "ipc_stream" => read_ipc_stream(bytes, expected, limits()),
        "parquet" => read_parquet(bytes, expected, limits()),
        _ => unreachable!(),
    }
}

fn write_format(schema: SchemaRef, batches: &[RecordBatch], fmt: &str) -> Vec<u8> {
    match fmt {
        "ipc_file" => write_ipc_file(schema, batches, limits()).unwrap(),
        "ipc_stream" => write_ipc_stream(schema, batches, limits()).unwrap(),
        "parquet" => write_parquet(schema, batches, limits()).unwrap(),
        _ => unreachable!(),
    }
}

#[test]
fn pyarrow_raw_source_fixtures_round_trip_and_rust_outputs_are_external_reader_ready() {
    assert_long_payload_matches_wide(&wide_rows(), &long_rows());
    let output = output_dir();
    fs::create_dir_all(&output).unwrap();
    for (profile, expected) in [("wide", wide_rows()), ("long", long_rows())] {
        let expected_schema = schema(profile);
        for fmt in ["ipc_file", "ipc_stream", "parquet"] {
            let name = format!("{profile}.{fmt}");
            let encoded = fixture(&name);
            let batches = read_format(&encoded, fmt, Arc::clone(&expected_schema)).unwrap();
            assert!(!batches.is_empty());
            for batch in &batches {
                assert_eq!(batch.schema().as_ref(), expected_schema.as_ref());
            }
            assert_rows(&batches, &expected);
            fs::write(
                output.join(&name),
                write_format(Arc::clone(&expected_schema), &batches, fmt),
            )
            .unwrap();

            // The reader must reject every tested class of schema drift and truncation.
            let changed_metadata = expected_schema
                .as_ref()
                .clone()
                .with_metadata(metadata(&[("changed", "true")]));
            assert!(
                read_format(&encoded, fmt, Arc::new(changed_metadata)).is_err(),
                "metadata drift: {name}"
            );

            let changed_type = Arc::new(Schema::new_with_metadata(
                std::iter::once(Field::new("source_row_id", DataType::Int32, false))
                    .chain(
                        expected_schema
                            .fields()
                            .iter()
                            .skip(1)
                            .map(|field| field.as_ref().clone()),
                    )
                    .collect::<Vec<_>>(),
                metadata(&META),
            ));
            assert!(
                read_format(&encoded, fmt, changed_type).is_err(),
                "type drift: {name}"
            );

            let changed_nullability = Arc::new(Schema::new_with_metadata(
                std::iter::once(Field::new("source_row_id", DataType::Utf8, true))
                    .chain(
                        expected_schema
                            .fields()
                            .iter()
                            .skip(1)
                            .map(|field| field.as_ref().clone()),
                    )
                    .collect::<Vec<_>>(),
                metadata(&META),
            ));
            assert!(
                read_format(&encoded, fmt, changed_nullability).is_err(),
                "nullability drift: {name}"
            );

            let mut reordered = expected_schema
                .fields()
                .iter()
                .map(|field| field.as_ref().clone())
                .collect::<Vec<_>>();
            reordered.swap(0, 1);
            assert!(
                read_format(
                    &encoded,
                    fmt,
                    Arc::new(Schema::new_with_metadata(reordered, metadata(&META)))
                )
                .is_err(),
                "field order drift: {name}"
            );
            assert!(
                read_format(
                    &encoded[..encoded.len() - 1],
                    fmt,
                    Arc::clone(&expected_schema)
                )
                .is_err(),
                "truncation: {name}"
            );
        }
    }
}
