use super::{
    checkpoint_data::{decode_records, digest, encode_records, BlockRow, EventRow},
    publish_checkpoint_staged, ArrowError, CheckpointFormat, CheckpointManifest, ContiguousBlock,
};
use hdf5_metno::{dataset::Layout, File, H5Type, LinkType};
use std::{fmt::Display, io::Read, mem::size_of, path::Path};

const SCHEMA: &[u8] = b"kairo-ecs-hdf5-checkpoint-v1";
const ENGINE_SCHEMA: &[u8] = b"kairo-ecs-hdf5-checkpoint-v2";
const SIGNATURE: [u8; 8] = *b"\x89HDF\r\n\x1a\n";

pub(super) fn write(
    path: &Path,
    manifest: &CheckpointManifest,
    engine: Option<&[u8]>,
) -> Result<(), ArrowError> {
    require_utf8_path(path)?;
    manifest.validate_for_restore()?;
    let (rows, text) = encode_records(&manifest.records);
    let blocks = manifest
        .blocks
        .iter()
        .map(BlockRow::from)
        .collect::<Vec<_>>();

    publish_checkpoint_staged(path, |stage| {
        let file = File::create(stage.path()).map_err(io)?;
        let schema = if engine.is_some() {
            ENGINE_SCHEMA
        } else {
            SCHEMA
        };
        write_field(&file, "schema", schema)?;
        write_field(&file, "checkpoint_id", manifest.checkpoint_id.as_bytes())?;
        write_field(&file, "run_id", manifest.run_id.as_bytes())?;
        write_field(
            &file,
            "schema_fingerprint",
            manifest.schema_fingerprint.as_bytes(),
        )?;
        write_field(&file, "blocks", &blocks)?;
        write_field(&file, "records", &rows)?;
        write_field(&file, "text", &text)?;
        write_field(&file, "legacy_checksum", &[manifest.checksum])?;
        if let Some(state) = engine {
            write_field(&file, "engine_state", state)?;
        }
        write_field(&file, "sha256", &digest(schema, manifest, engine))?;
        file.close().map_err(io)
    })
}

pub(super) fn read(
    path: &Path,
    require_engine: bool,
) -> Result<(CheckpointManifest, Option<Vec<u8>>), ArrowError> {
    require_utf8_path(path)?;
    let mut signature = [0; 8];
    std::fs::File::open(path)
        .map_err(io)?
        .read_exact(&mut signature)
        .map_err(io)?;
    if signature != SIGNATURE {
        return Err(ArrowError::InvalidCheckpointFormat {
            expected: "native hdf5",
            actual: "unrecognized file signature".into(),
        });
    }
    let file = File::open(path).map_err(io)?;
    file.iter_visit_default(|_, info| {
        if info.link_type == LinkType::Hard {
            Ok(())
        } else {
            Err(hdf5_metno::Error::Internal(
                "checkpoint datasets must be local hard links".into(),
            ))
        }
    })
    .map_err(io)?;
    let schema = if require_engine {
        ENGINE_SCHEMA
    } else {
        SCHEMA
    };
    if read_field::<u8>(&file, "schema", Some(schema.len()))? != schema {
        return Err(ArrowError::UnexpectedStreamHeader);
    }
    let rows = read_field::<EventRow>(&file, "records", None)?;
    let all_text = read_field::<u8>(&file, "text", None)?;
    let records = decode_records(rows, &all_text)?;
    let blocks = read_field::<BlockRow>(&file, "blocks", None)?
        .into_iter()
        .map(ContiguousBlock::try_from)
        .collect::<Result<Vec<_>, ArrowError>>()?;
    let [checksum] = read_field::<u64>(&file, "legacy_checksum", Some(1))?[..] else {
        return Err(ArrowError::InvalidNumber("checksum"));
    };
    let manifest = CheckpointManifest {
        checkpoint_id: read_string(&file, "checkpoint_id")?,
        format: CheckpointFormat::Hdf5,
        run_id: read_string(&file, "run_id")?,
        schema_fingerprint: read_string(&file, "schema_fingerprint")?,
        row_count: records.len(),
        blocks,
        records,
        checksum,
    };
    let engine = require_engine
        .then(|| read_field::<u8>(&file, "engine_state", None))
        .transpose()?;
    if read_field::<u8>(&file, "sha256", Some(32))? != digest(schema, &manifest, engine.as_deref())
    {
        return Err(ArrowError::InvalidNumber("checkpoint sha256"));
    }
    Ok((manifest, engine))
}

fn write_field<T: H5Type>(file: &File, name: &str, values: &[T]) -> Result<(), ArrowError> {
    file.new_dataset_builder()
        .no_chunk()
        .with_data(values)
        .create(name)
        .map_err(io)?;
    Ok(())
}

fn read_field<T: H5Type>(
    file: &File,
    name: &str,
    expected_count: Option<usize>,
) -> Result<Vec<T>, ArrowError> {
    let data = file.dataset(name).map_err(io)?;
    let [count] = data.shape()[..] else {
        return Err(ArrowError::UnexpectedFieldHeader);
    };
    if expected_count.is_some_and(|expected| count != expected) {
        return Err(ArrowError::UnexpectedFieldHeader);
    }
    let bytes = count
        .checked_mul(size_of::<T>())
        .ok_or(ArrowError::InvalidNumber("dataset size"))? as u64;
    let properties = data.dcpl().map_err(io)?;
    let within_file = bytes == 0
        || data
            .offset()
            .and_then(|start| start.checked_add(bytes))
            .is_some_and(|end| end <= file.size());
    if !data.dtype().map_err(io)?.is::<T>()
        || properties.layout() != Layout::Contiguous
        || !properties.get_external().map_err(io)?.is_empty()
        || data.storage_size() != bytes
        || !within_file
    {
        return Err(ArrowError::UnexpectedFieldHeader);
    }
    data.read_raw().map_err(io)
}

fn read_string(file: &File, name: &str) -> Result<String, ArrowError> {
    String::from_utf8(read_field::<u8>(file, name, None)?).map_err(|_| ArrowError::InvalidUtf8)
}

fn require_utf8_path(path: &Path) -> Result<(), ArrowError> {
    if path.to_str().is_none() {
        return Err(ArrowError::InvalidArgument(
            "HDF5 checkpoint paths must be UTF-8",
        ));
    }
    Ok(())
}

fn io(error: impl Display) -> ArrowError {
    ArrowError::Io(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CheckpointRecord, EventStatus};
    use kairo_ecs_types::EventId;

    #[test]
    fn native_checkpoint_preserves_empty_tables_and_explicit_empty_payloads() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("empty.h5");
        let mut manifest = CheckpointManifest {
            checkpoint_id: "initial".into(),
            format: CheckpointFormat::Hdf5,
            run_id: "native".into(),
            schema_fingerprint: crate::event_log_schema_fingerprint(),
            row_count: 0,
            blocks: Vec::new(),
            records: Vec::new(),
            checksum: 0,
        };
        manifest.checksum = manifest.calculate_checksum();
        write(&path, &manifest, None).unwrap();
        let (actual, engine) = read(&path, false).unwrap();
        assert_eq!(actual, manifest);
        assert!(engine.is_none());

        manifest.records.push(CheckpointRecord {
            event_id: EventId::new(1, 0),
            entity_id: None,
            time_ticks: 1,
            priority: 0,
            sequence: 0,
            event_kind: "custom:1".into(),
            status: EventStatus::Dispatched,
            payload_ref: Some(String::new()),
        });
        manifest.blocks.push(ContiguousBlock {
            offset_bytes: 0,
            byte_len: 4,
            row_count: 1,
        });
        manifest.row_count = 1;
        manifest.checksum = manifest.calculate_checksum();
        write(&path, &manifest, None).unwrap();
        assert_eq!(read(&path, false).unwrap(), (manifest, None));
    }
}
