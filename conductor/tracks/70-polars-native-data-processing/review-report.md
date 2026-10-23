# Review Report: Track 70 Polars-native data processing and standard interchange

## Phase 2 Closeout Review (23 October 2026)

### Summary
Phase 2 (FFI and polyglot consumers) is complete across Tasks 2.1–2.3 after an iterative per-file review of the full phase diff (`d02a010..00f523c`, 23 code/doc files, +2,032/−326) as Principal Software Engineer. Quality, safety, and contract-compliance hold: the Arrow crate stays `unsafe`-free with explicit schema-negotiation rejection; every `unsafe` in the FFI bridge carries a documented SAFETY rationale; the C header and its generated mirror stay byte-identical; empty/null/multi-batch/release-once semantics are tested dual-mode; and the v2 Schema is projected consistently across Python, TypeScript, R, and Julia with per-binding actual-copy and unsupported-target documentation.

### Verification Checks
- [x] **Plan Compliance**: Tasks 2.1 (failing fixtures), 2.2 (populated export + native Python handoff, gate override recorded), and 2.3 (C stream ABI + reachable binding v2 migration) executed without false completions. Task 2.3's scope expansion to R was user-approved after execution discovered the R event-log surface the initial scope check missed.
- [x] **Correctness & Safety**: `arrow_c_data.rs` contains no `unsafe` (crate-level `unsafe_code` forbid); transmutes in `crates/kairo-ecs-ffi` are between layout-identical `#[repr(C)]` mirrors with compile-checked 320-bit sizes and documented release-once semantics; null-pointer and `max_batch_rows` validation leave out-params untouched on failure.
- [x] **Style Compliance**: Pass — rust/ts/r/julia style guides honored (no `unsafe` outside bridge crates, documented Wasm↔JS copies, decimal-string precision-safe projections).
- [x] **Contract**: v2 identifiers (`kairo_ecs.event_log.v2`), 14 typed columns, null-together generational pairs, full u128 tick limbs, explicit rejection of malformed negotiation — verified in FFI, arrow, and per-binding tests.
- [x] **Test Results**: Passed — arrow 16+9, FFI 5+14+9 (native-data) / 5+14+8 (default), Python 26, TS 10 (typecheck clean), Julia 7/7 testsets, R 6 conformance + 27 smoke (+1 documented optional-arrow skip), WASM crate check clean and Polars-free.
- [x] **Conductor validators**: `validate_conductor_phase_gates.ps1` and `validate_conductor_dag.ps1` → 0 errors, 0 warnings.

### Findings
- **Medium (fixed in closeout)**: R facade silently rounded entity IDs above the 2^53 double limit instead of rejecting explicitly. Fix: exact-integer guard (`kairoecs_exact_integer`) on entity IDs and ticks with contract-accurate errors; tests added for the 2^53 boundary (exact) and out-of-range rejection (ticks and entity).
- **Finding introduced and caught during the fix**: the first guard rewrite dropped the nullable-entity early return; re-running the targeted lanes caught it (conformance fixtures failed) before closeout — receipts only count the final green runs.
- **Low (fixed in closeout)**: R `order_event_log` compared limbs only numerically, externally supplied rows above 2^53 could collapse in `as.numeric`. Fix: lexicographic decimal-string tie-breaks after the numeric comparison.
- **Recorded, not applied (cross-owner)**: `arrow_c_data_interface_supported = TRUE` in the R/Julia native-library loader stubs sits next to `not configured` FFI status — ambiguous capability flag; recorded as Track 63/07/08 follow-up in handoff.md (Wave 9 surface, outside the Phase 2 diff).
- **Recorded, not applied (cross-owner)**: "native data CLIs enable native-data by default" spec clause deferred to Track 66 reconciliation; `kairo-ecs-cli` has no Arrow usage and no native-data CLI consumer exists.

### Phase Disposition
Phase 2 is **CLOSED**. Next-phase decision: advance to Phase 3 (durable output and execution budgets: Tasks 3.1–3.x) pending maintainer merge of the stacked PRs #290 → #291 → #292, plus the closeout branch PR.

---

## Phase 1 Closeout Review (23 October 2026)

### Summary
Phase 1 (Native tables and real formats) has completed all tasks (1.1–1.4). Verified that `polars_interop` module provides exact Schema v2 DataFrame creation and reconstruction, full 128-bit tick preservation with checked limb borrow subtraction, real Parquet Zstd and Arrow IPC writers/readers, and lazy tabular transforms. All 4 Codex review items are resolved and validated.

### Verification Checks
- [x] **Plan Compliance**: Tasks 1.1, 1.2, 1.3, and 1.4 completed without false completions. Phase 1 is 100% complete. Phases 2, 3, and 4 remain accurately open.
- [x] **Table & Schema v2 Integrity**: `records_to_dataframe` and `dataframe_to_records` verified with strict column type checks and nullability tests in `track70_phase1_native_tables.rs`.
- [x] **Full-Range Tick Arithmetic**: Full 128-bit ticks verified across limbs (`u128::MAX`, `1<<64`). Checked `subtract_tick_limbs` borrow arithmetic and underflow rejection verified.
- [x] **Real Format Interoperability**: Real Parquet Zstandard compression verified via Thrift footer inspection (magic `PAR1`, length, field declarations) and `read_parquet` roundtrip. Real Arrow IPC stream roundtrip verified.
- [x] **Lazy Tabular Transforms**: `filter_by_event_kind`, `filter_by_entity`, and `aggregate_counts_by_status` verified via Polars expressions.
- [x] **Codex Review Resolution**: All 4 Codex review comments resolved and validated.
- [x] **Code Health & Quality**:
  - `cargo fmt --all --check`: clean
  - `cargo clippy -p kairo-ecs-arrow --all-targets --all-features -- -D warnings`: clean (0 warnings)
  - `cargo test -p kairo-ecs-arrow --features polars-native`: 25 tests passed (0 failures)
  - `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`: 0 errors, 0 warnings
  - `pwsh -NoProfile -File scripts/validate_conductor_dag.ps1`: 0 errors, 0 warnings

### Phase Disposition
Phase 1 is **CLOSED**. Next-phase decision: advance to Phase 2 (FFI and polyglot consumers: Tasks 2.1–2.4).

---

## Phase 0 Closeout Review (23 October 2026)

### Summary
Phase 0 (Baseline and dependency qualification) has completed all tasks (0.1–0.5). Verified that all dependency locks, schema definitions, baseline behavioral tests, conformance fixtures, and architectural decision records are merged and validated on `origin/main`.

### Verification Checks
- [x] **Plan Compliance**: Tasks 0.1–0.4 delivered to `main` across PRs #285, #286, and #287 without false completions. Phase 0 tasks 100% complete. Phases 1–4 remain accurately open.
- [x] **Contract & Schema Verification**: Schema v2 contract verified with limb-subtraction and field tests in `crates/kairo-ecs-arrow/tests/schema_v2_contract.rs`. Conformance fixture registered in `conformance/fixtures/manifest.json` and passed by `node tests/conformance/conformance-check.mjs`.
- [x] **Dependency & Feature Isolation**: `polars 0.45` qualified behind optional `polars-native` feature in `crates/kairo-ecs-arrow/Cargo.toml`. Verified that minimal-core and Wasm builds remain free of Polars via `crates/kairo-ecs-arrow/tests/polars_isolation_proof.rs`.
- [x] **Code Health & Quality**:
  - `cargo fmt --all --check`: clean
  - `cargo clippy -p kairo-ecs-arrow --all-targets --all-features -- -D warnings`: clean (0 warnings)
  - `cargo test -p kairo-ecs-arrow --all-targets`: 19 tests passed (including 3 failing baseline characterizations)
  - `node tests/conformance/conformance-check.mjs`: all 7 fixtures passed
  - `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`: 0 errors, 0 warnings
  - `pwsh -NoProfile -File scripts/validate_conductor_dag.ps1`: 0 errors, 0 warnings

### Phase Disposition
Phase 0 is **CLOSED**. Next-phase decision: advance to Phase 1 (Native tables and real formats: Tasks 1.1–1.4).

---

## Initial Planning Review (22 October 2026)

### Summary
The planning package reflects the user-approved clean break and library-replacement objective; runtime implementation remains pending.

### Verification Checks
- [x] Plan Compliance: both repositories, upstream-first gate, Rust-side processing, Parquet default, bounded floating-point comparisons.
- [x] Style Compliance: experimental maturity and explicit evidence boundaries.
- [x] New Tests: runtime test requirements specified; no runtime tests claimed implemented.
- [x] Test Coverage: planning registry/DAG/context checks only; runtime coverage not assessed.
- [x] Test Results: planning validators passed as recorded below.
