use std::sync::{atomic::AtomicBool, Arc};

use arrow_array::RecordBatch;
use arrow_schema::{Schema, SchemaRef};
use bytes::Bytes;
use parquet::{
    arrow::{arrow_reader::ParquetRecordBatchReaderBuilder, ArrowWriter},
    basic::Compression,
    file::metadata::KeyValue,
    file::properties::WriterProperties,
};

use crate::{
    limits::{output_limit_hit, BoundedBuffer},
    IoError, IoLimits,
};

/// Read Parquet bytes after checking file metadata and then each emitted batch.
pub fn read_parquet(
    input: &[u8],
    expected: SchemaRef,
    limits: IoLimits,
) -> Result<Vec<RecordBatch>, IoError> {
    limits.validate()?;
    limits.check_input_len(input.len())?;
    limits.check_columns(expected.fields().len())?;

    let builder = ParquetRecordBatchReaderBuilder::try_new(Bytes::copy_from_slice(input))?;
    let validated_schema = Arc::clone(builder.schema());
    ensure_schema(validated_schema.as_ref(), expected.as_ref())?;

    let metadata = builder.metadata();
    let file_rows = usize::try_from(metadata.file_metadata().num_rows())
        .map_err(|_| IoError::LimitExceeded("max_total_rows"))?;
    if file_rows > limits.max_total_rows {
        return Err(IoError::LimitExceeded("max_total_rows"));
    }

    // Parquet record batches may span row-group boundaries. Enforce max_batches
    // against actual decoder output below, rather than estimating from groups.
    let mut reader = builder.with_batch_size(limits.max_batch_rows).build()?;
    let mut batches = Vec::new();
    let mut rows = 0usize;
    while let Some(batch) = reader.next() {
        let batch = batch?;
        let decoded_schema = batch.schema();
        if decoded_schema.fields() != validated_schema.fields()
            || (!decoded_schema.metadata().is_empty()
                && decoded_schema.metadata() != validated_schema.metadata())
        {
            return Err(IoError::SchemaMismatch);
        }
        let batch = batch.with_schema(Arc::clone(&validated_schema))?;
        limits.check_batch(&batch, expected.as_ref())?;
        limits.next_batch_count(batches.len())?;
        rows = limits.next_row_count(rows, batch.num_rows())?;
        batches.push(batch);
    }
    Ok(batches)
}

/// Write Parquet bytes after validating every input batch and resource count.
pub fn write_parquet(
    expected: SchemaRef,
    batches: &[RecordBatch],
    limits: IoLimits,
) -> Result<Vec<u8>, IoError> {
    limits.check_batches(batches, expected.as_ref())?;
    let mut schema_metadata = expected
        .metadata()
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect::<Vec<_>>();
    schema_metadata.sort_by(|left, right| left.0.cmp(&right.0));
    let properties = WriterProperties::builder()
        .set_compression(Compression::UNCOMPRESSED)
        .set_max_row_group_row_count(Some(limits.max_batch_rows))
        .set_max_row_group_bytes(None)
        .set_key_value_metadata(Some(
            schema_metadata
                .into_iter()
                .map(|(key, value)| KeyValue::new(key, value))
                .collect(),
        ))
        .build();
    let exceeded = Arc::new(AtomicBool::new(false));
    let sink = BoundedBuffer::new(limits.max_output_bytes, exceeded.clone());
    let mut writer = match ArrowWriter::try_new(sink, expected, Some(properties)) {
        Ok(writer) => writer,
        Err(error) => return Err(map_output_error(&exceeded, error.into())),
    };
    for batch in batches {
        if let Err(error) = writer.write(batch) {
            return Err(map_output_error(&exceeded, error.into()));
        }
    }
    let sink = match writer.into_inner() {
        Ok(sink) => sink,
        Err(error) => return Err(map_output_error(&exceeded, error.into())),
    };
    Ok(sink.into_inner())
}

fn ensure_schema(actual: &Schema, expected: &Schema) -> Result<(), IoError> {
    if actual == expected {
        Ok(())
    } else {
        Err(IoError::SchemaMismatch)
    }
}

fn map_output_error(exceeded: &Arc<AtomicBool>, error: IoError) -> IoError {
    if output_limit_hit(exceeded) {
        IoError::LimitExceeded("max_output_bytes")
    } else {
        error
    }
}

#[cfg(test)]
mod tests {
    use arrow_array::{Int32Array, RecordBatch, StringArray};
    use arrow_schema::{DataType, Field, Schema, SchemaRef};
    use bytes::Bytes;
    use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
    use std::collections::{BTreeMap, HashMap};

    use super::{read_parquet, write_parquet};
    use crate::{IoError, IoLimits};

    fn schema() -> SchemaRef {
        SchemaRef::new(Schema::new(vec![Field::new(
            "value",
            DataType::Int32,
            false,
        )]))
    }

    fn metadata_schema(global_owner: &str, field_unit: &str) -> SchemaRef {
        let mut field_metadata = HashMap::new();
        field_metadata.insert("unit".to_string(), field_unit.to_string());
        let field = Field::new("value", DataType::Int32, false).with_metadata(field_metadata);
        let mut schema_metadata = HashMap::new();
        schema_metadata.insert("owner".to_string(), global_owner.to_string());
        SchemaRef::new(Schema::new(vec![field]).with_metadata(schema_metadata))
    }

    fn batch(schema: &SchemaRef, values: &[i32]) -> RecordBatch {
        RecordBatch::try_new(
            schema.clone(),
            vec![std::sync::Arc::new(Int32Array::from(values.to_vec()))],
        )
        .unwrap()
    }

    fn rows(batches: &[RecordBatch]) -> Vec<i32> {
        batches
            .iter()
            .flat_map(|batch| {
                batch
                    .column(0)
                    .as_any()
                    .downcast_ref::<Int32Array>()
                    .unwrap()
                    .values()
                    .iter()
                    .copied()
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    fn permissive_limits() -> IoLimits {
        IoLimits {
            max_input_bytes: 1024 * 1024,
            max_output_bytes: 1024 * 1024,
            max_batch_rows: 2,
            max_total_rows: 16,
            max_batches: 8,
            max_columns: 4,
        }
    }

    #[test]
    fn parquet_round_trip_bounds_row_groups_and_rows() {
        let schema = schema();
        let batches = vec![batch(&schema, &[1, 2]), batch(&schema, &[3])];
        let bytes = write_parquet(schema.clone(), &batches, permissive_limits()).unwrap();
        let decoded = read_parquet(&bytes, schema.clone(), permissive_limits()).unwrap();
        assert_eq!(rows(&decoded), vec![1, 2, 3]);
        assert!(decoded.iter().all(|batch| batch.num_rows() <= 2));
        let one_row_batches = read_parquet(
            &bytes,
            schema.clone(),
            IoLimits {
                max_batch_rows: 1,
                ..permissive_limits()
            },
        )
        .unwrap();
        assert_eq!(rows(&one_row_batches), vec![1, 2, 3]);
        assert!(one_row_batches.iter().all(|batch| batch.num_rows() <= 1));
    }

    #[test]
    fn max_batches_counts_returned_batches_across_row_groups() {
        let schema = schema();
        let input_batches = vec![batch(&schema, &[1]), batch(&schema, &[2])];
        let bytes = write_parquet(
            schema.clone(),
            &input_batches,
            IoLimits {
                max_batch_rows: 1,
                max_total_rows: 2,
                max_batches: 2,
                ..permissive_limits()
            },
        )
        .unwrap();

        let metadata_builder =
            ParquetRecordBatchReaderBuilder::try_new(Bytes::copy_from_slice(&bytes)).unwrap();
        assert_eq!(metadata_builder.metadata().num_row_groups(), 2);

        let decoded = read_parquet(
            &bytes,
            schema.clone(),
            IoLimits {
                max_batch_rows: 2,
                max_total_rows: 2,
                max_batches: 1,
                ..permissive_limits()
            },
        )
        .unwrap();
        assert_eq!(decoded.len(), 1);
        assert_eq!(rows(&decoded), vec![1, 2]);

        let large_batch_size = read_parquet(
            &bytes,
            schema.clone(),
            IoLimits {
                max_batch_rows: usize::MAX,
                max_total_rows: 2,
                max_batches: 1,
                ..permissive_limits()
            },
        )
        .unwrap();
        assert_eq!(large_batch_size.len(), 1);
        assert_eq!(rows(&large_batch_size), vec![1, 2]);

        let three_row_groups = [
            batch(&schema, &[1]),
            batch(&schema, &[2]),
            batch(&schema, &[3]),
        ];
        let three_group_bytes = write_parquet(
            schema.clone(),
            &three_row_groups,
            IoLimits {
                max_batch_rows: 1,
                max_total_rows: 3,
                max_batches: 3,
                ..permissive_limits()
            },
        )
        .unwrap();
        assert!(matches!(
            read_parquet(
                &three_group_bytes,
                schema,
                IoLimits {
                    max_batch_rows: 2,
                    max_total_rows: 3,
                    max_batches: 1,
                    ..permissive_limits()
                }
            ),
            Err(IoError::LimitExceeded("max_batches"))
        ));
    }

    #[test]
    fn parquet_empty_batch_list_preserves_schema() {
        let schema = schema();
        let bytes = write_parquet(schema.clone(), &[], permissive_limits()).unwrap();
        assert!(read_parquet(&bytes, schema, permissive_limits())
            .unwrap()
            .is_empty());
    }

    #[test]
    fn reader_enforces_encoded_bytes_rows_columns_and_schema() {
        let schema = schema();
        let batches = vec![batch(&schema, &[1, 2]), batch(&schema, &[3])];
        let bytes = write_parquet(schema.clone(), &batches, permissive_limits()).unwrap();
        let mismatch = SchemaRef::new(Schema::new(vec![Field::new(
            "other",
            DataType::Int32,
            false,
        )]));
        assert!(matches!(
            read_parquet(&bytes, mismatch, permissive_limits()),
            Err(IoError::SchemaMismatch)
        ));
        assert!(matches!(
            read_parquet(
                &bytes,
                schema.clone(),
                IoLimits {
                    max_input_bytes: bytes.len() - 1,
                    ..permissive_limits()
                }
            ),
            Err(IoError::LimitExceeded("max_input_bytes"))
        ));
        assert!(matches!(
            read_parquet(
                &bytes,
                schema.clone(),
                IoLimits {
                    max_total_rows: 2,
                    ..permissive_limits()
                }
            ),
            Err(IoError::LimitExceeded("max_total_rows"))
        ));
        assert!(matches!(
            read_parquet(
                &bytes,
                schema,
                IoLimits {
                    max_batches: 1,
                    ..permissive_limits()
                }
            ),
            Err(IoError::LimitExceeded("max_batches"))
        ));
    }

    #[test]
    fn writer_prevalidates_batches_and_caps_output() {
        let schema = schema();
        let too_many_rows = vec![batch(&schema, &[1, 2, 3])];
        assert!(matches!(
            write_parquet(schema.clone(), &too_many_rows, permissive_limits()),
            Err(IoError::LimitExceeded("max_batch_rows"))
        ));
        let one = vec![batch(&schema, &[1])];
        assert!(matches!(
            write_parquet(
                schema.clone(),
                &one,
                IoLimits {
                    max_output_bytes: 1,
                    ..permissive_limits()
                }
            ),
            Err(IoError::LimitExceeded("max_output_bytes"))
        ));
        let zero_row = batch(&schema, &[]);
        let two_zero_rows = [zero_row, batch(&schema, &[])];
        assert!(matches!(
            write_parquet(
                schema.clone(),
                &two_zero_rows,
                IoLimits {
                    max_batches: 1,
                    ..permissive_limits()
                }
            ),
            Err(IoError::LimitExceeded("max_batches"))
        ));
    }

    #[test]
    fn parquet_reader_restores_validated_schema_metadata_on_batches() {
        let schema = metadata_schema("fixture", "ns");
        let input = vec![batch(&schema, &[1, 2]), batch(&schema, &[3])];
        let bytes = write_parquet(schema.clone(), &input, permissive_limits()).unwrap();
        let decoded = read_parquet(&bytes, schema.clone(), permissive_limits()).unwrap();

        assert_eq!(rows(&decoded), vec![1, 2, 3]);
        assert_eq!(decoded.len(), 2);
        assert!(decoded
            .iter()
            .all(|batch| batch.schema().as_ref() == schema.as_ref()));
    }

    #[test]
    fn parquet_footer_copies_sorted_global_schema_metadata() {
        let base = metadata_schema("fixture", "ns");
        let mut globals = base.metadata().clone();
        globals.insert("zeta".to_string(), "last".to_string());
        globals.insert("alpha".to_string(), "first".to_string());
        let schema = SchemaRef::new(Schema::new_with_metadata(base.fields().clone(), globals));
        let bytes =
            write_parquet(schema.clone(), &[batch(&schema, &[1])], permissive_limits()).unwrap();

        let builder =
            ParquetRecordBatchReaderBuilder::try_new(Bytes::copy_from_slice(&bytes)).unwrap();
        let footer = builder
            .metadata()
            .file_metadata()
            .key_value_metadata()
            .unwrap();
        let reserved = footer
            .iter()
            .filter(|pair| pair.key == "ARROW:schema")
            .collect::<Vec<_>>();
        assert_eq!(reserved.len(), 1);
        assert!(reserved[0].value.is_some());

        let actual = footer
            .iter()
            .filter(|pair| pair.key != "ARROW:schema")
            .map(|pair| (pair.key.clone(), pair.value.clone().unwrap()))
            .collect::<Vec<_>>();
        let expected = schema
            .metadata()
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(actual, expected.into_iter().collect::<Vec<_>>());
    }

    #[test]
    fn caller_arrow_schema_metadata_round_trips_without_replacing_reserved_footer_value() {
        let base = metadata_schema("fixture", "ns");
        let mut globals = base.metadata().clone();
        globals.insert("ARROW:schema".to_string(), "caller-value".to_string());
        let schema = SchemaRef::new(Schema::new_with_metadata(base.fields().clone(), globals));
        let bytes =
            write_parquet(schema.clone(), &[batch(&schema, &[1])], permissive_limits()).unwrap();

        let builder =
            ParquetRecordBatchReaderBuilder::try_new(Bytes::copy_from_slice(&bytes)).unwrap();
        let footer = builder
            .metadata()
            .file_metadata()
            .key_value_metadata()
            .unwrap();
        let reserved = footer
            .iter()
            .filter(|pair| pair.key == "ARROW:schema")
            .collect::<Vec<_>>();
        assert_eq!(reserved.len(), 1);
        assert_ne!(reserved[0].value.as_deref(), Some("caller-value"));

        let decoded = read_parquet(&bytes, schema.clone(), permissive_limits()).unwrap();
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].schema().as_ref(), schema.as_ref());
    }

    #[test]
    fn schema_metadata_mismatch_is_not_ignored() {
        let schema = metadata_schema("fixture", "ns");
        let expected = metadata_schema("other", "ns");
        let bytes = write_parquet(schema, &[], permissive_limits()).unwrap();
        assert!(matches!(
            read_parquet(&bytes, expected, permissive_limits()),
            Err(IoError::SchemaMismatch)
        ));
    }

    #[test]
    fn field_metadata_mismatch_is_not_ignored() {
        let schema = metadata_schema("fixture", "ns");
        let bytes =
            write_parquet(schema.clone(), &[batch(&schema, &[1])], permissive_limits()).unwrap();
        let expected = metadata_schema("fixture", "other");
        assert!(matches!(
            read_parquet(&bytes, expected, permissive_limits()),
            Err(IoError::SchemaMismatch)
        ));
    }

    #[test]
    fn column_cap_is_checked_before_parquet_reader_or_writer_work() {
        let schema = SchemaRef::new(Schema::new(vec![
            Field::new("value", DataType::Int32, false),
            Field::new("label", DataType::Utf8, false),
        ]));
        let two_column_batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                std::sync::Arc::new(Int32Array::from(vec![1])),
                std::sync::Arc::new(StringArray::from(vec!["row"])),
            ],
        )
        .unwrap();
        let batches = [two_column_batch];
        let limits = IoLimits {
            max_columns: 1,
            ..permissive_limits()
        };
        assert!(matches!(
            write_parquet(schema.clone(), &batches, limits),
            Err(IoError::LimitExceeded("max_columns"))
        ));
        let bytes = write_parquet(schema.clone(), &batches, permissive_limits()).unwrap();
        assert!(matches!(
            read_parquet(&bytes, schema.clone(), limits),
            Err(IoError::LimitExceeded("max_columns"))
        ));
        assert!(matches!(
            write_parquet(
                schema.clone(),
                &[],
                IoLimits {
                    max_columns: 0,
                    ..permissive_limits()
                }
            ),
            Err(IoError::InvalidLimits)
        ));
    }

    #[test]
    fn malformed_and_truncated_files_are_errors_not_panics() {
        let schema = schema();
        for bytes in [&[][..], &[1, 2, 3][..]] {
            assert!(read_parquet(bytes, schema.clone(), permissive_limits()).is_err());
        }
        let valid = write_parquet(schema.clone(), &[], permissive_limits()).unwrap();
        assert!(read_parquet(&valid[..valid.len() - 1], schema, permissive_limits()).is_err());
    }
}
