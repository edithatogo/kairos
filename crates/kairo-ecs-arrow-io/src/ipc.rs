use std::{
    io::Cursor,
    sync::{atomic::AtomicBool, Arc},
};

use arrow_array::RecordBatch;
use arrow_ipc::{
    reader::{FileReader, StreamReader},
    writer::{FileWriter, StreamWriter},
};
use arrow_schema::{Schema, SchemaRef};

use crate::{
    limits::{output_limit_hit, BoundedBuffer},
    IoError, IoLimits,
};

/// Read an Arrow IPC file after enforcing the encoded-input and schema bounds.
pub fn read_ipc_file(
    input: &[u8],
    expected: SchemaRef,
    limits: IoLimits,
) -> Result<Vec<RecordBatch>, IoError> {
    limits.validate()?;
    limits.check_input_len(input.len())?;
    limits.check_columns(expected.fields().len())?;

    let mut reader = FileReader::try_new(Cursor::new(input), None)?;
    ensure_schema(reader.schema().as_ref(), expected.as_ref())?;
    if reader.num_batches() > limits.max_batches {
        return Err(IoError::LimitExceeded("max_batches"));
    }
    read_batches(&mut reader, expected.as_ref(), limits)
}

/// Read an Arrow IPC stream incrementally under the supplied row and batch caps.
pub fn read_ipc_stream(
    input: &[u8],
    expected: SchemaRef,
    limits: IoLimits,
) -> Result<Vec<RecordBatch>, IoError> {
    limits.validate()?;
    limits.check_input_len(input.len())?;
    limits.check_columns(expected.fields().len())?;

    let mut reader = StreamReader::try_new(Cursor::new(input), None)?;
    ensure_schema(reader.schema().as_ref(), expected.as_ref())?;
    read_batches(&mut reader, expected.as_ref(), limits)
}

/// Write an Arrow IPC file, validating the complete input before starting output.
pub fn write_ipc_file(
    expected: SchemaRef,
    batches: &[RecordBatch],
    limits: IoLimits,
) -> Result<Vec<u8>, IoError> {
    limits.check_batches(batches, expected.as_ref())?;
    let exceeded = Arc::new(AtomicBool::new(false));
    let sink = BoundedBuffer::new(limits.max_output_bytes, exceeded.clone());
    let mut writer = match FileWriter::try_new(sink, expected.as_ref()) {
        Ok(writer) => writer,
        Err(error) => return Err(map_output_error(&exceeded, error.into())),
    };
    for batch in batches {
        if let Err(error) = writer.write(batch) {
            return Err(map_output_error(&exceeded, error.into()));
        }
    }
    if let Err(error) = writer.finish() {
        return Err(map_output_error(&exceeded, error.into()));
    }
    let sink = match writer.into_inner() {
        Ok(sink) => sink,
        Err(error) => return Err(map_output_error(&exceeded, error.into())),
    };
    Ok(sink.into_inner())
}

/// Write an Arrow IPC stream, validating the complete input before starting output.
pub fn write_ipc_stream(
    expected: SchemaRef,
    batches: &[RecordBatch],
    limits: IoLimits,
) -> Result<Vec<u8>, IoError> {
    limits.check_batches(batches, expected.as_ref())?;
    let exceeded = Arc::new(AtomicBool::new(false));
    let sink = BoundedBuffer::new(limits.max_output_bytes, exceeded.clone());
    let mut writer = match StreamWriter::try_new(sink, expected.as_ref()) {
        Ok(writer) => writer,
        Err(error) => return Err(map_output_error(&exceeded, error.into())),
    };
    for batch in batches {
        if let Err(error) = writer.write(batch) {
            return Err(map_output_error(&exceeded, error.into()));
        }
    }
    if let Err(error) = writer.finish() {
        return Err(map_output_error(&exceeded, error.into()));
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

fn read_batches<I>(
    reader: &mut I,
    expected: &Schema,
    limits: IoLimits,
) -> Result<Vec<RecordBatch>, IoError>
where
    I: Iterator<Item = Result<RecordBatch, arrow_schema::ArrowError>>,
{
    let mut batches = Vec::new();
    let mut rows = 0usize;
    for batch in reader {
        let batch = batch?;
        limits.check_batch(&batch, expected)?;
        let count = limits.next_batch_count(batches.len())?;
        rows = limits.next_row_count(rows, batch.num_rows())?;
        debug_assert_eq!(count, batches.len() + 1);
        batches.push(batch);
    }
    Ok(batches)
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
    use std::collections::HashMap;

    use super::{read_ipc_file, read_ipc_stream, write_ipc_file, write_ipc_stream};
    use crate::{IoError, IoLimits};

    fn schema() -> SchemaRef {
        SchemaRef::new(Schema::new(vec![Field::new(
            "value",
            DataType::Int32,
            false,
        )]))
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
            max_batch_rows: 8,
            max_total_rows: 16,
            max_batches: 8,
            max_columns: 4,
        }
    }

    #[test]
    fn file_and_stream_round_trip_multiple_batches() {
        let schema = schema();
        let batches = vec![batch(&schema, &[1, 2]), batch(&schema, &[3])];
        let file = write_ipc_file(schema.clone(), &batches, permissive_limits()).unwrap();
        let stream = write_ipc_stream(schema.clone(), &batches, permissive_limits()).unwrap();
        assert_eq!(
            rows(&read_ipc_file(&file, schema.clone(), permissive_limits()).unwrap()),
            vec![1, 2, 3]
        );
        assert_eq!(
            rows(&read_ipc_stream(&stream, schema, permissive_limits()).unwrap()),
            vec![1, 2, 3]
        );
    }

    #[test]
    fn file_and_stream_keep_typed_empty_schema() {
        let schema = schema();
        let file = write_ipc_file(schema.clone(), &[], permissive_limits()).unwrap();
        let stream = write_ipc_stream(schema.clone(), &[], permissive_limits()).unwrap();
        assert_eq!(
            read_ipc_file(&file, schema.clone(), permissive_limits())
                .unwrap()
                .len(),
            0
        );
        assert_eq!(
            read_ipc_stream(&stream, schema, permissive_limits())
                .unwrap()
                .len(),
            0
        );
    }

    #[test]
    fn zero_row_batch_counts_toward_batch_limit() {
        let schema = schema();
        let zero = batch(&schema, &[]);
        let bytes = write_ipc_stream(schema.clone(), &[zero], permissive_limits()).unwrap();
        let limits = IoLimits {
            max_batches: 1,
            ..permissive_limits()
        };
        assert_eq!(read_ipc_stream(&bytes, schema, limits).unwrap().len(), 1);
    }

    #[test]
    fn readers_reject_schema_mismatch_and_each_resource_limit() {
        let schema = schema();
        let batches = vec![batch(&schema, &[1, 2]), batch(&schema, &[3])];
        let bytes = write_ipc_stream(schema.clone(), &batches, permissive_limits()).unwrap();
        let mismatch = SchemaRef::new(Schema::new(vec![Field::new(
            "different",
            DataType::Int32,
            false,
        )]));
        assert!(matches!(
            read_ipc_stream(&bytes, mismatch, permissive_limits()),
            Err(IoError::SchemaMismatch)
        ));
        assert!(matches!(
            read_ipc_stream(
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
            read_ipc_stream(
                &bytes,
                schema.clone(),
                IoLimits {
                    max_batch_rows: 1,
                    ..permissive_limits()
                }
            ),
            Err(IoError::LimitExceeded("max_batch_rows"))
        ));
        assert!(matches!(
            read_ipc_stream(
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
            read_ipc_stream(
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
    fn writers_reject_output_cap_before_returning_partial_bytes() {
        let schema = schema();
        let batches = vec![batch(&schema, &[1, 2])];
        let limits = IoLimits {
            max_output_bytes: 1,
            ..permissive_limits()
        };
        assert!(matches!(
            write_ipc_file(schema.clone(), &batches, limits),
            Err(IoError::LimitExceeded("max_output_bytes"))
        ));
        assert!(matches!(
            write_ipc_stream(schema, &batches, limits),
            Err(IoError::LimitExceeded("max_output_bytes"))
        ));
    }

    #[test]
    fn malformed_and_truncated_inputs_are_errors_not_panics() {
        let schema = schema();
        for bytes in [&[][..], &[1, 2, 3][..]] {
            assert!(read_ipc_file(bytes, schema.clone(), permissive_limits()).is_err());
            assert!(read_ipc_stream(bytes, schema.clone(), permissive_limits()).is_err());
        }
        let valid = write_ipc_stream(schema.clone(), &[], permissive_limits()).unwrap();
        assert!(read_ipc_stream(&valid[..valid.len() - 1], schema, permissive_limits()).is_err());
    }

    #[test]
    fn field_metadata_mismatch_is_rejected() {
        let schema = schema();
        let bytes =
            write_ipc_file(schema.clone(), &[batch(&schema, &[1])], permissive_limits()).unwrap();
        let mut metadata = HashMap::new();
        metadata.insert("unit".to_string(), "other".to_string());
        let expected = SchemaRef::new(Schema::new(vec![Field::new(
            "value",
            DataType::Int32,
            false,
        )
        .with_metadata(metadata)]));
        assert!(matches!(
            read_ipc_file(&bytes, expected, permissive_limits()),
            Err(IoError::SchemaMismatch)
        ));
    }

    #[test]
    fn column_cap_is_checked_for_file_stream_and_writer_paths() {
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
        let limited = IoLimits {
            max_columns: 1,
            ..permissive_limits()
        };
        let file = write_ipc_file(schema.clone(), &batches, permissive_limits()).unwrap();
        let stream = write_ipc_stream(schema.clone(), &batches, permissive_limits()).unwrap();
        assert!(matches!(
            read_ipc_file(&file, schema.clone(), limited),
            Err(IoError::LimitExceeded("max_columns"))
        ));
        assert!(matches!(
            read_ipc_stream(&stream, schema.clone(), limited),
            Err(IoError::LimitExceeded("max_columns"))
        ));
        assert!(matches!(
            write_ipc_file(schema.clone(), &[], limited),
            Err(IoError::LimitExceeded("max_columns"))
        ));
        assert!(matches!(
            write_ipc_stream(schema, &batches, limited),
            Err(IoError::LimitExceeded("max_columns"))
        ));
    }

    #[test]
    fn file_batch_cap_is_checked_before_reading_batches() {
        let schema = schema();
        let batches = vec![batch(&schema, &[1]), batch(&schema, &[2])];
        let file = write_ipc_file(schema.clone(), &batches, permissive_limits()).unwrap();
        assert!(matches!(
            read_ipc_file(
                &file,
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
    fn writer_counts_zero_row_batches_too() {
        let schema = schema();
        let empty = batch(&schema, &[]);
        assert!(matches!(
            write_ipc_stream(
                schema.clone(),
                &[empty.clone(), empty],
                IoLimits {
                    max_batches: 1,
                    ..permissive_limits()
                }
            ),
            Err(IoError::LimitExceeded("max_batches"))
        ));
    }
}
