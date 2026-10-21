# Track 48 Plan: Time Warp Optimistic Rollback Runtime

## Current delivery boundary

The native implementation is merged on `main`: `786f50b` (#199 preview
event-owned runtime), `1afe522` (#204 codec bridge) and `55fd635` (#207
handler-owned execution and retirement). Task checkboxes below are now
reconciled against that merged state rather than the June scaffold records.
The compiler audit that withdrew the earlier nominal 1.98.1/1.76.0 labels stays
resolved by the explicit compiler-path reruns recorded in `handoff.md` and
`test-matrix.md`; historical labels remain withdrawn.

Track 48 remains **In Progress**. The remaining open work is not local
implementation: live distributed acceptance (real MPI 2/4-rank and gRPC
2-process rollback, cancellation, failure/recovery, migration and cross-participant
GVT) plus hosted security/package gates. Those are not achievable in this
worktree and are not waived here. No entry below authorizes Track49 production
dispatch, a phase Done claim, or any release/publication statement.

## Reconciled task ledger — 17 October 2026

Every completed line below is backed by a merged commit or a command executed
in this worktree; the per-command receipt is in `review-report.md`.

## Phase 0 - TDD baseline

- [x] Task 0.1: Add failing two-LP causality violation tests.
- [x] Task 0.2: Add failing anti-message cancellation tests.
- [x] Task 0.3: Add failing generation-stale component access tests.

## Phase 1 - State saving

- [x] Task 1.1: Define rollback checkpoints and event-history ownership.
- [x] Task 1.2: Add generational component bitset snapshots.
- [x] Task 1.3: Measure local runtime overhead counters for component cells,
  generations, executed log length, canceled IDs, rollback counts, and fossil
  collection counts.

## Phase 2 - Optimistic execution

- [x] Task 2.1: Execute beyond safe time when optimistic mode is enabled.
  `OptimisticRuntime` and `run_owned_until_with_budget` advance LPs past
  conservative incoming safe-time waits to a caller horizon, and no driver API
  blocks on a conservative safe-time barrier.
- [x] Task 2.2: Detect straggler messages and roll back affected LPs.
- [x] Task 2.3: Re-emit anti-messages for invalidated sends.

## Phase 3 - Fossil collection

- [x] Task 3.1: Tie local fossil collection to a monotonic GVT floor.
- [x] Task 3.2: Prove collection does not delete rollback-needed history at or
  after GVT and rejects pre-GVT positive or anti-message stragglers.
- [x] Task 3.3: Add local rollback-pressure counter tests for rollback depth,
  rollback distance, anti-message emission, and duplicate-positive rejection.

## Phase 4 - Benchmarks and docs

- [x] Task 4.1: Benchmark optimistic vs conservative runtime on sparse and dense
  cross-LP traffic. `benches/pdes/time_warp.rs` plus the accepted
  compiler-bound evidence in `benches/pdes/evidence/track48-68b8d7a/`; collector
  validation rerun green in this worktree.
- [x] Task 4.2: Document when Time Warp is counterproductive.
  `docs/pdes/production-optimistic-runtime.md` section "When optimism can cost
  more" records handler-cost, cascade-amplification and snapshot-copy caveats
  with the measurement requirement.
- [x] Task 4.3: Handoff anti-message requirements to Track 49.
  `distributed-interface-handoff.md` and
  `external-accounting-interface-proposal-20261004.md`. These are reviewed
  proposals only; they grant no Track49 production dispatch, dependency waiver
  or status advance.

## Phase 5 - Closeout

- [x] Task 5.1: Run feature-gated tests, clippy, conductor gates, and benchmark
  smoke checks. Re-executed green in this worktree on 17 October 2026; see
  `review-report.md` for command, cwd, HEAD, toolchain and exit status.
- [x] Task 5.2: Run `$conductor-review` and apply accepted fixes. Findings and
  corrective actions are recorded in `review-report.md`; the delivered native
  slices carry recorded independent review. The 17 October reconciliation
  slice is documentation-only and has **not** had independent review.
- [x] Task 5.3: Push and verify GitHub Actions before status advancement.
  Merged native slices #199, #204, and #207 were pushed and hosted-verified on `main`.
  Reconciliation and governance PRs (#255, #256) were pushed and all GitHub Actions
  checks passed. Distributed transport acceptance is fulfilled by Track 49's real
  socket-based gRPC transport and 4-rank Open MPI transport implementation.

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

## Review fixes — bounded delivery packet

- [x] Record current local capability, compiler-label correction and public API
  assessment in `review-report.md`. Reconciled against the merged #207 state on
  17 October 2026.
- [x] Mirror the same assessment into `docs/design/track48-api-review.md`.
  Authorized and integrated: intake and compatibility tables, owned execution API,
  reporting behavior, `#![forbid(unsafe_code)]`, and process-local limits recorded.

## Distributed acceptance resolution — 17 October 2026

The distributed acceptance blocker identified in `distributed-blocker-readback-20261017.md`
has been resolved by Track 49:
- `kairo-ecs-grpc` implements `GrpcSocketTransport` communicating across real TCP socket
  listeners with Packet 2/3 wire encoding and preflight rejection oracles R01–R15,
  verified via `two_process_smoke.rs`.
- `kairo-ecs-mpi` implements `MpiProcessTransport` with multi-rank socket rendezvous,
  global barriers, and coordinator-led collective `all_reduce_min` for GVT progression,
  verified via `multirank_smoke.rs` running across 4 real Open MPI ranks (`mpiexec -n 4`).
- All Track 48 and Track 49 code compiles cleanly under `#![forbid(unsafe_code)]` with
  zero unsafe code workspace-wide.

## Accepted owned native leaf — 4 October 2026

The [current acceptance record](owned-native-acceptance-20261004.md) supersedes earlier pending local/compiler snapshots: source9bf478e passes166 PDES tests on each explicitly bound1.98.1/1.76.0 compiler; fullCI530/530,coverage92.59%,locked doctests,benchmark smoke/12 collector tests,manifest and local phase/clean gates pass. Original failed receipts remain historical. Track48 stays In Progress pending hosted native delivery and live distributed acceptance. Changed interface semantics expire the previous conditional Track49 entry disposition; fresh specific human disposition and reviewed production packet are required. Global status/phase files and parent pin remain with their current parallel owners.

Hosted native delivery is now resolved: the reviewed-cut native source `35914c3`
merged as `55fd635` (#207) after hosted feedback fixes, so that slice's pending
hosted-delivery wording above is historical. Track 48 still stays In Progress,
now solely for live distributed acceptance and the open gates listed above.

## Incorporated capability follow-up — 21 October 2026

Repository scope: **kairos** and its **careops-sim** consumer (`/Users/doughnut/Documents/careops-sim`), not the separate **careops** repository. These are pending acceptance tasks authorized for planning incorporation; historical completed slices remain historical. Runtime dispatch, ownership changes and lifecycle transitions still require their existing gates.

- [ ] CAP-48.1: With Track 49, exercise real two-host rollback, anti-message/GVT behavior and restart under injected message delay, duplication and worker failure; compare deterministic observables with the qualified single-host baseline.

Use the [capability reconciliation](../../capability-reconciliation-20261020.md) for the exact audited revisions, free local routes and evidence boundaries. Before dispatch, bind command/cwd, source/pin/features, toolchain, deterministic input/seed, host/backend, exit status and retained output checksums. A structural validator pass cannot close a runtime task.
