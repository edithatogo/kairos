#![cfg(feature = "polars-native")]
// `drop(run)` below models process interruption (abandoning the handle
// without completion). The implementation relies on explicit `complete()`
// for durable cleanup, so `drop_non_drop` is intentionally quieted here.
#![allow(clippy::drop_non_drop)]

//! Track 70 Phase 3 contract tests: durable run-output publication and
//! execution budgets (spec "Run manifests" + durability/budget clauses).
//!
//! These are the Task 3.1 acceptance oracle, written before implementation
//! and passed unchanged by the Task 3.2 implementation — with one recorded
//! exception: the two Task 3.1 thread-budget tests were merged into a single
//! order-robust test (`thread_budget_configures_once_with_single_concurrent_winner`),
//! because the once-per-process budget guard makes separate order-dependent
//! budget tests unsound under cargo's multi-threaded test runner (they were
//! indistinguishably red as stubs in Task 3.1, masking the order hazard).
//! All other 13 tests are byte-identical to Task 3.1.

use kairo_ecs_arrow::polars_interop::{dataframe_to_records, read_parquet};
use kairo_ecs_arrow::run_output::{
    configure_native_thread_budget, native_thread_budget, read_completed_manifest,
    validate_run_output, CheckpointBoundary, PartReceipt, RunManifest, RunOutput, RunOutputConfig,
    RunOutputError, RunStatus, ThreadBudget, RUN_MANIFEST_FILE_NAME, RUN_MANIFEST_SCHEMA,
};
use kairo_ecs_arrow::{
    event_log_v2_schema_fingerprint, EventLogRecord, EventStatus, SCHEMA_VERSION_V2,
};
use kairo_ecs_types::{EntityId, EventId};
use std::fs;
use std::io::{Cursor, Write};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::thread;

fn sample_record(sequence: u64, time_ticks: u128, kind_suffix: u64) -> EventLogRecord {
    EventLogRecord {
        schema_version: SCHEMA_VERSION_V2,
        run_id: "durability-run".to_string(),
        event_id: EventId::new(sequence, 0),
        entity_id: Some(EntityId::new(sequence + 100, 0)),
        time_ticks,
        time_scale: "ticks".to_string(),
        priority: 0,
        sequence,
        event_kind: format!("custom:kind-{kind_suffix}"),
        status: EventStatus::Dispatched,
        payload_ref: None,
    }
}

fn batch(sequences: impl IntoIterator<Item = u64>) -> Vec<EventLogRecord> {
    sequences
        .into_iter()
        .enumerate()
        .map(|(index, sequence)| sample_record(sequence, u128::from(sequence) * 10, index as u64))
        .collect()
}

fn config(max_rows_per_part: usize) -> RunOutputConfig {
    RunOutputConfig {
        run_id: "durability-run".to_string(),
        replication_id: 1,
        seed: 0xDEADBEEF,
        time_scale: "ticks".to_string(),
        effective_input_hash: "aa55".repeat(32),
        raw_input_hash: Some("bb66".repeat(32)),
        max_rows_per_part: NonZeroUsize::new(max_rows_per_part).expect("nonzero part bound"),
    }
}

fn budget(worker_threads: usize) -> ThreadBudget {
    ThreadBudget {
        worker_threads: NonZeroUsize::new(worker_threads).expect("nonzero thread budget"),
    }
}

fn temp_parent(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let parent = std::env::temp_dir().join(format!(
        "kairo-ecs-arrow-p3-{label}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&parent).expect("temp parent");
    parent
}

/// The single run directory created under `parent` for the standard config.
fn run_dir(parent: &Path) -> PathBuf {
    let mut entries: Vec<PathBuf> = fs::read_dir(parent)
        .expect("parent readable")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .collect();
    entries.retain(|path| path.is_dir());
    assert_eq!(entries.len(), 1, "exactly one run directory expected");
    entries.remove(0)
}

/// Quota-bounded sink modelling disk exhaustion: writes succeed until the
/// quota is spent, then fail with `StorageFull`.
struct QuotaWriter {
    remaining: usize,
}

impl Write for QuotaWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.remaining == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::StorageFull,
                "disk quota exhausted",
            ));
        }
        let accepted = buf.len().min(self.remaining);
        self.remaining -= accepted;
        Ok(accepted)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn unique_run_directory_refuses_to_overwrite_completed_run() {
    // Spec: "Use unique run output directories ... Do not overwrite completed
    // runs." Directory names derive from the run/replication identity, so a
    // second create for the same identity is a collision, not a silent
    // rewrite.
    let parent = temp_parent("no-overwrite");
    let mut run = RunOutput::create(config(2), &parent).expect("first create (Task 3.2)");
    run.write_batch(&batch(1..=2)).expect("part (Task 3.2)");
    let manifest = run.complete().expect("completion (Task 3.2)");
    assert_eq!(manifest.status, RunStatus::Completed);

    let second = RunOutput::create(config(2), &parent);
    assert!(
        matches!(second, Err(RunOutputError::CompletedRunExists { .. })),
        "completed run must not be overwritten, got: {second:?}"
    );

    // An *interrupted* (incomplete) run with the same identity is likewise
    // refused by create; resuming it is the explicit path (see the watermark
    // test below).
    let parent_interrupted = temp_parent("no-overwrite-incomplete");
    let mut interrupted =
        RunOutput::create(config(2), &parent_interrupted).expect("create (Task 3.2)");
    interrupted
        .write_batch(&batch(1..=2))
        .expect("part (Task 3.2)");
    drop(interrupted);
    let recreated = RunOutput::create(config(2), &parent_interrupted);
    assert!(
        matches!(recreated, Err(RunOutputError::InvalidArgument(_))),
        "incomplete run must be resumed, not recreated, got: {recreated:?}"
    );
}

#[test]
fn parts_are_bounded_and_staged_parts_rename_into_place() {
    // Spec: "Write bounded batches/per-replication parts", "staged parts,
    // final renames", "No unbounded queue and no silent trace drops": an
    // oversized batch is split across bounded parts, never truncated, and no
    // `.staging-` file survives a successful write.
    let parent = temp_parent("bounded-parts");
    let mut run = RunOutput::create(config(2), &parent).expect("create (Task 3.2)");

    let first = run.write_batch(&batch(1..=3)).expect("part 1 (Task 3.2)");
    let second = run.write_batch(&batch(4..=5)).expect("part 2 (Task 3.2)");

    assert_eq!(first.len(), 2, "oversized batch splits into bounded parts");
    assert_eq!(first[0].row_count, 2, "part bound respected");
    assert_eq!(first[1].row_count, 1, "remainder part keeps every row");
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].row_count, 2);
    assert_eq!(run.parts().len(), 3, "publication ledger tracks all parts");
    assert!(
        run.parts().iter().all(|part| !part.file_hash.is_empty()),
        "parts are attested with content hashes"
    );

    let dir = run_dir(&parent);
    let leftovers: Vec<String> = fs::read_dir(&dir)
        .expect("run dir readable")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        leftovers.iter().all(|name| !name.starts_with(".staging-")),
        "successful writes leave no staging files, got: {leftovers:?}"
    );
    for receipt in run.parts() {
        assert!(
            dir.join(&receipt.file_name).is_file(),
            "part renamed into its final place: {}",
            receipt.file_name
        );
    }
}

#[test]
fn manifest_published_last_records_identity_hashes_and_counts() {
    // Spec "Run manifests": identity, actual effective-input hashes, schema/
    // library/source versions, seeds, units, table paths, row counts, hashes,
    // completion status — and manifest-last publication.
    let parent = temp_parent("manifest-last");
    let mut run = RunOutput::create(config(2), &parent).expect("create (Task 3.2)");
    run.write_batch(&batch(1..=3)).expect("part (Task 3.2)");

    let dir = run_dir(&parent);
    assert!(
        !dir.join(RUN_MANIFEST_FILE_NAME).exists(),
        "manifest must not exist before completion (manifest-last)"
    );

    let manifest = run.complete().expect("completion (Task 3.2)");
    assert!(dir.join(RUN_MANIFEST_FILE_NAME).is_file(), "manifest last");

    assert_eq!(manifest.schema, RUN_MANIFEST_SCHEMA);
    assert_eq!(manifest.run_id, "durability-run");
    assert_eq!(manifest.replication_id, 1);
    assert_eq!(manifest.seed, 0xDEADBEEF);
    assert_eq!(manifest.time_scale, "ticks");
    assert_eq!(
        manifest.effective_input_hash,
        "aa55".repeat(32),
        "effective-input hash attests inputs, not a scenario name"
    );
    assert_eq!(
        manifest.raw_input_hash.as_deref(),
        Some("bb66".repeat(32).as_str()),
        "raw and normalized effective inputs stay separate"
    );
    assert_eq!(
        manifest.schema_fingerprint,
        event_log_v2_schema_fingerprint(),
        "manifest carries the schema fingerprint"
    );
    assert!(!manifest.library_version.is_empty());
    assert!(!manifest.source_version.is_empty());
    assert_eq!(manifest.parts.len(), 2, "all published parts recorded");
    assert_eq!(manifest.parts[0].row_count, 2);
    assert_eq!(manifest.parts[1].row_count, 1);
    assert_eq!(manifest.status, RunStatus::Completed);
}

#[test]
fn reader_requires_completed_manifest() {
    // Spec: "Readers require a completed manifest and valid hashes" +
    // "Test process interruption before/after part and manifest publication."
    // Interruption BEFORE manifest publication: parts exist, manifest does
    // not, and no reader accepts the run.
    let parent = temp_parent("interrupted-pre-manifest");
    let mut run = RunOutput::create(config(2), &parent).expect("create (Task 3.2)");
    run.write_batch(&batch(1..=2)).expect("part (Task 3.2)");
    drop(run); // simulated process interruption before completion

    let dir = run_dir(&parent);
    let read = read_completed_manifest(&dir);
    assert!(
        matches!(read, Err(RunOutputError::ManifestMissing { .. })),
        "interrupted run has no completed manifest, got: {read:?}"
    );
    let validated = validate_run_output(&dir);
    assert!(
        matches!(validated, Err(RunOutputError::ManifestMissing { .. })),
        "validation requires the completed manifest, got: {validated:?}"
    );
}

#[test]
fn reader_rejects_tampered_part_content() {
    // Spec: readers require "valid hashes"; hash re-attestation must catch a
    // modified published part.
    let parent = temp_parent("tampered-part");
    let mut run = RunOutput::create(config(2), &parent).expect("create (Task 3.2)");
    run.write_batch(&batch(1..=2)).expect("part (Task 3.2)");
    let manifest = run.complete().expect("completion (Task 3.2)");

    let dir = run_dir(&parent);
    let part_path = dir.join(&manifest.parts[0].file_name);
    let mut bytes = fs::read(&part_path).expect("part readable");
    let last = bytes.len() - 1;
    bytes[last] = bytes[last].wrapping_add(1);
    fs::write(&part_path, &bytes).expect("tamper applied");

    let validated = validate_run_output(&dir);
    assert!(
        matches!(validated, Err(RunOutputError::HashMismatch { .. })),
        "tampered part must fail hash re-attestation, got: {validated:?}"
    );
}

#[test]
fn reader_rejects_invalid_manifest_files() {
    // Spec: readers require a *completed* manifest. Malformed JSON, a wrong
    // schema identifier, and a status-less manifest are all rejected
    // explicitly (never interpreted as an incomplete-but-usable run).
    let parent = temp_parent("invalid-manifest");
    // Build a real (interrupted) run directory so the crafted manifests land
    // in an actual run layout; the test targets manifest parsing only.
    let mut run = RunOutput::create(config(2), &parent).expect("create");
    run.write_batch(&batch(1..=2)).expect("part");
    drop(run);
    let dir = run_dir(&parent);

    let cases: &[(&str, &str)] = &[
        ("empty", ""),
        ("wrong-schema", "{\"schema\":\"kairo_ecs.run_manifest.v0\"}"),
        ("status-less", "{\"schema\":\"kairo_ecs.run_manifest.v1\"}"),
    ];
    for (label, payload) in cases {
        fs::write(dir.join(RUN_MANIFEST_FILE_NAME), payload).expect("crafted manifest");
        let read = read_completed_manifest(&dir);
        assert!(
            matches!(read, Err(RunOutputError::ManifestInvalid(_))),
            "{label} manifest must be rejected, got: {read:?}"
        );
    }
}

#[test]
fn interruption_leaves_staging_files_that_never_validate() {
    // Interruption DURING a staged part write: staging leftovers may remain,
    // but the run still has no completed manifest and never validates.
    let parent = temp_parent("interrupted-staging");
    let mut run = RunOutput::create(config(2), &parent).expect("create (Task 3.2)");
    run.write_batch(&batch(1..=2)).expect("part (Task 3.2)");
    let dir = run_dir(&parent);
    fs::write(dir.join(".staging-part-00004"), b"partial").expect("leftover staged");
    drop(run); // interruption with a staging leftover on disk

    let validated = validate_run_output(&dir);
    assert!(
        matches!(validated, Err(RunOutputError::ManifestMissing { .. })),
        "staging leftovers never substitute for publication, got: {validated:?}"
    );
}

#[test]
fn checkpoint_boundary_persists_and_resume_neither_loses_nor_duplicates() {
    // Spec: "Persist checkpoint-to-trace partition boundary (replication ID,
    // part ID and sequence watermark) so resume neither loses nor duplicates
    // records."
    let parent = temp_parent("resume-watermark");
    let mut run_a = RunOutput::create(config(2), &parent).expect("create (Task 3.2)");
    run_a.write_batch(&batch(1..=4)).expect("part 1 (Task 3.2)");
    run_a.write_batch(&batch(5..=6)).expect("part 2 (Task 3.2)");

    let boundary = run_a
        .checkpoint_boundary()
        .expect("boundary after published parts (Task 3.2)");
    assert_eq!(
        boundary,
        CheckpointBoundary {
            replication_id: 1,
            part_id: 3,
            sequence_watermark: 6,
        }
    );
    drop(run_a); // interruption at the checkpoint boundary

    let mut run_b = RunOutput::resume(config(2), &parent, boundary)
        .expect("resume continues the interrupted run (Task 3.2)");
    run_b
        .write_batch(&batch(7..=8))
        .expect("resumed part (Task 3.2)");
    let manifest = run_b.complete().expect("completion (Task 3.2)");

    // The published sequence ranges partition 1..=8 with no gaps and no
    // duplicates across the interruption.
    let mut published: Vec<u64> = manifest
        .parts
        .iter()
        .flat_map(|part: &PartReceipt| part.sequence_first..=part.sequence_last)
        .collect();
    published.sort_unstable();
    let expected: Vec<u64> = (1..=8).collect();
    assert_eq!(published, expected, "resume neither loses nor duplicates");
}

#[test]
fn disk_exhaustion_fails_part_write_explicitly_without_silent_drop() {
    // Spec: "Test ... disk exhaustion", "No unbounded queue and no silent
    // trace drops", backpressure "changes wall-clock progress only".
    let parent = temp_parent("disk-full");
    let mut run = RunOutput::create(config(2), &parent).expect("create (Task 3.2)");

    let mut quota = QuotaWriter { remaining: 8 };
    let failed = run.write_batch_to(&batch(1..=4), &mut quota);
    assert!(
        matches!(failed, Err(RunOutputError::Io(_))),
        "disk exhaustion surfaces as an explicit I/O failure, got: {failed:?}"
    );
    assert!(
        run.parts().is_empty(),
        "a failed write records no part (no silent drop)"
    );

    // Backpressure is progress-only: after the condition clears, the same
    // rows publish successfully.
    let mut space = QuotaWriter {
        remaining: usize::MAX,
    };
    let receipts = run
        .write_batch_to(&batch(1..=4), &mut space)
        .expect("retry after exhaustion (Task 3.2)");
    assert_eq!(receipts.len(), 2, "batch split across bounded parts");
    let rows: usize = receipts.iter().map(|part| part.row_count).sum();
    assert_eq!(rows, 4, "every row published after retry");
}

#[test]
fn thread_budget_configures_once_with_single_concurrent_winner() {
    // Spec: "Configure native thread budget once before Polars pool
    // initialization; never mutate global environment concurrently." The
    // budget guard is process-global, so both budget properties are asserted
    // from this single order-robust test (see the file header for the merge
    // rationale): exactly one concurrent configuration wins, the loser sees
    // BudgetAlreadyConfigured, the winner's value is observable, and later
    // reconfiguration is rejected.
    let outcomes = thread::scope(|scope| {
        let handles: Vec<_> = (0..2)
            .map(|_| scope.spawn(|| configure_native_thread_budget(budget(4))))
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("budget thread joins"))
            .collect::<Vec<_>>()
    });

    let winners = outcomes.iter().filter(|result| result.is_ok()).count();
    assert_eq!(winners, 1, "exactly one concurrent configuration may win");
    for result in &outcomes {
        if let Err(error) = result {
            assert!(
                matches!(error, RunOutputError::BudgetAlreadyConfigured),
                "losing configuration attempts see the once-guard, got: {error:?}"
            );
        }
    }

    let configured = native_thread_budget().expect("budget read-back");
    assert_eq!(
        configured.map(|thread_budget| thread_budget.worker_threads.get()),
        Some(4),
        "the winning budget is observable"
    );

    let reconfiguration = configure_native_thread_budget(budget(2));
    assert!(
        matches!(
            reconfiguration,
            Err(RunOutputError::BudgetAlreadyConfigured)
        ),
        "reconfiguration must be rejected, got: {reconfiguration:?}"
    );
}

#[test]
fn high_cardinality_many_parts_stay_bounded() {
    // Spec: "Test multiple worker counts, skew, high cardinality" — many
    // high-cardinality batches stay within the part bound and every row is
    // published exactly once.
    let parent = temp_parent("high-cardinality");
    let mut run = RunOutput::create(config(3), &parent).expect("create (Task 3.2)");

    let total_parts = 40usize;
    for part_index in 0..total_parts {
        let first_sequence = (part_index * 3 + 1) as u64;
        run.write_batch(&batch(first_sequence..first_sequence + 3))
            .expect("part (Task 3.2)");
    }

    let manifest = run.complete().expect("completion (Task 3.2)");
    assert_eq!(manifest.parts.len(), total_parts);
    assert!(
        manifest.parts.iter().all(|part| part.row_count <= 3),
        "every part respects the bound"
    );
    let rows: usize = manifest.parts.iter().map(|part| part.row_count).sum();
    assert_eq!(rows, total_parts * 3, "no row dropped or duplicated");

    let dir = run_dir(&parent);
    let leftovers = fs::read_dir(&dir)
        .expect("run dir readable")
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_name().to_string_lossy().starts_with(".staging-"))
        .count();
    assert_eq!(leftovers, 0, "all staged files renamed into place");
}

#[test]
fn path_containment_rejects_escape_targets() {
    // Spec: "Validate path containment and collision handling."
    for bad_identity in ["escape/../run", "nested/run", ""] {
        let parent = temp_parent("containment");
        let mut bad = config(2);
        bad.run_id = bad_identity.to_string();
        let created = RunOutput::create(bad, &parent);
        assert!(
            matches!(created, Err(RunOutputError::InvalidArgument(_))),
            "identity {bad_identity:?} must be rejected by containment validation, got: {created:?}"
        );
        let _ = fs::remove_dir_all(&parent);
    }
}

#[test]
fn skewed_batches_keep_explicit_sequence_ordering() {
    // Spec: "Explicit deterministic row ordering" under skew: rows inside a
    // part follow the canonical (time ticks, priority, sequence) order even
    // when handed shuffled input, receipts carry inclusive min/max sequence
    // ranges, parts never overlap in sequence, and out-of-order capture is
    // rejected explicitly instead of silently reordered or dropped.
    let parent = temp_parent("skew-ordering");
    let mut run = RunOutput::create(config(10), &parent).expect("create (Task 3.2)");

    // Time inverted relative to sequence: canonical order must sort by time
    // first, so the published row order differs from arrival order.
    let inverted = vec![
        sample_record(1, 100, 0),
        sample_record(2, 50, 1),
        sample_record(3, 75, 2),
    ];
    let receipts = run.write_batch(&inverted).expect("first batch (Task 3.2)");
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].row_count, 3);
    assert_eq!(
        (receipts[0].sequence_first, receipts[0].sequence_last),
        (1, 3),
        "receipt range is inclusive min/max sequence of the part"
    );

    let part_path = run_dir(&parent).join(&receipts[0].file_name);
    let bytes = fs::read(&part_path).expect("part readable");
    let frame = read_parquet(Cursor::new(bytes)).expect("independent parquet read");
    let published = dataframe_to_records(&frame).expect("records from part");
    let published_sequences: Vec<u64> = published.iter().map(|record| record.sequence).collect();
    assert_eq!(
        published_sequences,
        vec![2, 3, 1],
        "rows canonically ordered by (time ticks, priority, sequence), not arrival order"
    );

    // Subsequent batches continue strictly after the watermark; their ranges
    // never overlap published parts.
    let next = run
        .write_batch(&batch(4..=6))
        .expect("next batch (Task 3.2)");
    assert_eq!((next[0].sequence_first, next[0].sequence_last), (4, 6));

    // Out-of-order capture is rejected explicitly (no silent reorder/drop).
    let stale = run.write_batch(&batch([3, 7]));
    assert!(
        matches!(stale, Err(RunOutputError::InvalidArgument(_))),
        "sequence 3 is at or below the watermark 6 and must be rejected, got: {stale:?}"
    );

    let ranges: Vec<(u64, u64)> = run
        .parts()
        .iter()
        .map(|part| (part.sequence_first, part.sequence_last))
        .collect();
    for pair in ranges.windows(2) {
        assert!(
            pair[0].1 < pair[1].0,
            "part sequence ranges are strictly increasing and non-overlapping: {ranges:?}"
        );
    }
}

#[test]
fn empty_run_publishes_zero_part_manifest() {
    // A run with no published parts still completes: the manifest exists,
    // carries zero parts, and validates — the empty case stays explicit.
    let parent = temp_parent("empty-run");
    let run = RunOutput::create(config(2), &parent).expect("create (Task 3.2)");
    let manifest: RunManifest = run.complete().expect("completion (Task 3.2)");
    assert!(manifest.parts.is_empty());

    let dir = run_dir(&parent);
    let validated = validate_run_output(&dir);
    assert!(
        validated.is_ok(),
        "empty completed run validates, got: {validated:?}"
    );
}

#[cfg(unix)]
#[test]
fn protected_root_is_enforced_for_create_and_resume() {
    use std::os::unix::fs::PermissionsExt;
    let parent = tempfile::tempdir().unwrap();
    let mut run = RunOutput::create(config(2), parent.path()).unwrap();
    run.write_batch(&batch(1..=2)).unwrap();
    let boundary = run.checkpoint_boundary().unwrap();
    drop(run);
    let existing = fs::read_dir(parent.path()).unwrap().count();
    fs::set_permissions(parent.path(), fs::Permissions::from_mode(0o777)).unwrap();
    assert!(RunOutput::create(config(2), parent.path()).is_err());
    assert!(RunOutput::resume(config(2), parent.path(), boundary).is_err());
    assert_eq!(fs::read_dir(parent.path()).unwrap().count(), existing);
}

#[test]
fn existing_part_is_never_replaced() {
    let parent = tempfile::tempdir().unwrap();
    let mut run = RunOutput::create(config(2), parent.path()).unwrap();
    let dir = run_dir(parent.path());
    let part = dir.join("part-00001.parquet");
    fs::write(&part, b"preexisting").unwrap();
    assert!(run.write_batch(&batch(1..=2)).is_err());
    assert_eq!(fs::read(part).unwrap(), b"preexisting");
    assert!(run.parts().is_empty());
    assert!(!dir.join(RUN_MANIFEST_FILE_NAME).exists());
}

#[cfg(unix)]
#[test]
fn replacement_run_path_is_rejected_before_writing() {
    let parent = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let mut run = RunOutput::create(config(2), parent.path()).unwrap();
    let dir = run_dir(parent.path());
    let moved = parent.path().join("moved");
    fs::rename(&dir, &moved).unwrap();
    std::os::unix::fs::symlink(outside.path(), &dir).unwrap();
    assert!(run.write_batch(&batch(1..=2)).is_err());
    assert!(run.complete().is_err());
    assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
    assert_eq!(fs::read_dir(moved).unwrap().count(), 0);
}

#[cfg(unix)]
#[test]
fn reader_rejects_symlinked_part_even_with_matching_bytes() {
    let parent = tempfile::tempdir().unwrap();
    let mut run = RunOutput::create(config(2), parent.path()).unwrap();
    run.write_batch(&batch(1..=2)).unwrap();
    let manifest = run.complete().unwrap();
    let dir = run_dir(parent.path());
    let part = dir.join(&manifest.parts[0].file_name);
    let moved = parent.path().join("original-part");
    fs::rename(&part, &moved).unwrap();
    std::os::unix::fs::symlink(&moved, &part).unwrap();
    assert!(validate_run_output(&dir).is_err());
}

#[test]
fn resume_ignores_abandoned_staging_names_without_overwriting_them() {
    let parent = tempfile::tempdir().unwrap();
    let mut run = RunOutput::create(config(2), parent.path()).unwrap();
    run.write_batch(&batch(1..=2)).unwrap();
    let boundary = run.checkpoint_boundary().unwrap();
    let dir = run_dir(parent.path());
    let abandoned_part = dir.join(".staging-part-00002.parquet");
    let abandoned_manifest = dir.join(".staging-run_manifest.json");
    fs::write(&abandoned_part, b"partial part").unwrap();
    fs::write(&abandoned_manifest, b"partial manifest").unwrap();
    drop(run);
    let mut resumed = RunOutput::resume(config(2), parent.path(), boundary).unwrap();
    resumed.write_batch(&batch(3..=4)).unwrap();
    resumed.complete().unwrap();
    let manifest = validate_run_output(&dir).unwrap();
    assert_eq!(
        manifest
            .parts
            .iter()
            .map(|part| part.row_count)
            .sum::<usize>(),
        4
    );
    assert_eq!(fs::read(abandoned_part).unwrap(), b"partial part");
    assert_eq!(fs::read(abandoned_manifest).unwrap(), b"partial manifest");
}
