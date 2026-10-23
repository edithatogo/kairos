//! Track 70 Phase 1: Polars-native tabular processing and standard interchange.
//!
//! Provides conversion between KairoECS `EventLogRecord` / `EventLogBatch` and
//! upstream Polars `DataFrame`, alongside true Parquet/Zstd and Arrow IPC writers and readers.

use crate::{ArrowError, EventLogRecord, EventStatus, SCHEMA_VERSION_V2};
use kairo_ecs_types::{EntityId, EventId};
use polars::io::mmap::MmapBytesReader;
use polars::prelude::*;
use std::io::Write;

/// Checked 128-bit tick limb subtraction with borrow arithmetic.
///
/// Computes `(hi_b, lo_b) - (hi_a, lo_a)` without floating point rounding or intermediate truncation.
/// Returns `Ok((hi_diff, lo_diff))` or `Err(ArrowError::InvalidNumber)` on underflow.
pub fn subtract_tick_limbs(
    hi_b: u64,
    lo_b: u64,
    hi_a: u64,
    lo_a: u64,
) -> Result<(u64, u64), ArrowError> {
    let (lo_diff, borrow) = lo_b.overflowing_sub(lo_a);
    let borrow_val = if borrow { 1u64 } else { 0u64 };

    let hi_sub = hi_a
        .checked_add(borrow_val)
        .ok_or(ArrowError::InvalidNumber(
            "tick limb borrow addition overflowed high limb",
        ))?;

    if hi_b < hi_sub {
        return Err(ArrowError::InvalidNumber(
            "underflow: subtrahend exceeds minuend in 128-bit tick subtraction",
        ));
    }

    let hi_diff = hi_b - hi_sub;
    Ok((hi_diff, lo_diff))
}

/// Convert a slice of [`EventLogRecord`]s into an official Polars [`DataFrame`] adhering to Schema v2.
pub fn records_to_dataframe(records: &[EventLogRecord]) -> Result<DataFrame, ArrowError> {
    if let Some(bad_record) = records
        .iter()
        .find(|r| r.schema_version != SCHEMA_VERSION_V2)
    {
        return Err(ArrowError::UnsupportedSchemaVersion(
            bad_record.schema_version,
        ));
    }

    let n = records.len();
    let mut schema_versions = Vec::with_capacity(n);
    let mut run_ids = Vec::with_capacity(n);
    let mut event_id_indexes = Vec::with_capacity(n);
    let mut event_id_gens = Vec::with_capacity(n);
    let mut entity_id_indexes: Vec<Option<u64>> = Vec::with_capacity(n);
    let mut entity_id_gens: Vec<Option<u32>> = Vec::with_capacity(n);
    let mut time_ticks_his = Vec::with_capacity(n);
    let mut time_ticks_los = Vec::with_capacity(n);
    let mut time_scales = Vec::with_capacity(n);
    let mut priorities = Vec::with_capacity(n);
    let mut sequences = Vec::with_capacity(n);
    let mut event_kinds = Vec::with_capacity(n);
    let mut statuses = Vec::with_capacity(n);
    let mut payload_refs: Vec<Option<String>> = Vec::with_capacity(n);

    for r in records {
        schema_versions.push(r.schema_version);
        run_ids.push(r.run_id.as_str());
        event_id_indexes.push(r.event_id.index);
        event_id_gens.push(r.event_id.generation);

        match r.entity_id {
            Some(ent) => {
                entity_id_indexes.push(Some(ent.index));
                entity_id_gens.push(Some(ent.generation));
            }
            None => {
                entity_id_indexes.push(None);
                entity_id_gens.push(None);
            }
        }

        let hi = (r.time_ticks >> 64) as u64;
        let lo = (r.time_ticks & 0xffff_ffff_ffff_ffff) as u64;
        time_ticks_his.push(hi);
        time_ticks_los.push(lo);

        time_scales.push(r.time_scale.as_str());
        priorities.push(r.priority);
        sequences.push(r.sequence);
        event_kinds.push(r.event_kind.as_str());
        statuses.push(r.status.as_str());
        payload_refs.push(r.payload_ref.clone());
    }

    let cols = vec![
        UInt16Chunked::from_slice("schema_version".into(), &schema_versions).into_series(),
        Series::new("run_id".into(), &run_ids),
        Series::new("event_id_index".into(), &event_id_indexes),
        Series::new("event_id_gen".into(), &event_id_gens),
        Series::new("entity_id_index".into(), &entity_id_indexes),
        Series::new("entity_id_gen".into(), &entity_id_gens),
        Series::new("time_ticks_hi".into(), &time_ticks_his),
        Series::new("time_ticks_lo".into(), &time_ticks_los),
        Series::new("time_scale".into(), &time_scales),
        Series::new("priority".into(), &priorities),
        Series::new("sequence".into(), &sequences),
        Series::new("event_kind".into(), &event_kinds),
        Series::new("status".into(), &statuses),
        Series::new("payload_ref".into(), &payload_refs),
    ];

    DataFrame::new(cols.into_iter().map(|s| s.into()).collect())
        .map_err(|e| ArrowError::Io(e.to_string()))
}

/// Reconstruct [`EventLogRecord`]s from a Schema v2 Polars [`DataFrame`].
pub fn dataframe_to_records(df: &DataFrame) -> Result<Vec<EventLogRecord>, ArrowError> {
    let row_count = df.height();
    if row_count == 0 {
        return Ok(Vec::new());
    }

    let schema_version_col = df
        .column("schema_version")
        .map_err(|e| ArrowError::Io(e.to_string()))?
        .u16()
        .map_err(|e| ArrowError::Io(e.to_string()))?;

    let run_id_col = df
        .column("run_id")
        .map_err(|e| ArrowError::Io(e.to_string()))?
        .str()
        .map_err(|e| ArrowError::Io(e.to_string()))?;

    let event_id_index_col = df
        .column("event_id_index")
        .map_err(|e| ArrowError::Io(e.to_string()))?
        .u64()
        .map_err(|e| ArrowError::Io(e.to_string()))?;

    let event_id_gen_col = df
        .column("event_id_gen")
        .map_err(|e| ArrowError::Io(e.to_string()))?
        .u32()
        .map_err(|e| ArrowError::Io(e.to_string()))?;

    let entity_id_index_col = df
        .column("entity_id_index")
        .map_err(|e| ArrowError::Io(e.to_string()))?
        .u64()
        .map_err(|e| ArrowError::Io(e.to_string()))?;

    let entity_id_gen_col = df
        .column("entity_id_gen")
        .map_err(|e| ArrowError::Io(e.to_string()))?
        .u32()
        .map_err(|e| ArrowError::Io(e.to_string()))?;

    let time_ticks_hi_col = df
        .column("time_ticks_hi")
        .map_err(|e| ArrowError::Io(e.to_string()))?
        .u64()
        .map_err(|e| ArrowError::Io(e.to_string()))?;

    let time_ticks_lo_col = df
        .column("time_ticks_lo")
        .map_err(|e| ArrowError::Io(e.to_string()))?
        .u64()
        .map_err(|e| ArrowError::Io(e.to_string()))?;

    let time_scale_col = df
        .column("time_scale")
        .map_err(|e| ArrowError::Io(e.to_string()))?
        .str()
        .map_err(|e| ArrowError::Io(e.to_string()))?;

    let priority_col = df
        .column("priority")
        .map_err(|e| ArrowError::Io(e.to_string()))?
        .i32()
        .map_err(|e| ArrowError::Io(e.to_string()))?;

    let sequence_col = df
        .column("sequence")
        .map_err(|e| ArrowError::Io(e.to_string()))?
        .u64()
        .map_err(|e| ArrowError::Io(e.to_string()))?;

    let event_kind_col = df
        .column("event_kind")
        .map_err(|e| ArrowError::Io(e.to_string()))?
        .str()
        .map_err(|e| ArrowError::Io(e.to_string()))?;

    let status_col = df
        .column("status")
        .map_err(|e| ArrowError::Io(e.to_string()))?
        .str()
        .map_err(|e| ArrowError::Io(e.to_string()))?;

    let payload_ref_col = df
        .column("payload_ref")
        .map_err(|e| ArrowError::Io(e.to_string()))?
        .str()
        .map_err(|e| ArrowError::Io(e.to_string()))?;

    let mut records = Vec::with_capacity(row_count);
    for i in 0..row_count {
        let schema_ver = schema_version_col.get(i).unwrap_or(0);
        if schema_ver != SCHEMA_VERSION_V2 {
            return Err(ArrowError::UnsupportedSchemaVersion(schema_ver));
        }

        let run_id = run_id_col.get(i).unwrap_or("").to_string();
        let ev_idx = event_id_index_col.get(i).unwrap_or(0);
        let ev_gen = event_id_gen_col.get(i).unwrap_or(0);

        let ent_idx = entity_id_index_col.get(i);
        let ent_gen = entity_id_gen_col.get(i);
        let entity_id = match (ent_idx, ent_gen) {
            (Some(idx), Some(gen)) => Some(EntityId::new(idx, gen)),
            (None, None) => None,
            _ => {
                return Err(ArrowError::InvalidBlockLayout(
                    "entity_id_index and entity_id_gen must have matching lockstep nullability",
                ));
            }
        };

        let hi = time_ticks_hi_col.get(i).unwrap_or(0) as u128;
        let lo = time_ticks_lo_col.get(i).unwrap_or(0) as u128;
        let time_ticks = (hi << 64) | lo;

        let time_scale = time_scale_col.get(i).unwrap_or("ticks").to_string();
        let priority = priority_col.get(i).unwrap_or(0);
        let sequence = sequence_col.get(i).unwrap_or(0);
        let event_kind = event_kind_col.get(i).unwrap_or("").to_string();
        let status_str = status_col.get(i).unwrap_or("dispatched");
        let status = EventStatus::try_from(status_str)?;
        let payload_ref = payload_ref_col.get(i).map(|s| s.to_string());

        records.push(EventLogRecord {
            schema_version: schema_ver,
            run_id,
            event_id: EventId::new(ev_idx, ev_gen),
            entity_id,
            time_ticks,
            time_scale,
            priority,
            sequence,
            event_kind,
            status,
            payload_ref,
        });
    }

    Ok(records)
}

/// Write a [`DataFrame`] to Parquet with Zstandard compression.
pub fn write_parquet_zstd<W: Write>(df: &mut DataFrame, writer: W) -> Result<(), ArrowError> {
    ParquetWriter::new(writer)
        .with_compression(ParquetCompression::Zstd(None))
        .finish(df)
        .map_err(|e| ArrowError::Io(e.to_string()))?;
    Ok(())
}

/// Read a [`DataFrame`] from a Parquet reader.
pub fn read_parquet<R: MmapBytesReader>(reader: R) -> Result<DataFrame, ArrowError> {
    ParquetReader::new(reader)
        .finish()
        .map_err(|e| ArrowError::Io(e.to_string()))
}

/// Write a [`DataFrame`] to an Arrow IPC stream.
pub fn write_ipc_stream<W: Write>(df: &mut DataFrame, writer: W) -> Result<(), ArrowError> {
    IpcWriter::new(writer)
        .finish(df)
        .map_err(|e| ArrowError::Io(e.to_string()))?;
    Ok(())
}

/// Read a [`DataFrame`] from an Arrow IPC stream reader.
pub fn read_ipc_stream<R: MmapBytesReader>(reader: R) -> Result<DataFrame, ArrowError> {
    IpcReader::new(reader)
        .finish()
        .map_err(|e| ArrowError::Io(e.to_string()))
}

/// Filter a DataFrame by event kind using Polars lazy expression optimization.
pub fn filter_by_event_kind(df: &DataFrame, kind: &str) -> Result<DataFrame, ArrowError> {
    df.clone()
        .lazy()
        .filter(col("event_kind").eq(lit(kind)))
        .collect()
        .map_err(|e| ArrowError::Io(e.to_string()))
}

/// Filter a DataFrame by generational entity ID using Polars lazy expressions.
pub fn filter_by_entity(
    df: &DataFrame,
    entity_index: u64,
    entity_gen: u32,
) -> Result<DataFrame, ArrowError> {
    df.clone()
        .lazy()
        .filter(
            col("entity_id_index")
                .eq(lit(entity_index))
                .and(col("entity_id_gen").eq(lit(entity_gen))),
        )
        .collect()
        .map_err(|e| ArrowError::Io(e.to_string()))
}

/// Compute aggregate event counts by status using Polars group-by expressions.
pub fn aggregate_counts_by_status(df: &DataFrame) -> Result<DataFrame, ArrowError> {
    df.clone()
        .lazy()
        .group_by([col("status")])
        .agg([len().alias("count")])
        .sort(["status"], SortMultipleOptions::default())
        .collect()
        .map_err(|e| ArrowError::Io(e.to_string()))
}
