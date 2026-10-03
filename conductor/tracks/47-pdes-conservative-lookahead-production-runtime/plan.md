# Track 47 Plan: PDES Conservative Lookahead Production Runtime

## Phase 0 - TDD baseline

- [x] Task 0.1: Add failing sequential-parity tests for deterministic DES, ABM,
  and mixed workloads.
- [x] Task 0.2: Add failing lookahead-violation tests with expected typed errors.
- [x] Task 0.3: Add failing GVT monotonicity and deadlock-stress tests.

## Phase 1 - Runtime architecture

- [x] Task 1.1: Replace scaffold-only scheduling with a feature-gated conservative
  scheduler that owns LP state, inbound queues, safe times, and null messages.
- [x] Task 1.2: Implement deterministic LP partitioning inputs and validation.
- [x] Task 1.3: Preserve the existing scaffold API with compatibility shims or
  documented migration notes.

## Phase 2 - Correctness implementation

- [x] Task 2.1: Enforce lookahead before remote scheduling.
- [x] Task 2.2: Compute GVT from LP local time and in-flight message timestamps.
- [x] Task 2.3: Add local no-deadlock smoke behavior for stalled LPs.

## Phase 3 - Benchmark and evidence

- [x] Task 3.1: Add 4/8/16/32 LP local benchmark-smoke samples.
- [x] Task 3.2: Record raw benchmark evidence using the Track 46 manifest fields.
- [x] Task 3.3: Add docs that distinguish local benchmark smoke from live scaling.

## Phase 4 - Integration handoff

- [x] Task 4.1: Handoff the LP and safe-time contract to Track 49.
- [x] Task 4.2: Handoff conservative runtime metrics to Track 55.
- [x] Task 4.3: Record any core scheduler handoff requests without modifying
  blocked paths directly.

## Phase 5 - Closeout

- [x] Task 5.1: Run local crate tests, full workspace checks, and conductor gates.
- [x] Task 5.2: Run `$conductor-review`, apply accepted fixes, and update handoff.
- [x] Task 5.3: Push and watch GitHub Actions before requesting status advancement.

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


## Production implementation review (2026-10-03)

The earlier checked Phase 0/2/3 items described helper/scaffold evidence. The
new source consumes those compatibility contracts and adds actual event-owned
threaded execution. Acceptance now uses `production_parity.rs` (18 model/seed/LP
cases via actual core Scheduler) and `production_protocol.rs` (14 tests,
including random and adversarial 8-LP 10,000-tick runtime progress), rather than
using helper reference parity as production proof. No historical failing-test
log is manufactured for the inherited helper checks.

Architecture/compatibility and integration handoff items are implemented in
`adr-production-runtime.md` and `docs/pdes/production-conservative-runtime.md`.
Phase 3.2 and 5.3 remain open until actual raw capture and hosted
closeout commands finish. Phase 5.1 has local Rust evidence recorded in the handoff;
final Conductor/clean Git acceptance remains pending. Source review fixes and executed gates are recorded
in the handoff; status advancement remains coordinated with the root ledgers.

Raw capture task 3.2 is now evidenced by the remotely verified `45f01c49be1d15cefb64ad48e59ee0a7e4146b3e` host bundle and registered manifest. Hosted checks, push closeout and final acceptance remain pending.

Hosted acceptance is blocked by the unchanged npm audit gate for GHSA-ch52-4w7c-c8xp (no published fixed dependency). Fifteen workflow runs passed on PR head `2502a4c`; follow-up review fixes require fresh hosted checks. Task 5.3 and track acceptance remain incomplete.

The human-approved EXC-193 now provides a bounded alternative to waiting for an official cache dependency release. Final local regression tests, exact-head hosted Actions and clean Git closeout remain required before Task 5.3 or track acceptance is complete.

Reviewed-source closeout: all 19 workflow runs succeeded at 8a6bf66, strict clean Git closeout passed, and independent hosted security review accepted the bounded EXC-193 classification with raw evidence retained. This supersedes earlier hosted-blocker entries above. The final status-only closeout commit requires fresh exact-head Actions before merge; Track 48 begins only after actual merge.
