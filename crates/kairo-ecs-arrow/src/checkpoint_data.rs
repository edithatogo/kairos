use super::{ArrowError, CheckpointManifest, CheckpointRecord, ContiguousBlock, EventStatus};
use kairo_ecs_types::{EntityId, EventId};
use sha2::{Digest, Sha256};

#[repr(C)]
#[cfg_attr(feature = "hdf5", derive(hdf5_metno::H5Type))]
pub(super) struct EventRow {
    pub(super) event_id_index: u64,
    pub(super) entity_id_index: u64,
    pub(super) time_ticks_hi: u64,
    pub(super) time_ticks_lo: u64,
    pub(super) sequence: u64,
    pub(super) kind_bytes: u64,
    pub(super) payload_bytes: u64,
    pub(super) event_id_generation: u32,
    pub(super) entity_id_generation: u32,
    pub(super) priority: i32,
    pub(super) entity_present: u8,
    pub(super) payload_present: u8,
    pub(super) status: u8,
}

#[repr(C)]
#[cfg_attr(feature = "hdf5", derive(hdf5_metno::H5Type))]
pub(super) struct BlockRow {
    pub(super) offset_bytes: u64,
    pub(super) byte_len: u64,
    pub(super) row_count: u64,
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

pub(super) fn encode_records(records: &[CheckpointRecord]) -> (Vec<EventRow>, Vec<u8>) {
    let mut text = Vec::new();
    let rows = records
        .iter()
        .map(|record| EventRow::encode(record, &mut text))
        .collect();
    (rows, text)
}

pub(super) fn decode_records(
    rows: Vec<EventRow>,
    mut text: &[u8],
) -> Result<Vec<CheckpointRecord>, ArrowError> {
    let records = rows
        .into_iter()
        .map(|row| row.decode(&mut text))
        .collect::<Result<Vec<_>, _>>()?;
    if !text.is_empty() {
        return Err(ArrowError::InvalidNumber("text length"));
    }
    Ok(records)
}

impl From<&ContiguousBlock> for BlockRow {
    fn from(block: &ContiguousBlock) -> Self {
        Self {
            offset_bytes: block.offset_bytes,
            byte_len: block.byte_len,
            row_count: block.row_count as u64,
        }
    }
}

impl TryFrom<BlockRow> for ContiguousBlock {
    type Error = ArrowError;

    fn try_from(block: BlockRow) -> Result<Self, ArrowError> {
        Ok(Self {
            offset_bytes: block.offset_bytes,
            byte_len: block.byte_len,
            row_count: usize::try_from(block.row_count)
                .map_err(|_| ArrowError::InvalidNumber("block row count"))?,
        })
    }
}

pub(super) fn digest(schema: &[u8], manifest: &CheckpointManifest) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(schema);
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
