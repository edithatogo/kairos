# Track 48 Plan: Time Warp Optimistic Rollback Runtime

## Phase 0 - TDD baseline

- [x] Task 0.1: Add failing two-LP causality violation tests.
- [x] Task 0.2: Add failing anti-message cancellation tests.
- [x] Task 0.3: Add failing generation-stale component access tests.

## Phase 1 - State saving

- [x] Task 1.1: Define rollback checkpoints and event-history ownership.
- [x] Task 1.2: Add generational component bitset snapshots.
- Task 1.3: Measure memory overhead per LP and event density.

## Phase 2 - Optimistic execution

- Task 2.1: Execute beyond safe time when optimistic mode is enabled.
- [x] Task 2.2: Detect straggler messages and roll back affected LPs.
- [x] Task 2.3: Re-emit anti-messages for invalidated sends.

## Phase 3 - Fossil collection

- Task 3.1: Tie fossil collection to GVT.
- Task 3.2: Prove collection does not delete rollback-needed history.
- Task 3.3: Add high-rollback pressure stress tests.

## Phase 4 - Benchmarks and docs

- Task 4.1: Benchmark optimistic vs conservative runtime on sparse and dense
  cross-LP traffic.
- Task 4.2: Document when Time Warp is counterproductive.
- Task 4.3: Handoff anti-message requirements to Track 49.

## Phase 5 - Closeout

- Task 5.1: Run feature-gated tests, clippy, conductor gates, and benchmark
  smoke checks.
- Task 5.2: Run `$conductor-review` and apply accepted fixes.
- Task 5.3: Push and verify GitHub Actions before status advancement.

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
