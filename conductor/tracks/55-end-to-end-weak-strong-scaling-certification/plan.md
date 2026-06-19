# Track 55 Plan: End-to-End Weak/Strong Scaling Certification

## Phase 0 - Scenario design

- Task 0.1: Define representative DES, ABM, hybrid, distributed, GPU, I/O, and
  FMI scenarios.
- Task 0.2: Add failing result-manifest validation for weak and strong scaling.
- Task 0.3: Add raw-result and checksum policy checks.

## Phase 1 - Weak scaling

- Task 1.1: Run weak scaling across increasing LP/rank/device counts.
- Task 1.2: Record throughput, efficiency, memory, I/O, rollback, and GVT
  metrics.
- Task 1.3: Attach raw manifests and immutable artifact references.

## Phase 2 - Strong scaling

- Task 2.1: Run fixed-workload scaling across increasing resources.
- Task 2.2: Record bottlenecks and efficiency loss.
- Task 2.3: Compare against target baseline libraries where fair comparison is
  possible.

## Phase 3 - Integrated scenario proof

- Task 3.1: Run PDES plus distributed synchronization.
- Task 3.2: Run GPU state transition batches with persistent memory.
- Task 3.3: Run checkpoint/restart and FMI co-simulation in the same scenario
  family where practical.

## Phase 4 - Certification report

- Task 4.1: Write release certification report.
- Task 4.2: Update SOTA scorecard and release-claim boundaries.
- Task 4.3: Handoff publication readiness to Tracks 42, 43, and 44.

## Phase 5 - Closeout

- Task 5.1: Run all track gates, full workspace tests, and Conductor gates.
- Task 5.2: Run `$conductor-review`, apply accepted fixes, and update handoff.
- Task 5.3: Push, watch GitHub Actions, and request final status movement only
  when all live evidence is complete.

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
