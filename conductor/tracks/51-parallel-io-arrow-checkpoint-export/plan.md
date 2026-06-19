# Track 51 Plan: Parallel I/O, Arrow Record Batches & Checkpoint Export

## Phase 0 - TDD baseline

- Task 0.1: Add failing Arrow record batch schema tests.
- Task 0.2: Add failing HDF5 and ADIOS2 checkpoint roundtrip tests.
- Task 0.3: Add failing restart parity tests from checkpoint artifacts.

## Phase 1 - Arrow implementation

- Task 1.1: Add real Arrow dependencies behind a `parallel-io` or Arrow
  runtime feature boundary.
- Task 1.2: Build typed arrays and record batches for event logs, snapshots,
  and checkpoint metadata.
- Task 1.3: Preserve existing lightweight schema tests as compatibility guards.

## Phase 2 - Checkpoint writers

- Task 2.1: Add HDF5 writer behind `hdf5`.
- Task 2.2: Add ADIOS2 writer behind `adios2`.
- Task 2.3: Add restart readers and checksum validation.

## Phase 3 - Parallel filesystem behavior

- Task 3.1: Add contiguous block write benchmarks.
- Task 3.2: Record Lustre/GPFS/MPI-I/O evidence when available.
- Task 3.3: Document local fallback behavior when no parallel filesystem is
  available.

## Phase 4 - Integration handoff

- Task 4.1: Handoff checkpoint output collection to Track 54.
- Task 4.2: Handoff restart and throughput metrics to Track 55.
- Task 4.3: Update docs and release claim boundaries.

## Phase 5 - Closeout

- Task 5.1: Run Arrow, HDF5, ADIOS2, restart, and Conductor gates.
- Task 5.2: Run `$conductor-review` and apply accepted fixes.
- Task 5.3: Push and verify GitHub Actions.

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
