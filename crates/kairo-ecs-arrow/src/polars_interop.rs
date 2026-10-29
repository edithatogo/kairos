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

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static EXECUTION_THREAD_BUDGET: AtomicUsize = AtomicUsize::new(0);

/// Set or inspect the maximum worker thread budget for Polars executions.
pub fn get_or_set_execution_thread_budget(threads: usize) -> usize {
    if threads > 0 {
        EXECUTION_THREAD_BUDGET.store(threads, Ordering::SeqCst);
    }
    let current = EXECUTION_THREAD_BUDGET.load(Ordering::SeqCst);
    if current == 0 {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    } else {
        current
    }
}

/// FNV-1a 64-bit checksum calculation for file verification.
pub fn calculate_checksum(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Metadata for a single published Parquet part file.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DurablePartMetadata {
    pub filename: String,
    pub row_count: usize,
    pub checksum: u64,
    pub max_sequence: u64,
}

/// Run-level manifest containing completion status, row counts, and checksums.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DurableManifest {
    pub run_id: String,
    pub schema_stream: String,
    pub total_rows: usize,
    pub watermark_sequence: u64,
    pub completed: bool,
    pub parts: Vec<DurablePartMetadata>,
}

impl DurableManifest {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("manifest contains only JSON-representable values")
    }

    pub fn from_json(json: &str) -> Result<Self, ArrowError> {
        serde_json::from_str(json)
            .map_err(|e| ArrowError::Io(format!("Invalid manifest JSON: {e}")))
    }
}

fn valid_component(value: &str) -> bool {
    use std::path::Component;
    let mut components = std::path::Path::new(value).components();
    !value.contains('\\')
        && matches!(components.next(), Some(Component::Normal(part)) if part == value)
        && components.next().is_none()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicationReceipt {
    pub manifest_path: PathBuf,
    pub total_rows: usize,
    pub parts_count: usize,
    pub watermark_sequence: u64,
}

pub struct DurableRunPublicationWriter {
    run_dir: PathBuf,
    run_id: String,
    batch_size: usize,
}

impl DurableRunPublicationWriter {
    pub fn new(base_dir: impl AsRef<Path>, run_id: impl Into<String>, batch_size: usize) -> Self {
        let r_id = run_id.into();
        let run_dir = base_dir.as_ref().join(&r_id);
        Self {
            run_dir,
            run_id: r_id,
            batch_size: if batch_size == 0 { 10_000 } else { batch_size },
        }
    }

    /// Publications always refuse to reuse an existing run directory.
    pub fn with_no_clobber(self) -> Self {
        self
    }

    /// Retained for source compatibility; completed or interrupted runs are
    /// never overwritten, regardless of the requested setting.
    pub fn with_overwrite(self, _allow_overwrite: bool) -> Self {
        self
    }

    pub fn publish(&self, records: &[EventLogRecord]) -> Result<PublicationReceipt, ArrowError> {
        if !valid_component(&self.run_id) {
            return Err(ArrowError::Io("Invalid run identifier".to_string()));
        }
        let parent = self
            .run_dir
            .parent()
            .ok_or_else(|| ArrowError::Io("Missing run parent".to_string()))?;
        fs::create_dir_all(parent).map_err(|e| ArrowError::Io(e.to_string()))?;
        // An exclusive directory creation prevents concurrent publishers or a retry
        // from modifying any completed or interrupted run.
        fs::create_dir(&self.run_dir)
            .map_err(|e| ArrowError::Io(format!("Cannot create unique run directory: {e}")))?;
        #[cfg(unix)]
        fs::File::open(parent)
            .and_then(|dir| dir.sync_all())
            .map_err(|e| ArrowError::Io(format!("Cannot sync run parent directory: {e}")))?;

        // Explicit canonical sort keys: time_ticks, priority, sequence
        let mut sorted_records = records.to_vec();
        sorted_records.sort_by(|a, b| {
            a.time_ticks
                .cmp(&b.time_ticks)
                .then_with(|| a.priority.cmp(&b.priority))
                .then_with(|| a.sequence.cmp(&b.sequence))
        });

        let mut parts = Vec::new();
        let total_rows = sorted_records.len();
        let mut watermark_sequence = 0u64;

        for (part_idx, chunk) in sorted_records.chunks(self.batch_size).enumerate() {
            let part_name = format!("part-{part_idx:05}.parquet");
            let part_path = self.run_dir.join(&part_name);

            let mut df = records_to_dataframe(chunk)?;
            let mut buf = Vec::new();
            write_parquet_zstd(&mut df, &mut buf)?;

            crate::atomic_write_file(&part_path, &buf, false)?;
            let checksum = calculate_checksum(&buf);

            let max_seq = chunk.iter().map(|r| r.sequence).max().unwrap_or(0);
            if max_seq > watermark_sequence {
                watermark_sequence = max_seq;
            }

            parts.push(DurablePartMetadata {
                filename: part_name,
                row_count: chunk.len(),
                checksum,
                max_sequence: max_seq,
            });
        }

        // Manifest-last publication: only write manifest after all parts are flushed
        let manifest = DurableManifest {
            run_id: self.run_id.clone(),
            schema_stream: crate::EVENT_LOG_STREAM_V2.to_string(),
            total_rows,
            watermark_sequence,
            completed: true,
            parts: parts.clone(),
        };

        let manifest_json = manifest.to_json();
        let manifest_path = self.run_dir.join("manifest.json");
        crate::atomic_write_file(&manifest_path, manifest_json.as_bytes(), false)?;

        Ok(PublicationReceipt {
            manifest_path,
            total_rows,
            parts_count: parts.len(),
            watermark_sequence,
        })
    }

    pub fn filter_new_records_since_watermark(
        &self,
        records: &[EventLogRecord],
        watermark: u64,
    ) -> Vec<EventLogRecord> {
        records
            .iter()
            .filter(|r| r.sequence > watermark)
            .cloned()
            .collect()
    }
}

#[cfg(unix)]
type OpenRunDirectory = fs::File;
#[cfg(not(unix))]
type OpenRunDirectory = PathBuf;

#[cfg(unix)]
fn open_run_directory(path: &Path) -> Result<OpenRunDirectory, ArrowError> {
    use rustix::fs::{open, Mode, OFlags};
    let fd = open(
        path,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|e| ArrowError::Io(format!("Cannot open run directory: {e}")))?;
    Ok(fs::File::from(fd))
}

#[cfg(not(unix))]
fn open_run_directory(path: &Path) -> Result<OpenRunDirectory, ArrowError> {
    let metadata = fs::symlink_metadata(path).map_err(|e| ArrowError::Io(e.to_string()))?;
    if !metadata.file_type().is_dir() {
        return Err(ArrowError::Io("Run path is not a directory".to_string()));
    }
    Ok(path.to_path_buf())
}

#[cfg(unix)]
fn read_run_file(directory: &OpenRunDirectory, name: &str) -> Result<Vec<u8>, ArrowError> {
    use rustix::fs::{openat, Mode, OFlags};
    use std::io::Read;
    let fd = openat(
        directory,
        name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
        Mode::empty(),
    )
    .map_err(|e| ArrowError::Io(format!("Cannot open run file: {e}")))?;
    let mut file = fs::File::from(fd);
    if !file
        .metadata()
        .map_err(|e| ArrowError::Io(e.to_string()))?
        .is_file()
    {
        return Err(ArrowError::Io("Run file is not regular".to_string()));
    }
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|e| ArrowError::Io(e.to_string()))?;
    Ok(bytes)
}

#[cfg(all(test, unix))]
mod path_hardening_tests {
    use super::*;

    #[test]
    fn open_directory_remains_bound_after_path_replacement() {
        use std::os::unix::fs::symlink;
        let parent = tempfile::tempdir().unwrap();
        let run = parent.path().join("run");
        let old = parent.path().join("old");
        let other = parent.path().join("other");
        fs::create_dir(&run).unwrap();
        fs::create_dir(&other).unwrap();
        fs::write(run.join("part"), b"original").unwrap();
        fs::write(other.join("part"), b"replacement").unwrap();
        let directory = open_run_directory(&run).unwrap();
        fs::rename(&run, &old).unwrap();
        symlink(&other, &run).unwrap();
        assert_eq!(read_run_file(&directory, "part").unwrap(), b"original");
        assert!(open_run_directory(&run).is_err());
        symlink(other.join("part"), old.join("linked")).unwrap();
        assert!(read_run_file(&directory, "linked").is_err());
    }
}

#[cfg(not(unix))]
fn read_run_file(directory: &OpenRunDirectory, name: &str) -> Result<Vec<u8>, ArrowError> {
    let path = directory.join(name);
    let metadata = fs::symlink_metadata(&path).map_err(|e| ArrowError::Io(e.to_string()))?;
    if !metadata.file_type().is_file() {
        return Err(ArrowError::Io("Run file is not regular".to_string()));
    }
    fs::read(path).map_err(|e| ArrowError::Io(e.to_string()))
}

pub struct DurableRunPublicationReader {
    run_dir: PathBuf,
    valid_run_id: bool,
}

impl DurableRunPublicationReader {
    pub fn new(base_dir: impl AsRef<Path>, run_id: impl AsRef<Path>) -> Self {
        let run_id = run_id.as_ref();
        Self {
            run_dir: base_dir.as_ref().join(run_id),
            valid_run_id: run_id.to_str().is_some_and(valid_component),
        }
    }

    fn open_run(&self) -> Result<OpenRunDirectory, ArrowError> {
        if !self.valid_run_id {
            return Err(ArrowError::Io("Invalid run identifier".to_string()));
        }
        open_run_directory(&self.run_dir)
    }

    pub fn read_manifest(&self) -> Result<DurableManifest, ArrowError> {
        self.read_manifest_at(&self.open_run()?)
    }

    fn read_manifest_at(
        &self,
        directory: &OpenRunDirectory,
    ) -> Result<DurableManifest, ArrowError> {
        let bytes = read_run_file(directory, "manifest.json")?;
        let content = std::str::from_utf8(&bytes).map_err(|e| ArrowError::Io(e.to_string()))?;
        let manifest = DurableManifest::from_json(content)?;
        if !manifest.completed
            || manifest.schema_stream != crate::EVENT_LOG_STREAM_V2
            || self.run_dir.file_name() != Some(std::ffi::OsStr::new(&manifest.run_id))
        {
            return Err(ArrowError::Io(
                "Run manifest is incomplete or has an unsupported schema".to_string(),
            ));
        }
        let mut names = std::collections::HashSet::new();
        let mut rows = 0usize;
        let mut watermark = 0u64;
        for part in &manifest.parts {
            if !valid_component(&part.filename)
                || !names.insert(&part.filename)
                || part.row_count == 0
            {
                return Err(ArrowError::Io(
                    "Invalid or duplicate manifest part".to_string(),
                ));
            }
            rows = rows
                .checked_add(part.row_count)
                .ok_or_else(|| ArrowError::Io("Manifest row count overflow".to_string()))?;
            watermark = watermark.max(part.max_sequence);
        }
        if rows != manifest.total_rows || watermark != manifest.watermark_sequence {
            return Err(ArrowError::Io(
                "Manifest totals do not match its parts".to_string(),
            ));
        }
        Ok(manifest)
    }

    pub fn get_watermark_sequence(&self) -> Result<u64, ArrowError> {
        let manifest = self.read_manifest()?;
        Ok(manifest.watermark_sequence)
    }

    pub fn read_all_records(&self) -> Result<Vec<EventLogRecord>, ArrowError> {
        let directory = self.open_run()?;
        let manifest = self.read_manifest_at(&directory)?;
        let mut all_records = Vec::new();

        for part in manifest.parts {
            let bytes = read_run_file(&directory, &part.filename)?;

            let checksum = calculate_checksum(&bytes);
            if checksum != part.checksum {
                return Err(ArrowError::InvalidBlockLayout(
                    "Part file checksum mismatch: data corrupted or tampered",
                ));
            }

            let df = read_parquet(Cursor::new(bytes))?;
            let records = dataframe_to_records(&df)?;
            if records.len() != part.row_count
                || records
                    .iter()
                    .map(|record| record.sequence)
                    .max()
                    .unwrap_or(0)
                    != part.max_sequence
            {
                return Err(ArrowError::Io(
                    "Manifest part metadata does not match its records".to_string(),
                ));
            }
            all_records.extend(records);
        }

        Ok(all_records)
    }
}
