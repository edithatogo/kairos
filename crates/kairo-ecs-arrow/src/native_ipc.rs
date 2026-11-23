//! Native Arrow IPC output for event, world and checkpoint metadata tables.

use std::io::{self, Write};
use std::path::Path;
use std::sync::Arc;

use polars_arrow::array::{Array, BooleanArray, ListArray, PrimitiveArray, Utf8Array};
use polars_arrow::datatypes::{ArrowDataType, ArrowSchema, Field, Metadata};
use polars_arrow::io::ipc::write::{
    default_ipc_fields, schema_to_bytes, StreamWriter, WriteOptions,
};
use polars_arrow::offset::OffsetsBuffer;
use polars_arrow::record_batch::RecordBatchT;

use crate::arrow_c_data::{build_v2_batch_arrays, StructArray};
use crate::polars_interop::records_to_dataframe;
use crate::{
    compute_sha256_hex, extend_stable_hash, publish_checkpoint, ArrowError, CheckpointManifest,
    ContiguousBlock, ContiguousWriteManifest, ParallelIoRecordBatch, EVENT_LOG_STREAM_V2,
    FNV_OFFSET_BASIS, SCHEMA_VERSION_V2,
};

const WORLD_SNAPSHOT_STREAM: &str = "kairo_ecs.world_snapshot.v1";
const CHECKPOINT_METADATA_STREAM: &str = "kairo_ecs.checkpoint_metadata.v1";

/// An owned, schema-bound Arrow record batch ready for IPC publication.
pub struct NativeIpcBatch {
    stream: &'static str,
    batch: RecordBatchT<Box<dyn Array>>,
}

impl NativeIpcBatch {
    /// Project the event records into the canonical v2 Arrow schema.
    ///
    /// # Errors
    /// Returns an error if the record batch or Arrow schema construction fails.
    pub fn from_event_log(batch: &ParallelIoRecordBatch) -> Result<Self, ArrowError> {
        let mut records = batch.records().to_vec();
        for record in &mut records {
            record.schema_version = SCHEMA_VERSION_V2;
        }
        let frame = records_to_dataframe(&records)?;
        let (fields, length, arrays, _) = build_v2_batch_arrays(&frame, None)?.into_data();
        Self::new(EVENT_LOG_STREAM_V2, length, fields, arrays)
    }

    /// Build an entity snapshot from ordered (index, generation, active) rows.
    ///
    /// # Errors
    /// Returns an error if the Arrow record batch cannot be constructed.
    pub fn from_world_snapshot(entities: &[(u64, u32, bool)]) -> Result<Self, ArrowError> {
        let (ids, generations): (Vec<_>, Vec<_>) = entities
            .iter()
            .map(|&(id, generation, _)| (id, generation))
            .unzip();
        let active: Vec<_> = entities.iter().map(|&(_, _, active)| active).collect();
        let arrays: Vec<Box<dyn Array>> = vec![
            Box::new(PrimitiveArray::from_vec(ids)),
            Box::new(PrimitiveArray::from_vec(generations)),
            Box::new(BooleanArray::from_slice(active)),
        ];
        let fields = required_fields(["entity_id", "generation", "active"], &arrays);
        Self::new(WORLD_SNAPSHOT_STREAM, entities.len(), fields, arrays)
    }

    /// Build one checkpoint descriptor with a typed list of source blocks.
    ///
    /// # Errors
    /// Returns an error for an invalid manifest or an unrepresentable Arrow batch.
    pub fn from_checkpoint_metadata(manifest: &CheckpointManifest) -> Result<Self, ArrowError> {
        manifest.validate_for_restore()?;
        let (offsets, lengths): (Vec<_>, Vec<_>) = manifest
            .blocks
            .iter()
            .map(|block| (block.offset_bytes, block.byte_len))
            .unzip();
        let rows: Vec<u64> = manifest
            .blocks
            .iter()
            .map(|block| block.row_count as u64)
            .collect();
        let block_arrays: Vec<Box<dyn Array>> = vec![
            Box::new(PrimitiveArray::from_vec(offsets)),
            Box::new(PrimitiveArray::from_vec(lengths)),
            Box::new(PrimitiveArray::from_vec(rows)),
        ];
        let block_fields =
            required_fields(["offset_bytes", "byte_len", "row_count"], &block_arrays);
        let blocks = StructArray::try_new(
            ArrowDataType::Struct(block_fields),
            manifest.blocks.len(),
            block_arrays,
            None,
        )
        .map_err(|error| ArrowError::Io(error.to_string()))?;
        let last = i64::try_from(blocks.len())
            .map_err(|_| ArrowError::InvalidNumber("IPC checkpoint block count"))?;
        let list_offsets = OffsetsBuffer::try_from(vec![0, last])
            .map_err(|error| ArrowError::Io(error.to_string()))?;
        let list_type = ArrowDataType::LargeList(Box::new(Field::new(
            "item".into(),
            blocks.dtype().clone(),
            false,
        )));
        let block_list = ListArray::try_new(list_type, list_offsets, Box::new(blocks), None)
            .map_err(|error| ArrowError::Io(error.to_string()))?;
        let arrays: Vec<Box<dyn Array>> = vec![
            Box::new(Utf8Array::<i64>::from_slice([manifest
                .checkpoint_id
                .as_str()])),
            Box::new(Utf8Array::<i64>::from_slice([manifest.format.as_str()])),
            Box::new(Utf8Array::<i64>::from_slice([manifest.run_id.as_str()])),
            Box::new(Utf8Array::<i64>::from_slice([manifest
                .schema_fingerprint
                .as_str()])),
            Box::new(PrimitiveArray::from_slice([manifest.row_count as u64])),
            Box::new(PrimitiveArray::from_slice([manifest.checksum])),
            Box::new(block_list),
        ];
        let fields = required_fields(
            [
                "checkpoint_id",
                "format",
                "run_id",
                "event_schema_fingerprint",
                "event_row_count",
                "manifest_checksum",
                "blocks",
            ],
            &arrays,
        );
        Self::new(CHECKPOINT_METADATA_STREAM, 1, fields, arrays)
    }

    fn new(
        stream: &'static str,
        length: usize,
        fields: Vec<Field>,
        arrays: Vec<Box<dyn Array>>,
    ) -> Result<Self, ArrowError> {
        let schema: ArrowSchema = fields
            .into_iter()
            .map(|field| (field.name.clone(), field))
            .collect();
        let batch = RecordBatchT::try_new(length, Arc::new(schema), arrays)
            .map_err(|error| ArrowError::Io(error.to_string()))?;
        Ok(Self { stream, batch })
    }

    /// Access the typed Arrow schema and arrays.
    pub fn record_batch(&self) -> &RecordBatchT<Box<dyn Array>> {
        &self.batch
    }

    /// Write a complete Arrow IPC stream and report its physical byte range.
    ///
    /// The fingerprint is the SHA-256 of the IPC schema message. The checksum
    /// covers all bytes written, including the terminator. A nonempty batch
    /// declares one block covering the entire stream. Its offset is relative
    /// to the start of this stream in the supplied writer.
    ///
    /// # Errors
    /// Returns an error if Arrow encoding, writing or flushing fails.
    pub fn write_contiguous<W: Write>(
        &self,
        output: W,
    ) -> Result<ContiguousWriteManifest, ArrowError> {
        let schema = self.batch.schema();
        let metadata: Metadata = [("kairo.stream".into(), self.stream.into())]
            .into_iter()
            .collect();
        let ipc_fields = default_ipc_fields(schema.iter_values());
        let schema_fingerprint =
            compute_sha256_hex(&schema_to_bytes(schema, &ipc_fields, Some(&metadata)));
        let mut writer = StreamWriter::new(
            IpcOutput {
                output,
                len: 0,
                checksum: FNV_OFFSET_BASIS,
            },
            WriteOptions { compression: None },
        );
        writer.set_custom_schema_metadata(Arc::new(metadata));
        writer
            .start(schema, Some(ipc_fields))
            .map_err(|error| ArrowError::Io(error.to_string()))?;
        if !self.batch.is_empty() {
            writer
                .write(&self.batch, None)
                .map_err(|error| ArrowError::Io(error.to_string()))?;
        }
        writer
            .finish()
            .map_err(|error| ArrowError::Io(error.to_string()))?;
        let mut output = writer.into_inner();
        output
            .flush()
            .map_err(|error| ArrowError::Io(error.to_string()))?;
        let row_count = self.batch.len();
        let blocks = if row_count == 0 {
            Vec::new()
        } else {
            vec![ContiguousBlock {
                offset_bytes: 0,
                byte_len: output.len as u64,
                row_count,
            }]
        };
        Ok(ContiguousWriteManifest {
            stream: self.stream.into(),
            schema_fingerprint,
            row_count,
            encoded_len: output.len,
            blocks,
            checksum: output.checksum,
        })
    }

    /// Atomically publish the Arrow IPC stream at a caller-owned destination.
    ///
    /// Use a trusted output directory and a single writer for each path.
    /// If a Unix parent-directory sync fails after publication, the new
    /// stream may already be visible. Read it back before retrying.
    ///
    /// # Errors
    /// Returns an error if IPC encoding, staging or publication fails.
    pub fn write_contiguous_to_path(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<ContiguousWriteManifest, ArrowError> {
        publish_checkpoint(path.as_ref(), |file| self.write_contiguous(file))
    }
}

fn required_fields<const N: usize>(names: [&str; N], arrays: &[Box<dyn Array>]) -> Vec<Field> {
    names
        .into_iter()
        .zip(arrays)
        .map(|(name, array)| Field::new(name.into(), array.dtype().clone(), false))
        .collect()
}

struct IpcOutput<W> {
    output: W,
    len: usize,
    checksum: u64,
}

impl<W: Write> Write for IpcOutput<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let count = self.output.write(bytes)?;
        self.len = self
            .len
            .checked_add(count)
            .ok_or_else(|| io::Error::other("IPC stream length overflow"))?;
        self.checksum = extend_stable_hash(self.checksum, &bytes[..count]);
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.output.flush()
    }
}
