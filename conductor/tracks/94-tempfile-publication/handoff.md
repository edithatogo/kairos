# Handoff

Updated: 2026-10-24. Maturity: complete; all phases (Phases 0–2) accepted.

## Summary

Track 94 completed: qualified `tempfile` (v3.27.0 / v3.17.1+), eliminated direct non-atomic file writes and static global temp directory paths in `crates/kairo-ecs-arrow`, implemented `atomic_write_file` with same-filesystem staging and persist/no-clobber controls, upgraded `DurableRunPublicationWriter` and `ParquetZstdCheckpointWriter` to atomic publication, and established hermetic test directory isolation across Rust and Python suites.

## Files changed

- `crates/kairo-ecs-arrow/Cargo.toml`
- `crates/kairo-ecs-arrow/src/lib.rs`
- `crates/kairo-ecs-arrow/src/polars_interop.rs`
- `crates/kairo-ecs-arrow/tests/track70_phase3_durable_publication.rs`
- `crates/kairo-ecs-arrow/tests/checkpoint_restart.rs`
- `crates/kairo-ecs-arrow/tests/track94_phase0_tempfile_baseline.rs`
- `crates/kairo-ecs-arrow/tests/track94_phase1_tempfile_verification.rs`
- `bindings/python/tests/test_track94_tempfile_polyglot.py`
- `website/src/content/docs/architecture.md`
- `conductor/tracks/94-tempfile-publication/plan.md`
- `conductor/tracks/94-tempfile-publication/metadata.json`
- `conductor/tracks/94-tempfile-publication/review-report.md`
- `conductor/tracks/94-tempfile-publication/evidence/phase0-qualification.md`
- `conductor/tracks/94-tempfile-publication/evidence/phase2-closeout-report.md`

## Contracts consumed

[Value policy](../../dependency-value-policy.md), existing owner contracts and prerequisites in spec.md.

## Contracts changed

- `kairo-ecs-arrow`: exports `atomic_write_file(destination, data, allow_overwrite) -> Result<(), ArrowError>`.
- `ParquetZstdCheckpointWriter`: adds `write_checkpoint_file(path, schema, batches, allow_overwrite) -> Result<(), ArrowError>`.
- `DurableRunPublicationWriter`: stages parts and manifest atomically beside destination, with `with_no_clobber()` and `with_overwrite()` methods.
- Test suites: converted to hermetic `tempfile::tempdir()` isolation.
- Ephemeral staging paths remain internal; no filesystem randomness enters simulation seeds, hashes, or manifest/parquet output identities.

## Tests added

- `crates/kairo-ecs-arrow/tests/track94_phase0_tempfile_baseline.rs`:
  - `baseline_direct_write_leaves_partial_file_on_interruption`
  - `atomic_staging_primitive_must_exist`
- `crates/kairo-ecs-arrow/tests/track94_phase1_tempfile_verification.rs`:
  - `test_concurrent_directory_creation_and_hermetic_isolation`
  - `test_atomic_publication_staging_and_cleanup_on_failure`
  - `test_no_clobber_protection_preserves_existing_output`
  - `test_checkpoint_writer_atomic_write_file_with_sha256`
  - `polars_publication_tests::test_durable_publication_writer_no_clobber_and_deterministic_identity`
- `bindings/python/tests/test_track94_tempfile_polyglot.py`:
  - `test_isolated_tempdir_publication_lifecycle`
  - `test_tempfile_isolation_preserves_deterministic_content`

## Known risks

Resolved: Cross-device link errors (`EXDEV`) avoided by staging strictly within the destination directory parent (`NamedTempFile::new_in(parent)`).

## Follow-up issues

CareOps publication sinks and CLI staging helpers can adopt `atomic_write_file` and `tempfile::TempDir`.

## Integration notes

See [programme](../../robust-dependencies.md). Worktree integration, local main, hosted CI and runtime acceptance remain distinct.

## Phase closeout evidence

Track 94 complete:
- Review command: `$conductor-review`
- Accepted fixes: auto-applied in-scope phase 0, 1, and 2 code, verification tests, and documentation.
- Validation:
  - `cargo test -p kairo-ecs-arrow --all-targets --features "polars-native parallel-io"` passed (36 tests).
  - `cargo clippy -p kairo-ecs-arrow --features "polars-native parallel-io" -- -D warnings` passed (0 warnings).
  - `PYTHONPATH=bindings/python .venv/bin/pytest bindings/python/tests/test_track94_tempfile_polyglot.py -q` passed (2 tests).
  - `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` passed.
  - `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_dag.ps1` passed.
- Strict git closeout gate: run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.
- Clean commit SHA and pushed ref recorded at each slice advancement.
- Next-phase decision: Track 94 is complete and ready for archival.
