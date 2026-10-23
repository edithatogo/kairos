//! Track 70 Phase 3: durable run-output publication for Schema v2 event logs.
//!
//! Contract surface for the spec clauses "Run manifests" and the durability
//! paragraph: unique run output directories, staged parts with final renames,
//! manifest-last publication, readers that require a completed manifest and
//! valid file hashes, a persisted checkpoint-to-trace partition boundary
//! (replication ID, part ID, sequence watermark) so resume neither loses nor
//! duplicates records, bounded batches with no unbounded queue and no silent
//! drops, and a native thread budget configured exactly once before Polars
//! pool initialization.
//!
//! # Durability model
//!
//! Parts are encoded to a bounded in-memory buffer (one part at most — the
//! memory bound is the configured part row bound), written to a `.staging-`
//! file beside their destination, fsynced, renamed into place, and recorded
//! in the ledger only after the rename. `complete` fsyncs every published
//! part and the run directory, then writes the manifest to a staging file,
//! fsyncs it, renames it into place, and fsyncs the directory again:
//! manifest-last, so the manifest's presence is the completion marker and
//! interrupted runs (before or during any staged write) have no manifest a
//! reader could accept. Filesystem rename/sync semantics are the
//! local-filesystem contract (spec: "Output publication is a
//! local-filesystem contract in this migration"); directory syncing applies
//! where the platform supports it (`cfg(unix)`).
//!
//! # Content attestation
//!
//! Part file hashes use the crate's canonical non-cryptographic content
//! checksum (FNV-1a 64, `stable_hash_bytes`), recorded in the manifest with
//! an explicit `content_hash_algorithm` label (`kairo-fnv1a64`). They attest
//! files against accidental divergence and tampering detection-by-hash
//! comparison, not cross-version byte reproducibility; a cryptographic
//! digest upgrade is Track 93's qualification scope and only requires
//! changing the labeled algorithm.
//!
//! # Ordering contract
//!
//! Capture-side sequence numbers are monotonic across writes: every record
//! handed to a write must carry a sequence greater than the highest sequence
//! already published into this run output (the watermark), so published
//! parts never overlap in sequence and the checkpoint boundary is a true
//! resume point. *Within* a part, rows are stored in the canonical
//! deterministic order (time ticks, priority, sequence) regardless of the
//! order rows arrived in the batch.
//!
//! # Isolation
//!
//! This module lives behind `polars-native` like the rest of the Track 70
//! runtime surface: default, minimal-core, and browser/Wasm builds do not
//! compile it, keeping those graphs Polars-free.
//!
//! # Error taxonomy
//!
//! Errors are hand-rolled like [`crate::ArrowError`] (this crate carries no
//! `thiserror` dependency). [`RunOutputError::NotImplementedUntilTask32`] is
//! retained as a never-constructed variant only so the Task 3.1 test suite's
//! imports keep compiling until it is retired with the Phase 3 closeout;
//! every other variant is live.

use std::io::{BufWriter, Write};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::polars_interop::{
    dataframe_to_records, read_parquet, records_to_dataframe, write_parquet_zstd,
};
use crate::{ArrowError, EventLogRecord};

/// Manifest schema identifier published with every completed run.
pub const RUN_MANIFEST_SCHEMA: &str = "kairo_ecs.run_manifest.v1";

/// File name of the completed-run manifest. Its presence in a run directory
/// is the completion marker: in-progress runs have no manifest at all
/// (manifest-last publication), so readers never need to guess run state.
pub const RUN_MANIFEST_FILE_NAME: &str = "run_manifest.json";

/// Prefix of staged part files. Staged files live in the run directory beside
/// their destination and are renamed into place; a validation pass must never
/// accept a staging leftover as a published part.
pub const PART_STAGING_PREFIX: &str = ".staging-";

/// Algorithm label recorded beside every content hash in the manifest.
pub const CONTENT_HASH_ALGORITHM: &str = "kairo-fnv1a64";

/// Published part file name prefix (bodies are `{prefix}{id:05}.parquet`).
const PART_FILE_PREFIX: &str = "part-";
const PART_FILE_SUFFIX: &str = ".parquet";

/// Errors for run-output publication, resumption, and validation.
#[derive(Debug)]
pub enum RunOutputError {
    /// Task 3.1 contract-first stub marker, retired by the Task 3.2
    /// implementation; never constructed anymore.
    NotImplementedUntilTask32(&'static str),
    /// A call argument is outside the documented contract (empty identity,
    /// non-canonical hash, path-escape target, out-of-order sequence).
    InvalidArgument(&'static str),
    /// The target run directory already holds a completed manifest; completed
    /// runs are never overwritten (fresh directory per run).
    CompletedRunExists { path: String },
    /// The run directory has no completed manifest (interrupted run or
    /// nonexistent directory).
    ManifestMissing { path: String },
    /// The manifest exists but is malformed, has the wrong schema identifier,
    /// or lacks the completion status.
    ManifestInvalid(String),
    /// A published part failed re-attestation: absent, resized, or its
    /// content hash no longer matches the manifest.
    HashMismatch { path: String },
    /// A part file that passed hash re-attestation could not be decoded back
    /// into records (writer bug or exotic corruption).
    PartDecode(String),
    /// The native thread budget was already configured; configuration is
    /// once-per-process. `PoolAlreadyInitialized` is reserved for explicit
    /// pool-init tracking; ordering the configuration before first Polars use
    /// remains a documented caller obligation.
    BudgetAlreadyConfigured,
    /// Reserved: the Polars pool was observed initialized before the thread
    /// budget was configured.
    PoolAlreadyInitialized,
    /// Filesystem failure (including disk exhaustion during staged writes).
    Io(String),
}

impl std::fmt::Display for RunOutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotImplementedUntilTask32(op) => {
                write!(f, "Track 70 Task 3.2 not yet implemented: {op}")
            }
            Self::InvalidArgument(reason) => write!(f, "invalid argument: {reason}"),
            Self::CompletedRunExists { path } => write!(
                f,
                "run output already completed at {path}; refusing to overwrite a completed run"
            ),
            Self::ManifestMissing { path } => {
                write!(f, "no completed run manifest at {path}")
            }
            Self::ManifestInvalid(reason) => write!(f, "invalid run manifest: {reason}"),
            Self::HashMismatch { path } => {
                write!(f, "published part failed hash re-attestation: {path}")
            }
            Self::PartDecode(reason) => write!(f, "published part failed to decode: {reason}"),
            Self::BudgetAlreadyConfigured => {
                write!(
                    f,
                    "native thread budget already configured for this process"
                )
            }
            Self::PoolAlreadyInitialized => {
                write!(
                    f,
                    "Polars pool already initialized; thread budget must be configured first"
                )
            }
            Self::Io(error) => write!(f, "I/O error: {error}"),
        }
    }
}

impl std::error::Error for RunOutputError {}

fn io_err(error: std::io::Error) -> RunOutputError {
    RunOutputError::Io(error.to_string())
}

fn arrow_err(error: ArrowError) -> RunOutputError {
    RunOutputError::PartDecode(error.to_string())
}

/// Native thread budget for the Polars pool, configured exactly once per
/// process before pool initialization (spec: "Configure native thread budget
/// once before Polars pool initialization; never mutate global environment
/// concurrently").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadBudget {
    pub worker_threads: NonZeroUsize,
}

static THREAD_BUDGET: OnceLock<ThreadBudget> = OnceLock::new();

/// Identity, effective-input attestation, and batch bound for one run output.
///
/// `effective_input_hash` must attest the actual effective inputs (raw versus
/// normalized preserved separately), never a scenario *name*; `seed` and
/// `time_scale` carry the randomness and units contract; `max_rows_per_part`
/// is the bounded-batch bound (spec: "Write bounded batches/per-replication
/// parts ... No unbounded queue and no silent trace drops").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunOutputConfig {
    pub run_id: String,
    pub replication_id: u64,
    pub seed: u64,
    pub time_scale: String,
    /// Hex digest of the actual effective inputs.
    pub effective_input_hash: String,
    /// Raw-input digest preserved separately when both exist.
    pub raw_input_hash: Option<String>,
    pub max_rows_per_part: NonZeroUsize,
}

/// One published per-replication part: identity, bounded row count, content
/// hash, and the inclusive *minimum and maximum* event-log sequence numbers
/// of the rows it contains (capture-side ordering keys; rows inside the part
/// are canonically sorted, so file order may differ from sequence order).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartReceipt {
    pub part_id: u32,
    /// Part file name relative to the run directory.
    pub file_name: String,
    pub row_count: usize,
    /// Hex content hash attesting the published file bytes.
    pub file_hash: String,
    /// Minimum event-log sequence number among the part's rows.
    pub sequence_first: u64,
    /// Maximum event-log sequence number among the part's rows.
    pub sequence_last: u64,
}

/// Checkpoint-to-trace partition boundary persisted with the checkpoint so a
/// resumed process continues the trace exactly after the watermark: no lost
/// records, no duplicated records (spec durability clause).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckpointBoundary {
    pub replication_id: u64,
    pub part_id: u32,
    /// Highest event-log sequence number published before the checkpoint.
    pub sequence_watermark: u64,
}

/// Completion status carried by a published manifest. The type has exactly
/// one variant by design: manifest-last publication means a manifest exists
/// only for a completed run, and readers reject anything else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunStatus {
    Completed,
}

/// Completed-run manifest (spec "Run manifests"): run/replication identity,
/// actual effective-input hashes, schema/library/source versions, seeds,
/// units, table paths, row counts, per-part file hashes, and completion
/// status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunManifest {
    pub schema: String,
    pub run_id: String,
    pub replication_id: u64,
    pub seed: u64,
    /// Units, e.g. `ticks`.
    pub time_scale: String,
    pub effective_input_hash: String,
    pub raw_input_hash: Option<String>,
    /// Algorithm label for the content hashes in this manifest.
    pub content_hash_algorithm: String,
    /// Schema v2 fingerprint of the exported event-log columns.
    pub schema_fingerprint: String,
    /// `kairo-ecs-arrow` crate version at write time.
    pub library_version: String,
    /// Source revision recorded by the build when available.
    pub source_version: String,
    /// Run directory–relative directory holding the published parts.
    pub table_dir: String,
    pub parts: Vec<PartReceipt>,
    pub status: RunStatus,
}

fn part_file_name(part_id: u32) -> String {
    format!("{PART_FILE_PREFIX}{part_id:05}{PART_FILE_SUFFIX}")
}

fn parse_part_id(file_name: &str) -> Option<u32> {
    file_name
        .strip_prefix(PART_FILE_PREFIX)?
        .strip_suffix(PART_FILE_SUFFIX)?
        .parse::<u32>()
        .ok()
}

fn validate_hex(value: &str, field: &'static str) -> Result<(), RunOutputError> {
    if value.is_empty()
        || !value.len().is_multiple_of(2)
        || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(RunOutputError::InvalidArgument(field));
    }
    Ok(())
}

fn validate_identity(config: &RunOutputConfig) -> Result<(), RunOutputError> {
    let run_id = &config.run_id;
    if run_id.is_empty()
        || run_id.contains('/')
        || run_id.contains('\\')
        || run_id.contains("..")
        || run_id.chars().any(char::is_control)
    {
        return Err(RunOutputError::InvalidArgument(
            "run_id must be a single non-empty path-safe component",
        ));
    }
    validate_hex(
        &config.effective_input_hash,
        "effective_input_hash must be a non-empty even-length hex string",
    )?;
    if let Some(raw) = &config.raw_input_hash {
        validate_hex(
            raw,
            "raw_input_hash must be a non-empty even-length hex string",
        )?;
    }
    if config.time_scale.trim().is_empty() {
        return Err(RunOutputError::InvalidArgument(
            "time_scale must not be empty",
        ));
    }
    Ok(())
}

fn run_directory_name(config: &RunOutputConfig) -> String {
    format!("{}-rep{}", config.run_id, config.replication_id)
}

/// Encode one bounded part (a canonical-order slice of the batch dataframe)
/// to Parquet Zstd bytes.
fn encode_part(slice: &mut polars::prelude::DataFrame) -> Result<Vec<u8>, RunOutputError> {
    let mut buffer = Vec::new();
    write_parquet_zstd(slice, &mut buffer).map_err(arrow_err)?;
    Ok(buffer)
}

/// Sort the records of a batch window into the canonical deterministic order
/// (time ticks, priority, sequence) — the explicit sort keys.
fn canonical_order_window(records: &[EventLogRecord]) -> Vec<EventLogRecord> {
    let mut window: Vec<EventLogRecord> = records.to_vec();
    window.sort_by(|left, right| {
        left.time_ticks
            .cmp(&right.time_ticks)
            .then(left.priority.cmp(&right.priority))
            .then(left.sequence.cmp(&right.sequence))
    });
    window
}

fn sequence_range(records: &[EventLogRecord]) -> (u64, u64) {
    let mut first = u64::MAX;
    let mut last = 0u64;
    for record in records {
        first = first.min(record.sequence);
        last = last.max(record.sequence);
    }
    (first, last)
}

/// An open, not-yet-completed run output directory.
///
/// Batches are published as one or more bounded parts, written staged
/// (`.staging-` prefix) and renamed into place; [`RunOutput::complete`]
/// flushes and syncs the parts and their directories, then publishes the
/// manifest last, consuming the handle so no write can follow completion.
#[derive(Debug)]
pub struct RunOutput {
    run_dir: PathBuf,
    config: RunOutputConfig,
    parts: Vec<PartReceipt>,
    next_part_id: u32,
    watermark: u64,
}

impl RunOutput {
    /// Create a unique run output directory under `parent_dir` for
    /// `config`.
    ///
    /// Contract: the directory name derives deterministically from the
    /// run/replication identity (sanitized); path containment is validated
    /// (no `..` components, no separators in identity fields, non-empty
    /// identity); an existing directory for the same identity is refused —
    /// completed ([`RunOutputError::CompletedRunExists`]) or interrupted
    /// ([`RunOutputError::InvalidArgument`], pointing at [`RunOutput::resume`])
    /// alike — so a run is never silently rewritten.
    pub fn create(config: RunOutputConfig, parent_dir: &Path) -> Result<Self, RunOutputError> {
        validate_identity(&config)?;
        let run_dir = parent_dir.join(run_directory_name(&config));
        if run_dir.exists() {
            if run_dir.join(RUN_MANIFEST_FILE_NAME).is_file() {
                return Err(RunOutputError::CompletedRunExists {
                    path: run_dir.display().to_string(),
                });
            }
            return Err(RunOutputError::InvalidArgument(
                "interrupted run output exists for this identity; resume it instead of recreating",
            ));
        }
        std::fs::create_dir_all(&run_dir).map_err(io_err)?;
        Ok(Self {
            run_dir,
            config,
            parts: Vec::new(),
            next_part_id: 1,
            watermark: 0,
        })
    }

    /// Resume an interrupted run: open the existing run directory for
    /// `config` and continue publishing strictly after `boundary`.
    ///
    /// Contract: the existing parts are re-attested (hash, row counts,
    /// sequence ranges, monotonicity), the next part continues at
    /// `boundary.part_id + 1`, and every accepted record's sequence is
    /// greater than `boundary.sequence_watermark`, so resume neither loses
    /// nor duplicates records relative to the pre-checkpoint trace. A stale
    /// or tampered boundary is rejected explicitly.
    pub fn resume(
        config: RunOutputConfig,
        parent_dir: &Path,
        boundary: CheckpointBoundary,
    ) -> Result<Self, RunOutputError> {
        validate_identity(&config)?;
        let run_dir = parent_dir.join(run_directory_name(&config));
        if !run_dir.is_dir() {
            return Err(RunOutputError::ManifestMissing {
                path: run_dir.display().to_string(),
            });
        }
        if run_dir.join(RUN_MANIFEST_FILE_NAME).is_file() {
            return Err(RunOutputError::CompletedRunExists {
                path: run_dir.display().to_string(),
            });
        }
        if boundary.replication_id != config.replication_id {
            return Err(RunOutputError::InvalidArgument(
                "checkpoint boundary replication does not match the run identity",
            ));
        }

        let mut entries: Vec<(u32, String)> = std::fs::read_dir(&run_dir)
            .map_err(io_err)?
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                let part_id = parse_part_id(&name)?;
                Some((part_id, name))
            })
            .collect();
        entries.sort_unstable();

        let mut parts = Vec::with_capacity(entries.len());
        let mut watermark = 0u64;
        for (index, (part_id, file_name)) in entries.into_iter().enumerate() {
            let expected_id = u32::try_from(index).expect("part count fits u32") + 1;
            if part_id != expected_id {
                return Err(RunOutputError::InvalidArgument(
                    "existing part files are not a contiguous sequence",
                ));
            }
            let path = run_dir.join(&file_name);
            let bytes = std::fs::read(&path).map_err(io_err)?;
            let file_hash = format!("{:016x}", crate::stable_hash_bytes(&bytes));
            let frame = read_parquet(std::io::Cursor::new(bytes)).map_err(arrow_err)?;
            let records = dataframe_to_records(&frame).map_err(arrow_err)?;
            if frame.height() != records.len() {
                return Err(RunOutputError::PartDecode(
                    "decoded part row count mismatch".to_string(),
                ));
            }
            if records.is_empty() {
                return Err(RunOutputError::PartDecode(
                    "existing part decodes to zero rows".to_string(),
                ));
            }
            let (sequence_first, sequence_last) = sequence_range(&records);
            if sequence_first <= watermark {
                return Err(RunOutputError::InvalidArgument(
                    "existing part files violate capture-side sequence monotonicity",
                ));
            }
            watermark = sequence_last;
            parts.push(PartReceipt {
                part_id,
                file_name,
                row_count: frame.height(),
                file_hash,
                sequence_first,
                sequence_last,
            });
        }

        let last_part_id = parts.last().map(|part| part.part_id).unwrap_or(0);
        if boundary.part_id != last_part_id || boundary.sequence_watermark != watermark {
            return Err(RunOutputError::InvalidArgument(
                "checkpoint boundary does not match the run directory's published state",
            ));
        }

        Ok(Self {
            run_dir,
            config,
            parts,
            next_part_id: last_part_id + 1,
            watermark,
        })
    }

    /// Publish `records` fully as one or more bounded parts.
    ///
    /// Contract: the records are Schema v2 event-log rows; rows are
    /// canonically ordered inside each part (explicit sort keys); each part
    /// is written staged beside its destination, fsynced, renamed into
    /// place, and attested with a content hash plus its inclusive min/max
    /// sequence range; a part's row count never exceeds
    /// `max_rows_per_part`; the whole window is published (split across
    /// parts as needed) — no row is dropped silently. Every record's
    /// sequence must be greater than the run's watermark
    /// ([`RunOutputError::InvalidArgument`] otherwise). Failures are
    /// explicit; a failed write records no part.
    pub fn write_batch(
        &mut self,
        records: &[EventLogRecord],
    ) -> Result<Vec<PartReceipt>, RunOutputError> {
        if records.is_empty() {
            return Ok(Vec::new());
        }
        let window_min = records
            .iter()
            .map(|record| record.sequence)
            .min()
            .expect("non-empty records");
        if window_min <= self.watermark {
            return Err(RunOutputError::InvalidArgument(
                "batch contains a sequence at or below the published watermark; capture-side sequences must be monotonic",
            ));
        }

        let bound = self.config.max_rows_per_part.get();
        let total = records.len();
        let mut receipts = Vec::new();
        let mut offset = 0usize;
        while offset < total {
            let len = bound.min(total - offset);
            let window = canonical_order_window(&records[offset..offset + len]);
            let (sequence_first, sequence_last) = sequence_range(&window);
            let part_id = self.next_part_id;
            let file_name = part_file_name(part_id);
            let mut slice = records_to_dataframe(&window).map_err(arrow_err)?;
            let encoded = encode_part(&mut slice)?;

            let staging_path = self
                .run_dir
                .join(format!("{PART_STAGING_PREFIX}{file_name}"));
            {
                let file = std::fs::File::create(&staging_path).map_err(io_err)?;
                let mut writer = BufWriter::new(file);
                writer.write_all(&encoded).map_err(io_err)?;
                writer.flush().map_err(io_err)?;
            }
            sync_path(&staging_path)?;
            let final_path = self.run_dir.join(&file_name);
            std::fs::rename(&staging_path, &final_path).map_err(io_err)?;
            sync_path(&self.run_dir)?;

            let receipt = PartReceipt {
                part_id,
                file_name: file_name.clone(),
                row_count: len,
                file_hash: format!("{:016x}", crate::stable_hash_bytes(&encoded)),
                sequence_first,
                sequence_last,
            };
            self.parts.push(receipt.clone());
            self.next_part_id += 1;
            self.watermark = self.watermark.max(sequence_last);
            receipts.push(receipt);
            offset += len;
        }
        Ok(receipts)
    }

    /// Publish `records` fully to a caller-provided sink with the same
    /// contract as [`RunOutput::write_batch`] (Schema v2 records, bounded
    /// parts, canonical order, watermark monotonicity, no silent drops).
    /// This is the injection point for disk-exhaustion testing (a sink that
    /// returns `io::ErrorKind::StorageFull`) and for streaming sinks; the
    /// receipts' hashes attest the exact bytes handed to the sink. Sink
    /// writes that fail mid-part record no receipt.
    pub fn write_batch_to(
        &mut self,
        records: &[EventLogRecord],
        sink: &mut dyn Write,
    ) -> Result<Vec<PartReceipt>, RunOutputError> {
        if records.is_empty() {
            return Ok(Vec::new());
        }
        let window_min = records
            .iter()
            .map(|record| record.sequence)
            .min()
            .expect("non-empty records");
        if window_min <= self.watermark {
            return Err(RunOutputError::InvalidArgument(
                "batch contains a sequence at or below the published watermark; capture-side sequences must be monotonic",
            ));
        }

        let bound = self.config.max_rows_per_part.get();
        let total = records.len();
        let mut receipts = Vec::new();
        let mut offset = 0usize;
        while offset < total {
            let len = bound.min(total - offset);
            let window = canonical_order_window(&records[offset..offset + len]);
            let (sequence_first, sequence_last) = sequence_range(&window);
            let part_id = self.next_part_id;
            let file_name = part_file_name(part_id);
            let mut slice = records_to_dataframe(&window).map_err(arrow_err)?;
            let encoded = encode_part(&mut slice)?;
            sink.write_all(&encoded).map_err(io_err)?;
            sink.flush().map_err(io_err)?;

            let receipt = PartReceipt {
                part_id,
                file_name: file_name.clone(),
                row_count: len,
                file_hash: format!("{:016x}", crate::stable_hash_bytes(&encoded)),
                sequence_first,
                sequence_last,
            };
            self.parts.push(receipt.clone());
            self.next_part_id += 1;
            self.watermark = self.watermark.max(sequence_last);
            receipts.push(receipt);
            offset += len;
        }
        Ok(receipts)
    }

    /// Parts published so far, in publication order.
    pub fn parts(&self) -> &[PartReceipt] {
        &self.parts
    }

    /// The current checkpoint-to-trace partition boundary, if any part has
    /// been published.
    pub fn checkpoint_boundary(&self) -> Option<CheckpointBoundary> {
        let last = self.parts.last()?;
        Some(CheckpointBoundary {
            replication_id: self.config.replication_id,
            part_id: last.part_id,
            sequence_watermark: last.sequence_last,
        })
    }

    /// Publish the run: fsync every part and the run directory, then write
    /// the manifest (staged, fsynced, renamed, directory fsynced) last.
    /// Consumes the handle; the manifest's presence is the completion marker.
    pub fn complete(self) -> Result<RunManifest, RunOutputError> {
        for part in &self.parts {
            sync_path(&self.run_dir.join(&part.file_name))?;
        }
        sync_path(&self.run_dir)?;

        let manifest = RunManifest {
            schema: RUN_MANIFEST_SCHEMA.to_string(),
            run_id: self.config.run_id.clone(),
            replication_id: self.config.replication_id,
            seed: self.config.seed,
            time_scale: self.config.time_scale.clone(),
            effective_input_hash: self.config.effective_input_hash.clone(),
            raw_input_hash: self.config.raw_input_hash.clone(),
            content_hash_algorithm: CONTENT_HASH_ALGORITHM.to_string(),
            schema_fingerprint: crate::event_log_v2_schema_fingerprint(),
            library_version: env!("CARGO_PKG_VERSION").to_string(),
            source_version: option_env!("KAIRO_BUILD_COMMIT")
                .unwrap_or("unknown")
                .to_string(),
            table_dir: ".".to_string(),
            parts: self.parts.clone(),
            status: RunStatus::Completed,
        };

        let payload = serde_json::to_vec_pretty(&manifest)
            .map_err(|error| RunOutputError::Io(error.to_string()))?;
        let staging_path = self
            .run_dir
            .join(format!("{PART_STAGING_PREFIX}{RUN_MANIFEST_FILE_NAME}"));
        {
            let file = std::fs::File::create(&staging_path).map_err(io_err)?;
            let mut writer = BufWriter::new(file);
            writer.write_all(&payload).map_err(io_err)?;
            writer.flush().map_err(io_err)?;
        }
        sync_path(&staging_path)?;
        let manifest_path = self.run_dir.join(RUN_MANIFEST_FILE_NAME);
        std::fs::rename(&staging_path, &manifest_path).map_err(io_err)?;
        sync_path(&self.run_dir)?;
        Ok(manifest)
    }
}

/// Flush a file's (or directory's) contents to stable storage where the
/// platform supports it (`cfg(unix)`); other platforms skip the explicit sync
/// and rely on close ordering (documented platform caveat).
fn sync_path(path: &Path) -> Result<(), RunOutputError> {
    #[cfg(unix)]
    {
        std::fs::File::open(path)
            .map_err(io_err)?
            .sync_all()
            .map_err(io_err)?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

/// Read the completed manifest of a run directory.
///
/// Contract: rejects a missing manifest ([`RunOutputError::ManifestMissing`])
/// — including interrupted runs that only have staged or renamed parts — and
/// a manifest that is malformed, carries the wrong schema identifier, or
/// lacks the completed status ([`RunOutputError::ManifestInvalid`]).
pub fn read_completed_manifest(run_dir: &Path) -> Result<RunManifest, RunOutputError> {
    let manifest_path = run_dir.join(RUN_MANIFEST_FILE_NAME);
    if !manifest_path.is_file() {
        return Err(RunOutputError::ManifestMissing {
            path: manifest_path.display().to_string(),
        });
    }
    let payload = std::fs::read(&manifest_path).map_err(io_err)?;
    let manifest: RunManifest = serde_json::from_slice(&payload)
        .map_err(|error| RunOutputError::ManifestInvalid(error.to_string()))?;
    if manifest.schema != RUN_MANIFEST_SCHEMA {
        return Err(RunOutputError::ManifestInvalid(format!(
            "unsupported manifest schema: {}",
            manifest.schema
        )));
    }
    Ok(manifest)
}

/// Validate a completed run output end to end: read the completed manifest,
/// then re-attest every part (existence, content hash, row counts, sequence
/// ranges) so readers only accept runs whose files still match their manifest
/// ([`RunOutputError::HashMismatch`] on any divergence).
pub fn validate_run_output(run_dir: &Path) -> Result<RunManifest, RunOutputError> {
    let manifest = read_completed_manifest(run_dir)?;
    for part in &manifest.parts {
        if part.file_name.contains('/')
            || part.file_name.contains('\\')
            || part.file_name.contains("..")
        {
            return Err(RunOutputError::ManifestInvalid(
                "manifest part path escapes the run directory".to_string(),
            ));
        }
        let path = run_dir.join(&part.file_name);
        let bytes = std::fs::read(&path).map_err(|_| RunOutputError::HashMismatch {
            path: path.display().to_string(),
        })?;
        let recorded = u64::from_str_radix(&part.file_hash, 16).map_err(|_| {
            RunOutputError::ManifestInvalid(format!(
                "part hash is not valid hex: {}",
                part.file_hash
            ))
        })?;
        if crate::stable_hash_bytes(&bytes) != recorded {
            return Err(RunOutputError::HashMismatch {
                path: path.display().to_string(),
            });
        }
        let frame = read_parquet(std::io::Cursor::new(bytes)).map_err(arrow_err)?;
        if frame.height() != part.row_count {
            return Err(RunOutputError::HashMismatch {
                path: path.display().to_string(),
            });
        }
        let records = dataframe_to_records(&frame).map_err(arrow_err)?;
        if records.is_empty() {
            return Err(RunOutputError::PartDecode(
                "validated part decodes to zero rows".to_string(),
            ));
        }
        let (sequence_first, sequence_last) = sequence_range(&records);
        if sequence_first != part.sequence_first || sequence_last != part.sequence_last {
            return Err(RunOutputError::HashMismatch {
                path: path.display().to_string(),
            });
        }
    }
    Ok(manifest)
}

/// Configure the native thread budget for the Polars pool.
///
/// Contract: succeeds at most once per process
/// ([`RunOutputError::BudgetAlreadyConfigured`] afterwards); the winning
/// caller alone records `POLARS_MAX_THREADS` before first Polars pool use, so
/// the global environment is never mutated concurrently. Ordering the
/// configuration before the first Polars use is the caller's documented
/// obligation ([`RunOutputError::PoolAlreadyInitialized`] is reserved for
/// explicit pool-init tracking).
pub fn configure_native_thread_budget(budget: ThreadBudget) -> Result<(), RunOutputError> {
    if THREAD_BUDGET.set(budget).is_err() {
        return Err(RunOutputError::BudgetAlreadyConfigured);
    }
    // Only the OnceLock winner reaches this point, so the global environment
    // is mutated exactly once per process and never concurrently with a rival
    // configuration attempt.
    // SAFETY-free on edition 2021: std::env::set_var is safe here.
    std::env::set_var(
        "POLARS_MAX_THREADS",
        budget.worker_threads.get().to_string(),
    );
    Ok(())
}

/// Read back the configured thread budget, if any.
pub fn native_thread_budget() -> Result<Option<ThreadBudget>, RunOutputError> {
    Ok(THREAD_BUDGET.get().copied())
}
