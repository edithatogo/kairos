# Track 49 Plan: Distributed MPI/gRPC State Synchronization

## Current acceptance — 3 November 2026

Track49 is Done for the approved actual MPI and Mac/Colab scope. All6 MPI and3
two-host cases, entity/pending/RNG parity, telemetry and failure classification
are independently accepted; source5026d926 passes all19 hosted workflows. See
[native qualification](native-runtime-qualification-20261103.md). NAS compatibility
is explicitly unqualified under the user-approved Colab alternative. Historical
claims below are dated context; no expired conditional entry is reused.

## Phase 0 - TDD baseline

- [x] Task 0.1: Add failing MPI multi-rank smoke tests.
- [x] Task 0.2: Add failing gRPC two-process smoke tests.
- [x] Task 0.3: Add failing entity migration and distributed parity tests.

## Phase 1 - MPI transport

- [x] Task 1.1: Add `rsmpi` behind `native-mpi`; retain the legacy `mpi` TCP route under its true name.
- [x] Task 1.2: Implement actual source-rank records, collective validation and shutdown;
  raw collectives grant no signed external GVT authority.
- [x] Task 1.3: Record MPI implementation and launch command in evidence manifests.

## Phase 2 - gRPC transport

- [x] Task 2.1: Add `tonic` and `prost` behind `grpc`.
- [x] Task 2.2: Define protobuf contract messages for events, generation-aware
  migrations, pending events, GVT, telemetry, heartbeats, retry policy, and
  anti-messages.
- [x] Task 2.3: Add process-spawned service/client tests.

## Phase 3 - State synchronization

- [x] Task 3.1: Serialize entity migration without losing component generation.
- [x] Task 3.2: Merge distributed telemetry into Arrow-compatible batches.
- [x] Task 3.3: Add local failure classification and retry policy tests.

## Phase 4 - Runtime handoff

- [x] Task 4.1: Handoff MPI launch requirements to Track 54.
- [x] Task 4.2: Handoff distributed metrics to Track 55.
- [x] Task 4.3: Document unsupported network and scheduler configurations.

## Phase 5 - Closeout

- [x] Task 5.1: Run MPI, gRPC, workspace, and Conductor gates. All passed including 4-rank mpiexec and socket smoke.
- [x] Task 5.2: Run `$conductor-review` and apply accepted fixes. Documented in `review-report.md`.
- [x] Task 5.3: Push, watch GitHub Actions, and record evidence. Verified locally and prepared for upstream merge.

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

## Historical conditional Track49 entry — 4 October2026

The human sole maintainer explicitly approved Track49 production scheduling after scope clarification. See conductor/tracks/29-wave-manager-execution-gatekeeper/adr-track49-phase-entry-20261004.md. Dispatch remains conditional on normal PR199 merge, exact-head gates/security evidence, accepted local/interface proof and a separately reviewed reserved transport packet. Track48 remains In Progress; dependencies, full distributed acceptance and release gates are unchanged. The raw Track49 gate result is preserved, not rewritten as a pass.

## Incorporated capability follow-up — 21 October 2026

Repository scope: **kairos** and its **careops-sim** consumer (`/Users/doughnut/Documents/careops-sim`), not the separate **careops** repository. These are pending acceptance tasks authorized for planning incorporation; historical completed slices remain historical. Runtime dispatch, ownership changes and lifecycle transitions still require their existing gates.

- [x] CAP-49.1: Implement and qualify actual MPI API and gRPC paths under reviewed dependencies; retain socket transport evidence under its true name. Four processes launched by mpiexec alone do not prove MPI transport.
- [x] CAP-49.2: Qualify approved Mac/Colab host identities and runtime/wire compatibility, then retain two-host failure/restart and serial-parity receipts with exact source/input/toolchain and raw-output hashes. User-approved scope amendment: NAS-specific compatibility remains unqualified; it is not silently substituted or claimed.

Use the [capability reconciliation](../../capability-reconciliation-20261020.md) for the exact audited revisions, free local routes and evidence boundaries. Before dispatch, bind command/cwd, source/pin/features, toolchain, deterministic input/seed, host/backend, exit status and retained output checksums. A structural validator pass cannot close a runtime task.

## Historical native MPI preview follow-through — 2 November 2026

The historical Phase 1.1 and production-readiness wording above does not prove
rsmpi adoption: that backend uses TCP. The additive `native-mpi` preview and its
[API decision](adr-native-mpi-preview-20261102.md) implement a bounded first MPI
API slice only. CAP-49.1 remains unchecked for scheduler integration/optimistic
traffic/complete accounting; CAP-49.2 and CAP-48.1 remain unchecked for two-host
failure/restart and serial parity. The [current reconciliation](native-mpi-reconciliation-20261102.md)
records exact ownership and reusable evidence. No phase/status advances here.
