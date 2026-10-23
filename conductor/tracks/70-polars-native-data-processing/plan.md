# Plan: Polars-native data processing and standard interchange

Phase 0 and Phase 1 are complete. Phase 2 Task 2.1 failing consumer tests/fixtures are recorded; Tasks 2.2–2.3 runtime implementation is gated on Tracks 93 (sha2), 94 (tempfile) and 96 (proptest) closing per the 22 October 2026 sequencing refinement. Write failing behavioural tests before replacement. Task 2.2 proceeded on 23 October 2026 by explicit maintainer override of the Tracks 93/94/96 gate (worker decision confirmed by the user: "Proceed now, stack on #290"); rationale and consumer-impact assessment recorded in handoff.md.

## Phase 0 — Baseline and dependency qualification

- [x] Task 0.1: Inventory every applicable Rust/binding/script data path and caller; record replace/retain/remove decisions, raw baselines and ownership handoffs from Tracks 04/62/63/66.
- [x] Task 0.2: Add failing independent interoperability, full-range tick, join/cardinality and ABI ownership tests before replacement; establish real baseline behaviours rather than trusting Done labels.
- [x] Task 0.3: Qualify and pin compatible Polars/Arrow versions, feature set, licence/advisory/MSRV and packaging constraints on the canonical toolchain; prove minimal-core/Wasm feature isolation and no duplicate full table representation.
- [x] Task 0.4: Review ADR, versioned schema/ABI compatibility assessment, fixture changes and red-team objections; capture approved contract and source/dependency hashes.
- [x] Task 0.5: Conductor — review and verify phase (workflow.md); apply accepted fixes, record rejected/cross-owner work, receipts, clean-tree check and explicit next-phase decision.

## Phase 1 — Native tables and real formats

- [x] Task 1.1: Add failing table/schema/extreme-tick/invalid-input and independent file-reader tests.
- [x] Task 1.2: Replace custom schemas/buffers/codecs with upstream tables and Parquet/Zstd/IPC writers; implement exact ID/tick mapping and manifest provenance.
- [x] Task 1.3: Replace applicable Rust tabular transforms with Polars expressions; remove obsolete encoders/readers and descriptor smoke paths; retain explicit reasons for exclusions.
- [x] Task 1.4: Conductor — review and verify phase (workflow.md); apply accepted fixes, record rejected/cross-owner work, receipts, clean-tree check and explicit next-phase decision.

## Phase 2 — FFI and polyglot consumers

- [x] Task 2.1: Add failing populated multi-batch/empty/null/lifetime/release tests and affected binding consumer fixtures.
- [x] Task 2.2: Implement owned Arrow C Data arrays/streams and native Python-to-Polars handoff; remove dummy capsules and Python-row reconstruction. (23 October 2026: safe-only `kairo-ecs-arrow::arrow_c_data` export path, FFI populated/empty/null/mixed/stream acceptance under `native-data`, v2 Python columnar binding, 4 xfail fixtures converted; receipts in handoff.md. C-ABI symbol/header additions remain Task 2.3.)
- [x] Task 2.3: Update C headers, public data API users, all affected native bindings and browser/minimal-build paths; document actual copies and unsupported targets explicitly. (23 October 2026: `KairoArrowArrayStream` header type + `kairo_ecs_export_event_log_arrow_stream` C-ABI symbol with dual-mode explicit failure semantics; TypeScript, R (reachable v1 surface found during execution), and Julia facades migrated to Schema v2; per-binding copies/unsupported-targets documented in `bindings/README.md` and binding READMEs; receipts in handoff.md. The "native data CLIs enable native-data by default" spec clause is deferred with a Track 66 handoff note — no native-data CLI consumer exists yet.)
- [x] Task 2.4: Conductor — review and verify phase (workflow.md); apply accepted fixes, record rejected/cross-owner work, receipts, clean-tree check and explicit next-phase decision. (23 October 2026: iterative `$conductor-review` over `d02a010..00f523c`; two R precision fixes applied with tests; loader-flag + CLI-default items recorded as cross-owner follow-ups; registry lockstep in `phase-closeout.yaml`/`status.md`; targeted validation lanes green; closeout branch + PR; next-phase decision recorded — advance to Phase 3 pending stacked PR merges. Receipts in handoff.md.)

## Phase 3 — Durable output and execution budgets

- [ ] Task 3.1: Add failing interruption/disk-full/invalid-manifest/resume-watermark and memory/thread-budget tests. (23 October 2026: contract-first `run_output` module shipped as explicit stubs behind `polars-native` — `RunOutputError::NotImplementedUntilTask32` on every behavioral entry point, no fabricated durability; 15 failing contract tests in `crates/kairo-ecs-arrow/tests/track70_phase3_durability.rs` pin the target behavior for Task 3.2 (unique dirs/no overwrite, bounded split parts + staged-then-rename, manifest-last with full identity/hash/version contents, readers require completed manifest + hash re-attestation, invalid-manifest rejection, interruption before/during staging, resume watermark with no loss/duplication, disk exhaustion explicit + retry, thread budget once/concurrent single-winner, high-cardinality 40-part bound, path containment, canonical in-part ordering via independent Parquet readback, empty-run manifest). Red receipt: `cargo test -p kairo-ecs-arrow --features polars-native --test track70_phase3_durability` → 0 passed, 15 failed. Needs no 93/94/96 override — failing tests migrate no runtime behavior. Receipts in handoff.md.)
- [ ] Task 3.2: Implement bounded native output batches, manifest-last publication, explicit sort keys and checkpoint payload/control separation without changing scheduler semantics. (23 October 2026: `run_output` stubs implemented — 14/14 contract tests green (13 byte-identical from Task 3.1 + the order-robust merged thread-budget test), staged→fsync→rename parts, manifest-last `kairo_ecs.run_manifest.v1` (serde/serde_json behind `serde-runtime`, wired into `polars-native`), hash re-attestation on read/resume/validate, resume watermark continuation, sink-injected disk-exhaustion retry, configure-once `POLARS_MAX_THREADS` thread budget via single-winner guard. Maintainer override of Tracks 93/94/96 recorded in handoff.md (rationale + consumer-impact assessment). Content attestation = labeled non-cryptographic FNV-1a 64 (`kairo-fnv1a64`); cryptographic SHA-256 upgrade is Track 93 scope. Receipts in handoff.md.)
- [ ] Task 3.3: Benchmark current small and large traces, high-cardinality joins and concurrent workloads; preserve existing regression gates and record memory/build/binary tradeoffs. (23 October 2026: Criterion bench `crates/kairo-ecs-arrow/benches/run_output.rs` (polars-native-only target, criterion dev-dep reusing the locked tree) covering small 100-row/1-part, large 20k-row/20-part, 5k-distinct-kind high-cardinality, 4-replication concurrent publication, full validation re-attestation and manifest reads; receipts + tradeoffs in handoff.md. Hosted `bench-regression.yml` gates untouched; wiring arrow benches into hosted thresholds = Track 31/ci-agent follow-up.)
- [ ] Task 3.4: Conductor — review and verify phase (workflow.md); apply accepted fixes, record rejected/cross-owner work, receipts, clean-tree check and explicit next-phase decision. (24 October 2026: `$conductor-review` over `07fb2d4..99f5b74`; accepted fixes = 2-line ffi test rustfmt drift + five pre-existing doc-lint errors surfaced by the first full `just ci` runs (intra-doc links, bare URL); rejected/cross-owner recorded (Tracks 63/07/08, 66, 31/ci-agent, Phase 4 variant retirement, staging-leftover polish); full `just ci` green — 693/693 tests, core coverage 93.32% ≥ 90%, docs/deny/audit clean; registry lockstep in phase-closeout.yaml/status.md; next-phase decision = advance to Phase 4 pending stacked PR merges. Receipts in handoff.md.)

## Phase 4 — Deletion audit and CareOps handoff

- [ ] Task 4.1: Run full Rust and relevant binding/independent-reader lanes; check exact replay and no hidden default-off or silently skipped implementation gates.
- [ ] Task 4.2: Delete superseded processing/tests/docs claims, record maintained-code reduction and retained exceptions, update public API/schema documentation and changelog.
- [ ] Task 4.3: Produce accepted immutable commit and receipt manifest for CareOps including schema fingerprint, resolved versions/features, supported platforms, artifacts and open limitations; no parent pin advance before this evidence.
- [ ] Task 4.4: Conductor — review and verify phase (workflow.md); apply accepted fixes, record rejected/cross-owner work, receipts, clean-tree check and explicit next-phase decision.

## Phase closeout gate

Before any task or phase in this track is marked complete, and before the next
phase begins:

1. Run `$conductor-review` against this track and the current diff.
2. Auto-apply accepted review fixes inside this track's owned paths.
3. Record rejected, cross-track, or blocked-path fixes in `handoff.md`.
4. Update `conductor/tracks.yaml`, `conductor/tracks.md`,
   `conductor/phase-closeout.yaml`, `conductor/status.md`,
   `conductor/implementation-readiness.md`, and `conductor/track-map.md` when
   readiness, ownership, dependency, gate, or wave data changes.
5. Run `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`
   plus the gates listed in `test-matrix.md`.
6. Commit and push the cleaned slice, then record the commit SHA or blocker in
   `handoff.md`.
7. Run `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`.
8. Advance only after there is no in-scope unstaged or untracked work except
   documented draft satellites.


Registry changes are coordinated with their current writer. This planning revision
does not close an implementation phase or authorise publication.

## Dependency sequencing refinement — 22 October 2026

Tracks 93 sha2, 94 tempfile and 96 proptest are prerequisites before runtime migration. Read-only shared inventory may start first. These tracks qualify existing retained consumers/primitives; Polars still owns real format migration and full process-interruption/durability/replay acceptance. Do not repair the obsolete pseudo-Parquet format or introduce a permanent legacy backend.
