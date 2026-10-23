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
//! # Task 3.1 contract-first state
//!
//! Every type below is the reviewed contract; every behavioral entry point
//! currently fails with [`RunOutputError::NotImplementedUntilTask32`]. The
//! failing integration tests in
//! `crates/kairo-ecs-arrow/tests/track70_phase3_durability.rs` pin the target
//! behavior and stay unchanged when Task 3.2 replaces these stubs with the
//! real implementation. No fabricated durability behavior is exposed: a
//! caller cannot observe a half-implemented publication path.
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
//! `thiserror` dependency). The full taxonomy is defined up front so the
//! failing tests can pin variant semantics; Task 3.2 only starts
//! constructing the non-stub variants.

use std::fmt::{Display, Formatter};
use std::io::Write;
use std::num::NonZeroUsize;
use std::path::Path;

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

/// Errors for run-output publication, resumption, and validation.
#[derive(Debug)]
pub enum RunOutputError {
    /// Task 3.1 contract-first stub marker: the named operation is not
    /// implemented yet. Removed in Task 3.2.
    NotImplementedUntilTask32(&'static str),
    /// A call argument is outside the documented contract (empty identity,
    /// non-canonical hash, path-escape target, out-of-order sequence,
    /// oversized part bound violation is impossible by construction).
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
    /// The native thread budget was already configured; configuration is
    /// once-per-process before Polars pool initialization.
    BudgetAlreadyConfigured,
    /// The Polars pool was initialized before the thread budget was
    /// configured, so the budget can no longer take effect.
    PoolAlreadyInitialized,
    /// Filesystem failure (including disk exhaustion during staged writes).
    Io(String),
}

impl Display for RunOutputError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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

/// Native thread budget for the Polars pool, configured exactly once per
/// process before pool initialization (spec: "Configure native thread budget
/// once before Polars pool initialization; never mutate global environment
/// concurrently").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadBudget {
    pub worker_threads: NonZeroUsize,
}

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
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Completed,
}

/// Completed-run manifest (spec "Run manifests"): run/replication identity,
/// actual effective-input hashes, schema/library/source versions, seeds,
/// units, table paths, row counts, per-part file hashes, and completion
/// status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunManifest {
    pub schema: &'static str,
    pub run_id: String,
    pub replication_id: u64,
    pub seed: u64,
    /// Units, e.g. `ticks`.
    pub time_scale: String,
    pub effective_input_hash: String,
    pub raw_input_hash: Option<String>,
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

/// An open, not-yet-completed run output directory.
///
/// Batches are published as one or more bounded parts, written staged
/// (`.staging-` prefix) and renamed into place; [`RunOutput::complete`]
/// flushes and syncs the parts and their directories, then publishes the
/// manifest last, consuming the handle so no write can follow completion.
#[derive(Debug)]
pub struct RunOutput {
    /// Task 3.2: run directory path, config, watermark, and published-part
    /// ledger.
    _private: (),
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
        let _ = (config, parent_dir);
        Err(RunOutputError::NotImplementedUntilTask32("create"))
    }

    /// Resume an interrupted run: open the existing run directory for
    /// `config` and continue publishing strictly after `boundary`.
    ///
    /// Contract: the next part continues at `boundary.part_id + 1`; every
    /// accepted record's sequence is greater than
    /// `boundary.sequence_watermark`, so resume neither loses nor duplicates
    /// records relative to the pre-checkpoint trace.
    pub fn resume(
        config: RunOutputConfig,
        parent_dir: &Path,
        boundary: CheckpointBoundary,
    ) -> Result<Self, RunOutputError> {
        let _ = (config, parent_dir, boundary);
        Err(RunOutputError::NotImplementedUntilTask32("resume"))
    }

    /// Publish `batch` fully as one or more bounded parts.
    ///
    /// Contract: rows are canonically ordered inside each part (explicit sort
    /// keys); each part is written staged beside its destination, renamed
    /// into place, and attested with a content hash plus its inclusive
    /// min/max sequence range; a part's row count never exceeds
    /// `max_rows_per_part`; the whole batch is published (split across parts
    /// as needed) — no row is dropped silently. Every record's sequence must
    /// be greater than the run's watermark ([`RunOutputError::InvalidArgument`]
    /// otherwise). Failures are explicit; a failed write records no part.
    pub fn write_batch(
        &mut self,
        batch: &crate::EventLogBatch,
    ) -> Result<Vec<PartReceipt>, RunOutputError> {
        let _ = batch;
        Err(RunOutputError::NotImplementedUntilTask32("write_batch"))
    }

    /// Publish `batch` fully to a caller-provided sink with the same contract
    /// as [`RunOutput::write_batch`]. This is the injection point for
    /// disk-exhaustion testing (a sink that returns
    /// `io::ErrorKind::StorageFull`) and for streaming sinks; the receipts'
    /// hashes attest the bytes handed to the sink.
    pub fn write_batch_to(
        &mut self,
        batch: &crate::EventLogBatch,
        sink: &mut dyn Write,
    ) -> Result<Vec<PartReceipt>, RunOutputError> {
        let _ = (batch, sink);
        Err(RunOutputError::NotImplementedUntilTask32("write_batch_to"))
    }

    /// Parts published so far, in publication order.
    pub fn parts(&self) -> &[PartReceipt] {
        &[]
    }

    /// The current checkpoint-to-trace partition boundary, if any part has
    /// been published.
    pub fn checkpoint_boundary(&self) -> Option<CheckpointBoundary> {
        None
    }

    /// Publish the run: flush and sync every part and their directories, then
    /// write the manifest last. Consumes the handle; the manifest's presence
    /// is the completion marker.
    pub fn complete(self) -> Result<RunManifest, RunOutputError> {
        Err(RunOutputError::NotImplementedUntilTask32("complete"))
    }
}

/// Read the completed manifest of a run directory.
///
/// Contract: rejects a missing manifest ([`RunOutputError::ManifestMissing`])
/// — including interrupted runs that only have staged or renamed parts — and
/// a manifest that is malformed, carries the wrong schema identifier, or
/// lacks the completed status ([`RunOutputError::ManifestInvalid`]).
pub fn read_completed_manifest(run_dir: &Path) -> Result<RunManifest, RunOutputError> {
    let _ = run_dir;
    Err(RunOutputError::NotImplementedUntilTask32(
        "read_completed_manifest",
    ))
}

/// Validate a completed run output end to end: read the completed manifest,
/// then re-attest every part (existence, size, content hash, row counts) so
/// readers only accept runs whose files still match their manifest
/// ([`RunOutputError::HashMismatch`] on any divergence).
pub fn validate_run_output(run_dir: &Path) -> Result<RunManifest, RunOutputError> {
    let _ = run_dir;
    Err(RunOutputError::NotImplementedUntilTask32(
        "validate_run_output",
    ))
}

/// Configure the native thread budget for the Polars pool.
///
/// Contract: succeeds at most once per process
/// ([`RunOutputError::BudgetAlreadyConfigured`] afterwards), must run before
/// first pool initialization ([`RunOutputError::PoolAlreadyInitialized`]
/// otherwise), and never mutates the global environment concurrently with
/// another configuration attempt.
pub fn configure_native_thread_budget(budget: ThreadBudget) -> Result<(), RunOutputError> {
    let _ = budget;
    Err(RunOutputError::NotImplementedUntilTask32(
        "configure_native_thread_budget",
    ))
}

/// Read back the configured thread budget, if any.
pub fn native_thread_budget() -> Result<Option<ThreadBudget>, RunOutputError> {
    Err(RunOutputError::NotImplementedUntilTask32(
        "native_thread_budget",
    ))
}
