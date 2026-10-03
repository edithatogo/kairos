# Review Report: Track 48 local optimistic runtime delivery

## Summary

The bounded local implementation is accepted for feature-preview review with actual pinned Rust1.98 workspace evidence; compiler-label corrections, actual MSRV rerun and coordinator metadata work are now accepted, while distributed/security/release acceptance remains separate.

Review scope is `origin/main..7a432ab46c3bfa5f3917f1cce0f4545fac39f649`
(20 non-Conductor files, 7053 insertions, six deletions), plus this isolated
delivery-doc packet based on that exact commit. Runtime source was reviewed in
bounded semantic slices at `3cd6d56`; benchmark source/evidence was reviewed at
`ec9828e`. This packet changes only Track48 documents and the new API review.
Its resulting commit is reported to the coordinator after commit; this document
does not invent its own future SHA or imply every historical file was reread.

## Verification Checks

- [x] **Plan Compliance**: Partial — local event-owned state/replay/cancellation/GVT/bitset behavior and sparse/dense evidence exist; Track48 remains In Progress pending live distributed integration.
- [x] **Style Compliance**: Pass for the bounded source/docs review; root pinned fmt, Clippy and rustdoc pass. No source edit is part of this packet.
- [x] **New Tests**: Yes — 13 independent runtime and two bitset/model held-outs supplement worker protocol and exhaustion cases, with collector negative tests.
- [x] **Test Coverage**: Partial — meaningful local ordering/state/RNG/cascade/failure oracles plus root core coverage512/553 (92.59%, floor90%); no distributed/HPC or total-model-memory proof.
- [x] **Test Results**: Passed local behavior under actual Rust1.99, followed by coordinator pinned Rust1.98 `just ci` at7a432ab: 458 tests, zero skipped, fmt/Clippy/rustdoc/deny/audit pass. Actual Rust1.76 crate rerun is pending; hosted exact-head checks remain pending.

The coordinator's resolved native receipt is
`artifacts/track48-final-validation/receipt-resolved-pinned.json`, command
`just ci`, cwd the programme worktree, source7a432ab, exit0, log SHA256
`1512af0eb93c132f03805297981a75ea73db233036d6f23dc5c7d18932985208`.
It explicitly binds Rust1.98.1 RUSTC/PATH, matching LLVM tools and fresh target.
The earlier wrapper-labelled independent receipt remains at
`/private/tmp/kairos-track48-independent-runtime/artifacts/track48-independent-runtime/receipt.json`;
it is behavioral evidence under actual Homebrew Rust1.99, not MSRV proof.

At7a432ab the coordinator also reports collector nine-case, local manifest,
phase, DAG and RequireClean gates passed. Strict source-bound benchmark raw files
at `benches/pdes/evidence/track48-ec9828e/` preserve actual local parity and five
alternating samples per case. Its Cargo compiler labels require reconciliation.
No native build was executed during this documentation packet; prior independent
tests and current coordinator receipts remain distinguishable.

## Findings

### High — Nominal toolchain labels did not bind the compiler

- **File**: `handoff.md` (historical independent acceptance paragraphs) and the local independent receipt/Cargo target caches.
- **Context**: Both nominal1.98 and1.76 targets identify Homebrew rustc1.99.0/LLVM23.1.2. Wrapper `rustc --version` metadata did not establish Cargo-selected compiler or the claimed MSRV.
- **Suggestion**: Preserve original command/log evidence, withdraw pinned labels, and require explicit PATH/RUSTC plus actual Cargo-cache/compiler evidence and fresh targets.
- **Corrective action**: Current handoff/test matrix/API review make the correction. Coordinator pinned1.98 `just ci` now passes; driver actual1.76 rerun and benchmark compiler reconciliation remain pending. No pinned MSRV pass is fabricated.

### Medium — Public API review and current handoff were incomplete

- **File**: `docs/design/track48-api-review.md`, `handoff.md`, `plan.md`, `test-matrix.md`.
- **Context**: CONTRIBUTING requires API review, but the terse ADR did not record ownership/errors/thread safety/compatibility/red-team assessment; historical scaffold descriptions obscured current local behavior.
- **Suggestion**: Use applicable API review forms, label historical sections and record current evidence with precise limits.
- **Corrective action**: This packet adds the review and current local summary. The existing PDES root is outside the protected inventory; no invented protected-root enrollment, stable promotion or policy waiver is asserted.

### Medium — Release notes and global Conductor narratives need coordinator reconciliation

- **File**: `CHANGELOG.md`, `conductor/phase-closeout.yaml`, `conductor/status.md`, `conductor/implementation-readiness.md`.
- **Context**: The changelog lacks the new feature-preview API/legacy repairs, and global narratives still describe replay/downstream antis/benchmarks as unimplemented. Their ownership is outside this packet.
- **Suggestion**: Add a user-facing affected-crate release note and update local capability, current review/commands/commits/evidence and blockers consistently; retain Track48 In Progress and Track49 dependencies.
- **Corrective action**: Handed to coordinator. Do not count these changes as complete until their owned commit/gates exist; synchronize tracks.yaml/tracks.md/track-map only as applicable to changed registry facts.

### High — Distributed/security acceptance cannot follow from local passes

- **File**: `test-matrix.md`, `distributed-interface-handoff.md`, Track49 dependency record.
- **Context**: Cross-process/rank straggler repair, downstream cancellation/redelivery, participant GVT and live artifacts remain required. Existing EXC-193 applies only to PR193; local cargo-deny/audit do not waive unresolved security/release gates.
- **Suggestion**: Keep local preview acceptance separate from distributed, hosted/security, dependency and release acceptance.
- **Corrective action**: Preserved. Coordinator live-quality readback reports main-branch CodeQL/Scorecard alert482 failure, including `GHSA-vfj7-8cjw-p6xm` (braces<=3.0.3, patched version absent in that readback), missing Codecov project status despite upload success and pending Renovate refresh afterPR197. No alert dismissal/bypass or extension of EXC-193 is authorized. No Track49 production dispatch, dependency change, archive/delete or Track48 Done is authorized. Final Actions/push/merge and any required scheduling decision remain coordinator/user gates.

## Final compiler-bound local delivery — 2026-10-03

Reviewed collector504aa4a integrated as68b8d7a. Absolute Cargo/rustc1.98.1, binary/configuration hashes, wrapper/selector refusal and fixed baseline flags resolve compiler drift. At clean68b8d7a,12 collector tests and strict fresh benchmark passed; independent review verified actual Cargo cache1.98.1/LLVM22.1.8,16 unchanged source hashes, raw logs and four parity cases with five alternating samples. Canonical evidence: benches/pdes/evidence/track48-68b8d7a/, SHAe7d08f74b0c448cac281cd95199e24cf9d34218a303a80ad0a66f8e1323cbc8c. Scope is tiny single-host lightweight-handler run-call smoke, with substantial timing variation and excluded setup/extraction/validation/fossil costs. No general speedup, CPU-concurrency or distributed claim.

Explicit1.98 workspace CI458/coverage92.59% and matching1.76 local PDES94 tests are accepted. The workspace doc-test command at7c35f2a exited0 across25 targets with zero runnable examples, separately preserved in benches/pdes/evidence/track48-doctests-7c35f2a/. CHANGELOG d62891e and global evidence synchronization eb02a14 are committed; previous pending statements describe historical review snapshots. Exact-head push/hosted security/package gates and live distributed acceptance remain pending. Track48 stays In Progress; Track49 dependency/production authority unchanged.

During compiler-fix coordination, an expired worker lease was explicitly recovered only after interruption; exact edits were preserved in stashes. A displayed cooperative lease token was revoked and rotated before continuation. No token is committed or active. Original failed attempts and superseded raw evidence remain preserved.
