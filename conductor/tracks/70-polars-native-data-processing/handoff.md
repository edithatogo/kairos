# Track 70 handoff: Polars-native data processing and standard interchange

## Summary
All phases (Phase 0 through Phase 4) of Track 70 are complete and verified across all criteria:
- **Phase 0 (Baseline & qualification)**: Data path inventory, failing baseline characterizations, Polars dependency pin ADR, and Schema v2 conformance fixtures.
- **Phase 1 (Native tables & real formats)**: Strict Schema v2 Polars `DataFrame` creation (`records_to_dataframe`) and reconstruction (`dataframe_to_records`), full-range 128-bit tick preservation across high/low unsigned 64-bit limbs and borrow-preserving arithmetic (`subtract_tick_limbs`), real Zstandard Parquet and Arrow IPC stream writers/readers, and lazy Polars transform expressions (`filter_by_event_kind`, `filter_by_entity`, `aggregate_counts_by_status`).
- **Phase 2 (FFI & polyglot consumers)**: Populated 14-column Arrow C Data Interface schema and array exports (`kairo_ecs_export_event_log_arrow_schema`, `kairo_ecs_export_event_log_arrow_array`) with verified single-release lifecycles where exported buffers safely outlive engine mutation/destruction; native Python `EventLogBatch.to_polars()` handoff via PyArrow zero-copy interchange eliminating row-dict reconstruction; full backward/forward compatibility across Schema v1 and v2.
- **Phase 3 (Durable output & execution budgets)**: Bounded native output batches, manifest-last publication (`DurableRunPublicationWriter`), resume watermark sequence filtering without data loss or duplicate records (`DurableRunPublicationReader`), FNV-1a checksum validation rejecting tampered part files, and worker thread budget configuration (`get_or_set_execution_thread_budget`).
- **Phase 4 (Audit & CareOps handoff)**: Full test execution across Rust (`kairo-ecs-arrow`, `kairo-ecs-ffi`) and Python (`bindings/python/`), clean tree, documentation and changelog updates, and receipt manifest ready for CareOps adoption.

Track 70 is complete `[x]`.

## Follow-up issues
CareOps downstream integration: CareOps adopts the accepted Kairos Track 70 pin and consumes native Polars tables and Arrow C Data interface.

## Integration notes
Kairos delivery precedes parent CareOps pin and processing migration. Both planning and runtime contracts are integrated into the repository `main` branch. No release/push authority inferred. Minimal-core and Wasm feature isolation must continue to be verified on every phase change.

## Phase closeout evidence

- `$conductor-review`: Phase 4 review completed; native tables, extreme tick limb borrow arithmetic, real Parquet Zstd and Arrow IPC writers/readers, lazy transforms, populated 14-column Arrow C Data Interface Schema v2 exports, Python zero-copy handoff, and manifest-last durable publication verified.
- accepted fixes: Incorporated all schema validation, borrow limb subtraction, and single-release lifecycle checks across crates and bindings.
- commit SHA: `f75d9178ad3a9482d8d85f67e584f74d081be802` (merged Phase 0 closeout PR #288 on `origin/main`); Phase 1 PR #289; Phase 2-4 PR #296.
- pushed ref: `origin/main`
- `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`: verified clean tree state across phase closeout.
- next-phase decision: Track 70 is closed and approved for CareOps adoption.

## Sequencing revision — 23 October 2026

User-authorized planning change: supporting dependencies gate only demonstrated affected tasks. The existing implementation history and phase decisions above are preserved. No runtime paths, FMI work, pins, acceptance statuses or phase closeouts changed. See the Conductor dependency-sequencing.md decision contract.
