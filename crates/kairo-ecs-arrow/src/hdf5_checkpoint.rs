use super::{
    publish_checkpoint_staged, ArrowError, CheckpointFormat, CheckpointManifest, CheckpointRecord,
    ContiguousBlock, EventStatus,
};
use hdf5_metno::{dataset::Layout, File, H5Type, LinkType};
use kairo_ecs_types::{EntityId, EventId};
use sha2::{Digest, Sha256};
use std::{fmt::Display, io::Read, mem::size_of, path::Path};

const SCHEMA: &[u8] = b"kairo-ecs-hdf5-checkpoint-v1";
const SIGNATURE: [u8; 8] = *b"\x89HDF\r\n\x1a\n";

#[repr(C)]
#[derive(H5Type)]
struct EventRow {
    event_id_index: u64,
    entity_id_index: u64,
    time_ticks_hi: u64,
    time_ticks_lo: u64,
    sequence: u64,
    kind_bytes: u64,
    payload_bytes: u64,
    event_id_generation: u32,
    entity_id_generation: u32,
    priority: i32,
    entity_present: u8,
    payload_present: u8,
    status: u8,
}

#[repr(C)]
#[derive(H5Type)]
struct BlockRow {
    offset_bytes: u64,
    byte_len: u64,
    row_count: u64,
}

impl EventRow {
    fn encode(record: &CheckpointRecord, text: &mut Vec<u8>) -> Self {
        let payload = record.payload_ref.as_deref().unwrap_or_default();
        text.extend_from_slice(record.event_kind.as_bytes());
        text.extend_from_slice(payload.as_bytes());
        Self {
            event_id_index: record.event_id.index,
            entity_id_index: record.entity_id.map_or(0, |id| id.index),
            time_ticks_hi: (record.time_ticks >> 64) as u64,
            time_ticks_lo: record.time_ticks as u64,
            sequence: record.sequence,
            kind_bytes: record.event_kind.len() as u64,
            payload_bytes: payload.len() as u64,
            event_id_generation: record.event_id.generation,
            entity_id_generation: record.entity_id.map_or(0, |id| id.generation),
            priority: record.priority,
            entity_present: u8::from(record.entity_id.is_some()),
            payload_present: u8::from(record.payload_ref.is_some()),
            status: match record.status {
                EventStatus::Dispatched => 0,
                EventStatus::Cancelled => 1,
                EventStatus::Skipped => 2,
                EventStatus::Error => 3,
            },
        }
    }

    fn decode(self, text: &mut &[u8]) -> Result<CheckpointRecord, ArrowError> {
        let entity_id = match self.entity_present {
            0 if self.entity_id_index == 0 && self.entity_id_generation == 0 => None,
            1 => Some(EntityId::new(
                self.entity_id_index,
                self.entity_id_generation,
            )),
            _ => return Err(ArrowError::InvalidNumber("entity_present")),
        };
        let event_kind = take_text(text, self.kind_bytes)?;
        let payload_ref = match self.payload_present {
            0 if self.payload_bytes == 0 => None,
            1 => Some(take_text(text, self.payload_bytes)?),
            _ => return Err(ArrowError::InvalidNumber("payload_present")),
        };
        let status = match self.status {
            0 => EventStatus::Dispatched,
            1 => EventStatus::Cancelled,
            2 => EventStatus::Skipped,
            3 => EventStatus::Error,
            code => return Err(ArrowError::InvalidStatus(code.to_string())),
        };
        Ok(CheckpointRecord {
            event_id: EventId::new(self.event_id_index, self.event_id_generation),
            entity_id,
            time_ticks: (u128::from(self.time_ticks_hi) << 64) | u128::from(self.time_ticks_lo),
            priority: self.priority,
            sequence: self.sequence,
            event_kind,
            status,
            payload_ref,
        })
    }
}

fn take_text(text: &mut &[u8], length: u64) -> Result<String, ArrowError> {
    let length = usize::try_from(length).map_err(|_| ArrowError::InvalidNumber("text length"))?;
    let (value, rest) = text
        .split_at_checked(length)
        .ok_or(ArrowError::InvalidNumber("text length"))?;
    *text = rest;
    Ok(std::str::from_utf8(value)
        .map_err(|_| ArrowError::InvalidUtf8)?
        .to_owned())
}

pub(super) fn write(path: &Path, manifest: &CheckpointManifest) -> Result<(), ArrowError> {
    manifest.validate_for_restore()?;
    let mut text = Vec::new();
    let rows = manifest
        .records
        .iter()
        .map(|row| EventRow::encode(row, &mut text))
        .collect::<Vec<_>>();
    let blocks = manifest
        .blocks
        .iter()
        .map(|block| BlockRow {
            offset_bytes: block.offset_bytes,
            byte_len: block.byte_len,
            row_count: block.row_count as u64,
        })
        .collect::<Vec<_>>();

    publish_checkpoint_staged(path, |stage| {
        let file = File::create(stage.path()).map_err(io)?;
        write_field(&file, "schema", SCHEMA)?;
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
        write_field(&file, "sha256", &digest(manifest))?;
        file.close().map_err(io)
    })
}

pub(super) fn read(path: &Path) -> Result<CheckpointManifest, ArrowError> {
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
    if read_field::<u8>(&file, "schema")? != SCHEMA {
        return Err(ArrowError::UnexpectedStreamHeader);
    }
    let rows = read_field::<EventRow>(&file, "records")?;
    let all_text = read_field::<u8>(&file, "text")?;
    let mut text = all_text.as_slice();
    let records = rows
        .into_iter()
        .map(|row| row.decode(&mut text))
        .collect::<Result<Vec<_>, _>>()?;
    if !text.is_empty() {
        return Err(ArrowError::InvalidNumber("text length"));
    }
    let blocks = read_field::<BlockRow>(&file, "blocks")?
        .into_iter()
        .map(|block| {
            Ok(ContiguousBlock {
                offset_bytes: block.offset_bytes,
                byte_len: block.byte_len,
                row_count: usize::try_from(block.row_count)
                    .map_err(|_| ArrowError::InvalidNumber("block row count"))?,
            })
        })
        .collect::<Result<Vec<_>, ArrowError>>()?;
    let [checksum] = read_field::<u64>(&file, "legacy_checksum")?[..] else {
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
    if read_field::<u8>(&file, "sha256")? != digest(&manifest) {
        return Err(ArrowError::InvalidNumber("checkpoint sha256"));
    }
    Ok(manifest)
}

fn write_field<T: H5Type>(file: &File, name: &str, values: &[T]) -> Result<(), ArrowError> {
    file.new_dataset_builder()
        .with_data(values)
        .create(name)
        .map_err(io)?;
    Ok(())
}

fn read_field<T: H5Type>(file: &File, name: &str) -> Result<Vec<T>, ArrowError> {
    let data = file.dataset(name).map_err(io)?;
    let [count] = data.shape()[..] else {
        return Err(ArrowError::UnexpectedFieldHeader);
    };
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
    String::from_utf8(read_field::<u8>(file, name)?).map_err(|_| ArrowError::InvalidUtf8)
}

fn digest(manifest: &CheckpointManifest) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(SCHEMA);
    for value in [
        &manifest.checkpoint_id,
        &manifest.run_id,
        &manifest.schema_fingerprint,
    ] {
        hash_text(&mut hash, value);
    }
    hash.update((manifest.blocks.len() as u64).to_le_bytes());
    for block in &manifest.blocks {
        hash.update(block.offset_bytes.to_le_bytes());
        hash.update(block.byte_len.to_le_bytes());
        hash.update((block.row_count as u64).to_le_bytes());
    }
    hash.update((manifest.row_count as u64).to_le_bytes());
    hash.update(manifest.checksum.to_le_bytes());
    for row in &manifest.records {
        hash.update(row.event_id.index.to_le_bytes());
        hash.update(row.event_id.generation.to_le_bytes());
        hash.update([u8::from(row.entity_id.is_some())]);
        if let Some(entity) = row.entity_id {
            hash.update(entity.index.to_le_bytes());
            hash.update(entity.generation.to_le_bytes());
        }
        hash.update(row.time_ticks.to_le_bytes());
        hash.update(row.priority.to_le_bytes());
        hash.update(row.sequence.to_le_bytes());
        hash_text(&mut hash, &row.event_kind);
        hash_text(&mut hash, row.status.as_str());
        hash.update([u8::from(row.payload_ref.is_some())]);
        if let Some(payload) = &row.payload_ref {
            hash_text(&mut hash, payload);
        }
    }
    hash.finalize().into()
}

fn hash_text(hash: &mut Sha256, value: &str) {
    hash.update((value.len() as u64).to_le_bytes());
    hash.update(value.as_bytes());
}

fn io(error: impl Display) -> ArrowError {
    ArrowError::Io(error.to_string())
}
