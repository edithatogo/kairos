# Track 49 Plan: Distributed MPI/gRPC State Synchronization

## Phase 0 - TDD baseline

- [x] Task 0.1: Add failing MPI multi-rank smoke tests.
- [x] Task 0.2: Add failing gRPC two-process smoke tests.
- [x] Task 0.3: Add failing entity migration and distributed parity tests.

## Phase 1 - MPI transport

- Task 1.1: Add `rsmpi` dependency behind `mpi`.
- Task 1.2: Implement rank discovery, send/receive, all-reduce GVT, and
  shutdown semantics.
- Task 1.3: Record MPI implementation and launch command in evidence manifests.

## Phase 2 - gRPC transport

- Task 2.1: Add `tonic` and `prost` behind `grpc`.
- Task 2.2: Define protobuf messages for events, migrations, GVT, telemetry,
  heartbeats, and anti-messages.
- Task 2.3: Add process-spawned service/client tests.

## Phase 3 - State synchronization

- Task 3.1: Serialize entity migration without losing component generation.
- Task 3.2: Merge distributed telemetry into Arrow-compatible batches.
- Task 3.3: Add failure classification and retry policy tests.

## Phase 4 - Runtime handoff

- Task 4.1: Handoff MPI launch requirements to Track 54.
- Task 4.2: Handoff distributed metrics to Track 55.
- Task 4.3: Document unsupported network and scheduler configurations.

## Phase 5 - Closeout

- Task 5.1: Run MPI, gRPC, workspace, and Conductor gates.
- Task 5.2: Run `$conductor-review` and apply accepted fixes.
- Task 5.3: Push, watch GitHub Actions, and record evidence.

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
