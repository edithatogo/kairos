# Track 70 handoff: Polars-native data processing and standard interchange

## Summary
Phase 0 (Baseline and dependency qualification) and Phase 1 (Native tables and real formats) are complete and verified across all criteria. Tasks 1.1 through 1.4 have delivered:
- Strict Schema v2 Polars `DataFrame` creation (`records_to_dataframe`) and reconstruction (`dataframe_to_records`).
- Full-range 128-bit tick preservation across high/low unsigned 64-bit limbs and checked borrow-preserving arithmetic (`subtract_tick_limbs`).
- Real Zstandard-compressed Parquet and Arrow IPC stream writers and readers.
- Lazy Polars tabular transform expressions (`filter_by_event_kind`, `filter_by_entity`, `aggregate_counts_by_status`).
- Comprehensive resolution of all 4 Codex review items (independent reader validation, extreme tick limb assertion, Schema v2 DataFrame verification, and exact offending record version reporting).

Track 70 remains `[~] In Progress` advancing to Phase 2 (FFI and polyglot consumers: Tasks 2.1–2.4).

## Phase 2 progress — Task 2.1 (23 October 2026)

- Task 2.1 complete: failing populated multi-batch/empty/null/lifetime/release tests and affected binding consumer fixtures recorded, per the Phase 0/1 failing-characterization convention (`should_panic` probes / strict `xfail` that must be replaced with real acceptance tests by Task 2.2).
- New Rust test target `crates/kairo-ecs-ffi/tests/track70_phase2_ffi_consumers.rs` (6 tests): populated-buffer contract, v2 schema identifiers/typed children, multi-batch stream export, schema-valid empty export, null-together generational IDs, plus a real passing release-exactly-once and struct-survives-engine-free guard.
- New Python consumer fixture `bindings/python/tests/test_track70_phase2_native_handoff.py` (4 strict-xfail tests): v2 schema fields via Arrow C capsule import, null-together entity limbs, schema-valid empty export, explicit rejection instead of fabricated capsule fallback.
- Baseline evidence documented: `kairo_ecs_export_event_log_arrow_array` is descriptor-only (`buffers == NULL`, `n_children == 0`); `kairo_ecs_export_event_log_arrow_schema` exposes `kairo_ecs.event_log.v1` with 0 children; Python `_arrow.py` retains the fabricated capsule fallback and the `to_pylist` row-reconstruction detour.
- Scope decision recorded by session coordination: Tasks 2.2–2.3 runtime implementation is gated on Tracks 93 (sha2), 94 (tempfile) and 96 (proptest) closing per the 22 October 2026 dependency sequencing refinement; Task 2.1 failing tests/fixtures do not migrate runtime behavior and were validated under `cargo test -p kairo-ecs-ffi --test track70_phase2_ffi_consumers` (6 passed, incl. 5 should-panic probes) and `python -m pytest bindings/python/tests` (21 passed, 4 xfailed, Python 3.14.8 disposable uv venv, pyarrow 26.0.0). Expired leases observed on Tracks 65/66 (disjoint paths); reconciled by owner, not adopted.
- Cross-owner note (handoff, not unilateral write): Task 2.2 touches `crates/kairo-ecs-ffi/`, `include/`, `bindings/` — FFI/binding owners review their surfaces; Track 66 CLI claim must be reconciled before any CLI changes.

## Follow-up issues
Phase 2 runtime implementation: implement owned Arrow C Data arrays/streams and native Python-to-Polars zero-copy handoff; update C headers and polyglot bindings; document actual copies and memory boundaries.

## Integration notes
Kairos delivery precedes parent CareOps pin and processing migration. Both planning and runtime contracts are integrated into the repository `main` branch. No release/push authority inferred. Minimal-core and Wasm feature isolation must continue to be verified on every phase change.

## Phase closeout evidence

- `$conductor-review`: Phase 1 review completed; native tables, extreme tick limb borrow arithmetic, real Parquet Zstd and Arrow IPC writers/readers, and lazy transforms verified.
- accepted fixes: All 4 review suggestions from Codex connector incorporated into `crates/kairo-ecs-arrow/src/polars_interop.rs` and `track70_phase1_native_tables.rs`.
- commit SHA: `f75d9178ad3a9482d8d85f67e584f74d081be802` (merged Phase 0 closeout PR #288 on `origin/main`); Phase 1 PR #289.
- pushed ref: `origin/main`
- `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`: verified clean tree state across phase closeout.
- next-phase decision: Phase 1 is closed and approved to advance to Phase 2 (FFI and polyglot consumers: Tasks 2.1–2.4).

## Sequencing revision — 23 October 2026

User-authorized planning change: supporting dependencies gate only demonstrated affected tasks. The existing implementation history and phase decisions above are preserved. No runtime paths, FMI work, pins, acceptance statuses or phase closeouts changed. See the Conductor dependency-sequencing.md decision contract.
