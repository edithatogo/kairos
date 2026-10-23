# Plan: Polars-native data processing and standard interchange

All implementation work is pending. Write failing behavioural tests before replacement.

## Phase 0 — Baseline and dependency qualification

- [ ] Task 0.1: Inventory every applicable Rust/binding/script data path and caller; record replace/retain/remove decisions, raw baselines and ownership handoffs from Tracks 04/62/63/66. Classify 93/94/96 (or paired parent) targets as removed/required/independent; record exact blocked migration task, owner, prerequisite receipt and oracle before affected dispatch. Record no-target decisions explicitly; do not mark adoption complete.
- [ ] Task 0.2: Add failing independent interoperability, full-range tick, join/cardinality and ABI ownership tests before replacement; establish real baseline behaviours rather than trusting Done labels.
- [ ] Task 0.3: Qualify and pin compatible Polars/Arrow versions, feature set, licence/advisory/MSRV and packaging constraints on the canonical toolchain; prove minimal-core/Wasm feature isolation and no duplicate full table representation.
- [ ] Task 0.4: Review ADR, versioned schema/ABI compatibility assessment, fixture changes and red-team objections; capture approved contract and source/dependency hashes.
- [ ] Task 0.5: Conductor — review and verify phase (workflow.md); apply accepted fixes, record rejected/cross-owner work, receipts, clean-tree check and explicit next-phase decision.

## Phase 1 — Native tables and real formats

- [ ] Task 1.1: Add failing table/schema/extreme-tick/invalid-input and independent file-reader tests.
- [ ] Task 1.2: Replace custom schemas/buffers/codecs with upstream tables and Parquet/Zstd/IPC writers; implement exact ID/tick mapping and manifest provenance.
- [ ] Task 1.3: Replace applicable Rust tabular transforms with Polars expressions; remove obsolete encoders/readers and descriptor smoke paths; retain explicit reasons for exclusions.
- [ ] Task 1.4: Conductor — review and verify phase (workflow.md); apply accepted fixes, record rejected/cross-owner work, receipts, clean-tree check and explicit next-phase decision.

## Phase 2 — FFI and polyglot consumers

- [ ] Task 2.1: Add failing populated multi-batch/empty/null/lifetime/release tests and affected binding consumer fixtures.
- [ ] Task 2.2: Implement owned Arrow C Data arrays/streams and native Python-to-Polars handoff; remove dummy capsules and Python-row reconstruction.
- [ ] Task 2.3: Update C headers, public data API users, all affected native bindings and browser/minimal-build paths; document actual copies and unsupported targets explicitly.
- [ ] Task 2.4: Conductor — review and verify phase (workflow.md); apply accepted fixes, record rejected/cross-owner work, receipts, clean-tree check and explicit next-phase decision.

## Phase 3 — Durable output and execution budgets

- [ ] Task 3.1: Add failing interruption/disk-full/invalid-manifest/resume-watermark and memory/thread-budget tests.
- [ ] Task 3.2: Implement bounded native output batches, manifest-last publication, explicit sort keys and checkpoint payload/control separation without changing scheduler semantics.
- [ ] Task 3.3: Benchmark current small and large traces, high-cardinality joins and concurrent workloads; preserve existing regression gates and record memory/build/binary tradeoffs.
- [ ] Task 3.4: Conductor — review and verify phase (workflow.md); apply accepted fixes, record rejected/cross-owner work, receipts, clean-tree check and explicit next-phase decision.

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

Track 70 starts with Phase 0 inventory and migration-boundary decisions; Tracks 93/94/96 are not blanket entry or runtime-migration prerequisites. For each digest, staging or property-test target, record its path, owner and disposition: removed by Polars, retained and required for a named migration task, or independent. Before dispatching an affected task, record the exact prerequisite task/receipt and acceptance oracle in the owning plan and dispatch packet (and the CareOps task graph where representable). Unresolved required work blocks that task only; unrelated migration work may proceed under existing ownership and contract gates. Do not retrofit code scheduled for deletion. If no targets remain, record a reviewed scope-elimination decision without claiming adoption. Required integrity, durability, interoperability, replay and independent test evidence remain mandatory in Polars. Separate dependency tracks and current owner handoffs remain in force.
