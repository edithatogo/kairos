//! C0 physical v2 fixtures through independently declared Rust schemas.
#![cfg(any(feature = "ipc", feature = "parquet"))]

use std::{fs, path::PathBuf, sync::Arc};

use arrow_array::RecordBatch;
use arrow_schema::{DataType, SchemaRef};
use kairo_ecs_arrow_io::{IoError, IoLimits};

#[path = "support/calibration_physical_schema_v2.rs"]
mod frozen;

fn fixture(name: &str, format: &str) -> Vec<u8> {
    fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/calibration_physical_v2")
            .join(format!("{name}.{format}")),
    )
    .expect("committed physical fixture")
}

fn output_dir() -> PathBuf {
    let output = std::env::var_os("KAIROS_C11_PHYSICAL_OUTDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::temp_dir().join(format!("kairos-c11-physical-{}", std::process::id()))
        });
    fs::create_dir_all(&output).expect("physical readback directory");
    output
}

fn limits() -> IoLimits {
    IoLimits {
        max_batch_rows: 2,
        max_total_rows: 10_000,
        max_batches: 10_000,
        ..IoLimits::default()
    }
}

fn rows(batches: &[RecordBatch]) -> Vec<RecordBatch> {
    batches
        .iter()
        .flat_map(|batch| (0..batch.num_rows()).map(|index| batch.slice(index, 1)))
        .collect()
}

fn variants(schema: &SchemaRef) -> Vec<SchemaRef> {
    use frozen::{change_field, change_global_metadata, FieldChange};

    let mut variants = vec![
        change_field(
            schema,
            &["record_type"],
            FieldChange::DataType(DataType::Binary),
        ),
        change_field(schema, &["schema_version"], FieldChange::Nullable(true)),
        change_field(
            schema,
            &["record_type"],
            FieldChange::Metadata("logical_path".to_string(), Some("wrong.path".to_string())),
        ),
        change_global_metadata(schema, "physical_version", Some("1")),
        change_global_metadata(schema, "physical_version", Some("unknown")),
        change_global_metadata(schema, "format", None),
    ];
    match schema.metadata().get("record_type").map(String::as_str) {
        Some("trace_event.v1") => {
            variants.push(change_field(
                schema,
                &["occurrence_time", "utc_i128_le"],
                FieldChange::DataType(DataType::FixedSizeBinary(8)),
            ));
            variants.push(change_field(
                schema,
                &["occurrence_time", "utc_i128_le"],
                FieldChange::Metadata("encoding".to_string(), Some("unsigned_i128_le".to_string())),
            ));
        }
        Some("trace_exclusion.v1") => variants.push(change_field(
            schema,
            &["raw_time_values", "element", "key"],
            FieldChange::DataType(DataType::LargeUtf8),
        )),
        Some("outcome_observation.v1") => variants.push(change_field(
            schema,
            &["lineage", "status"],
            FieldChange::Nullable(true),
        )),
        _ => unreachable!(),
    }
    variants
}

#[cfg(feature = "ipc")]
#[test]
fn actual_ipc_tables_use_full_frozen_schema_and_roundtrip() {
    use arrow_ipc::reader::FileReader;
    use kairo_ecs_arrow_io::{read_ipc_file, read_ipc_stream, write_ipc_file, write_ipc_stream};

    let out = output_dir();
    for (name, record_type) in frozen::TABLES {
        let schema = frozen::schema(record_type);
        let file = fixture(name, "ipc_file");
        let stream = fixture(name, "ipc_stream");
        let actual_schema = FileReader::try_new(std::io::Cursor::new(&file), None)
            .unwrap()
            .schema();
        assert_eq!(
            actual_schema.as_ref(),
            schema.as_ref(),
            "frozen schema {record_type}"
        );
        let file_rows = read_ipc_file(&file, Arc::clone(&schema), limits()).unwrap();
        let stream_rows = read_ipc_stream(&stream, Arc::clone(&schema), limits()).unwrap();
        assert_eq!(rows(&file_rows), rows(&stream_rows), "{record_type}");

        let file_again = write_ipc_file(Arc::clone(&schema), &file_rows, limits()).unwrap();
        let stream_again = write_ipc_stream(Arc::clone(&schema), &file_rows, limits()).unwrap();
        assert_eq!(
            rows(&file_rows),
            rows(&read_ipc_file(&file_again, Arc::clone(&schema), limits()).unwrap()),
            "IPC file output {record_type}"
        );
        assert_eq!(
            rows(&file_rows),
            rows(&read_ipc_stream(&stream_again, Arc::clone(&schema), limits()).unwrap()),
            "IPC stream output {record_type}"
        );
        fs::write(out.join(format!("{name}.ipc_file")), file_again).unwrap();
        fs::write(out.join(format!("{name}.ipc_stream")), stream_again).unwrap();
        assert!(matches!(
            write_ipc_file(
                Arc::clone(&schema),
                &file_rows,
                IoLimits {
                    max_output_bytes: 1,
                    ..limits()
                }
            ),
            Err(IoError::LimitExceeded("max_output_bytes"))
        ));
        assert!(matches!(
            write_ipc_stream(
                Arc::clone(&schema),
                &file_rows,
                IoLimits {
                    max_output_bytes: 1,
                    ..limits()
                }
            ),
            Err(IoError::LimitExceeded("max_output_bytes"))
        ));

        for bad_schema in variants(&schema) {
            assert!(
                matches!(
                    read_ipc_file(&file, Arc::clone(&bad_schema), limits()),
                    Err(IoError::SchemaMismatch)
                ),
                "IPC file accepted changed schema for {record_type}"
            );
            assert!(
                matches!(
                    read_ipc_stream(&stream, bad_schema, limits()),
                    Err(IoError::SchemaMismatch)
                ),
                "IPC stream accepted changed schema for {record_type}"
            );
        }

        assert!(read_ipc_file(&file[..file.len() - 1], Arc::clone(&schema), limits()).is_err());
        assert!(
            read_ipc_stream(&stream[..stream.len() - 1], Arc::clone(&schema), limits()).is_err()
        );

        assert!(matches!(
            read_ipc_file(
                &file,
                Arc::clone(&schema),
                IoLimits {
                    max_input_bytes: file.len() - 1,
                    ..limits()
                }
            ),
            Err(IoError::LimitExceeded("max_input_bytes"))
        ));
        assert!(matches!(
            read_ipc_stream(
                &stream,
                Arc::clone(&schema),
                IoLimits {
                    max_input_bytes: stream.len() - 1,
                    ..limits()
                }
            ),
            Err(IoError::LimitExceeded("max_input_bytes"))
        ));
        let total_rows: usize = file_rows.iter().map(RecordBatch::num_rows).sum();
        assert!(matches!(
            read_ipc_file(
                &file,
                Arc::clone(&schema),
                IoLimits {
                    max_total_rows: total_rows - 1,
                    ..limits()
                }
            ),
            Err(IoError::LimitExceeded("max_total_rows"))
        ));
        assert!(matches!(
            read_ipc_stream(
                &stream,
                Arc::clone(&schema),
                IoLimits {
                    max_total_rows: total_rows - 1,
                    ..limits()
                }
            ),
            Err(IoError::LimitExceeded("max_total_rows"))
        ));
        assert!(matches!(
            read_ipc_file(
                &file,
                Arc::clone(&schema),
                IoLimits {
                    max_batch_rows: 1,
                    ..limits()
                }
            ),
            Err(IoError::LimitExceeded("max_batch_rows"))
        ));
        assert!(matches!(
            read_ipc_stream(
                &stream,
                Arc::clone(&schema),
                IoLimits {
                    max_batch_rows: 1,
                    ..limits()
                }
            ),
            Err(IoError::LimitExceeded("max_batch_rows"))
        ));
        assert!(matches!(
            read_ipc_file(
                &file,
                Arc::clone(&schema),
                IoLimits {
                    max_batches: 1,
                    ..limits()
                }
            ),
            Err(IoError::LimitExceeded("max_batches"))
        ));
        assert!(matches!(
            read_ipc_stream(
                &stream,
                Arc::clone(&schema),
                IoLimits {
                    max_batches: 1,
                    ..limits()
                }
            ),
            Err(IoError::LimitExceeded("max_batches"))
        ));
        assert!(matches!(
            read_ipc_file(
                &file,
                Arc::clone(&schema),
                IoLimits {
                    max_columns: 1,
                    ..limits()
                }
            ),
            Err(IoError::LimitExceeded("max_columns"))
        ));
        assert!(matches!(
            read_ipc_stream(
                &stream,
                Arc::clone(&schema),
                IoLimits {
                    max_columns: 1,
                    ..limits()
                }
            ),
            Err(IoError::LimitExceeded("max_columns"))
        ));
    }
}

#[cfg(feature = "parquet")]
#[test]
fn actual_parquet_tables_use_full_frozen_schema_and_roundtrip() {
    use kairo_ecs_arrow_io::{read_parquet, write_parquet};

    let out = output_dir();
    for (name, record_type) in frozen::TABLES {
        let schema = frozen::schema(record_type);
        let parquet = fixture(name, "parquet");
        let base = read_parquet(&parquet, Arc::clone(&schema), limits()).unwrap();
        let encoded = write_parquet(Arc::clone(&schema), &base, limits()).unwrap();
        let decoded = read_parquet(&encoded, Arc::clone(&schema), limits()).unwrap();
        assert_eq!(rows(&base), rows(&decoded), "Parquet output {record_type}");
        fs::write(out.join(format!("{name}.parquet")), encoded).unwrap();
        assert!(matches!(
            write_parquet(
                Arc::clone(&schema),
                &base,
                IoLimits {
                    max_output_bytes: 1,
                    ..limits()
                }
            ),
            Err(IoError::LimitExceeded("max_output_bytes"))
        ));

        for bad_schema in variants(&schema) {
            assert!(
                matches!(
                    read_parquet(&parquet, bad_schema, limits()),
                    Err(IoError::SchemaMismatch)
                ),
                "Parquet accepted changed schema for {record_type}"
            );
        }
        assert!(
            read_parquet(&parquet[..parquet.len() - 1], Arc::clone(&schema), limits()).is_err()
        );
        assert!(matches!(
            read_parquet(
                &parquet,
                Arc::clone(&schema),
                IoLimits {
                    max_input_bytes: parquet.len() - 1,
                    ..limits()
                }
            ),
            Err(IoError::LimitExceeded("max_input_bytes"))
        ));
        let total_rows: usize = base.iter().map(RecordBatch::num_rows).sum();
        assert!(matches!(
            read_parquet(
                &parquet,
                Arc::clone(&schema),
                IoLimits {
                    max_total_rows: total_rows - 1,
                    ..limits()
                }
            ),
            Err(IoError::LimitExceeded("max_total_rows"))
        ));
        let one_row_batches = read_parquet(
            &parquet,
            Arc::clone(&schema),
            IoLimits {
                max_batch_rows: 1,
                ..limits()
            },
        )
        .unwrap();
        assert_eq!(rows(&base), rows(&one_row_batches));
        assert!(one_row_batches.iter().all(|batch| batch.num_rows() <= 1));
        assert!(matches!(
            read_parquet(
                &parquet,
                Arc::clone(&schema),
                IoLimits {
                    max_batches: 1,
                    ..limits()
                }
            ),
            Err(IoError::LimitExceeded("max_batches"))
        ));
        assert!(matches!(
            read_parquet(
                &parquet,
                Arc::clone(&schema),
                IoLimits {
                    max_columns: 1,
                    ..limits()
                }
            ),
            Err(IoError::LimitExceeded("max_columns"))
        ));
    }
}
