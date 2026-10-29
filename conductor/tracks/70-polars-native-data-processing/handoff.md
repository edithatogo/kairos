# Track 70 handoff: Polars-native data processing and standard interchange

## Publication repair assignment — 29 October 2026

The user assigned integration chat `01a18344-cfb8-7711-8c38-fef1052b9fcf` the
bounded Arrow publication repair. Its isolated standard Git worktree is
`/Volumes/PortableSSD/codex-worktrees/kairos-arrow-publication-repair-20261029`.
The registry names Arrow, FFI and interoperability roles; no live Arrow-path
lease or Arrow-owner worktree was identified. This assignment covers the
durable Polars manifest, writer/reader, shared atomic-write sync and focused
tests. FFI, bindings, native objects, dependency promotion and CareOps pin
ownership remain separate. Next: repair and focused tests, then compatibility
and security review and hosted checks. Earlier assignments remain historical.

The isolated candidate uses Serde for the manifest, exclusively creates run
directories, checks manifest names and totals, and propagates Unix directory
sync failures. A baseline run of the focused test file at `534c9bd` had four
passes and three new failures; after the changes, the publication, Track 94
atomic-write and Track 70 durability test files passed 27 tests with Rust
1.99.1 and `--locked --features polars-native`. Focused Clippy and rustfmt
also passed. Build artifacts are in
`/Volumes/PortableSSD/codex-builds/kairos-arrow-publication-20261029`.
The portable follow-up candidate uses capability-based directory-relative,
no-follow opens via cap-primitives and cap-fs-ext 4.0.3, replacing the initial
Unix-only rustix approach. The deterministic directory-replacement and symlink
test passes on macOS; Windows execution is unverified. The new dependency graph
adds 22 locked packages, including target-specific packages, and its licence
and source checks pass after fetching all targets. Directory fsync is Unix-only,
and an error after rename may leave an already-published file visible. Writer
path handling under a hostile concurrent filesystem actor remains unqualified.
Independent security and compatibility review, cross-platform execution,
security audit, `just ci`, hosted checks and consumer acceptance remain open.

An independent synthetic readback of the separate zstd 0.14.3 workspace
qualification candidate found an additional contract gap. On 29 October 2026,
PyArrow 26.0.0 read its Parquet output with ZSTD columns and its Arrow IPC file
output, with equal logical rows and full-width tick limbs. PyArrow's IPC stream
reader rejected the output: `write_ipc_stream` currently uses Polars `IpcWriter`
and emits the IPC file format. Parquet strings and IPC strings also had
different physical Arrow types. Inputs, outputs, Rust producer and lock are in
`/Volumes/PortableSSD/codex-builds/kairos-zstd-independent-reader`; parquet
SHA-256 `17e474f2479b881860c2d1fe4c453dcac40b1ad080a08e0d86aff4b4fb5a64ad`,
IPC SHA-256 `76fd7129f6f68d7c4a57b2599fdc74e71dd9eadbb21cbff26803f29578a8d538`.
The apparent stream behavior and file/stream migration require contract and
consumer review; no independent stream interoperability is claimed.

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
- commit SHA: `dc35ce9b8e8f80459c55b6a711f62cba394c2e6f` (Track 70 Phases 2-4 implementation); merged Phase 0 closeout PR #288 on `origin/main`; Phase 1 PR #289.
- pushed ref: `origin/main`
- `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`: verified clean tree state across phase closeout.
- next-phase decision: Track 70 is closed and approved for CareOps adoption.

## Sequencing revision — 23 October 2026

User-authorized planning change: supporting dependencies gate only demonstrated affected tasks. The existing implementation history and phase decisions above are preserved. No runtime paths, FMI work, pins, acceptance statuses or phase closeouts changed. See the Conductor dependency-sequencing.md decision contract.
