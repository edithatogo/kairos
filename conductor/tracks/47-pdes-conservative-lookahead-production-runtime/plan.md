# Track 47 Plan: PDES Conservative Lookahead Production Runtime

## Phase 0 - TDD baseline

- [x] Task 0.1: Add failing sequential-parity tests for deterministic DES, ABM,
  and mixed workloads.
- [x] Task 0.2: Add failing lookahead-violation tests with expected typed errors.
- [x] Task 0.3: Add failing GVT monotonicity and deadlock-stress tests.

## Phase 1 - Runtime architecture

- Task 1.1: Replace scaffold-only scheduling with a feature-gated conservative
  scheduler that owns LP state, inbound queues, safe times, and null messages.
- [x] Task 1.2: Implement deterministic LP partitioning inputs and validation.
- Task 1.3: Preserve the existing scaffold API with compatibility shims or
  documented migration notes.

## Phase 2 - Correctness implementation

- [x] Task 2.1: Enforce lookahead before remote scheduling.
- [x] Task 2.2: Compute GVT from LP local time and in-flight message timestamps.
- [x] Task 2.3: Add local no-deadlock smoke behavior for stalled LPs.

## Phase 3 - Benchmark and evidence

- [x] Task 3.1: Add 4/8/16/32 LP local benchmark-smoke samples.
- Task 3.2: Record raw benchmark evidence using the Track 46 manifest fields.
- [x] Task 3.3: Add docs that distinguish local benchmark smoke from live scaling.

## Phase 4 - Integration handoff

- Task 4.1: Handoff the LP and safe-time contract to Track 49.
- Task 4.2: Handoff conservative runtime metrics to Track 55.
- Task 4.3: Record any core scheduler handoff requests without modifying
  blocked paths directly.

## Phase 5 - Closeout

- Task 5.1: Run local crate tests, full workspace checks, and conductor gates.
- Task 5.2: Run `$conductor-review`, apply accepted fixes, and update handoff.
- Task 5.3: Push and watch GitHub Actions before requesting status advancement.

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
