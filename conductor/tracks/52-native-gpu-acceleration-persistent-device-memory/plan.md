# Track 52 Plan: Native GPU Acceleration with Persistent Device Memory

## Phase 0 - TDD baseline

- [ ] Task 0.1: Add failing backend initialization tests for real device paths
  and explicit unavailable-device errors.
- [x] Task 0.2: Add failing CPU/GPU parity tests for DES, ABM, and mixed
  batches.
  - Red: `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-gpu
    --features wgpu-backend,cuda-backend` failed because
    `PersistentGpuSession` and `ResidentBufferKind` were absent.
  - Green: same command passed after adding the backend-independent persistent
    session contract and mixed ABM/DES parity test.
- [x] Task 0.3: Add failing persistent buffer lifecycle tests.
  - Red: same failing test covered resident particle/entity buffers,
    copy-boundary counters, and no host state download before explicit readback.
  - Green: same command passed with resident-buffer lifecycle counters.

## Phase 1 - wgpu backend

- Task 1.1: Add real `wgpu` dependency behind `wgpu-backend`.
- Task 1.2: Implement adapter/device/queue initialization.
- Task 1.3: Dispatch compute shaders and validate buffer readback.

## Phase 2 - CUDA backend

- Task 2.1: Add native CUDA binding dependency behind `cuda-backend`.
- Task 2.2: Implement context, stream, module, and kernel dispatch.
- Task 2.3: Validate device error handling and cleanup.

## Phase 3 - Persistent memory and parity

- [x] Task 3.1: Keep agent/component buffers resident across ticks.
  - Contract baseline only: `PersistentGpuSession` keeps flat particle and
    entity-value buffers resident across ABM and DES ticks.
- [x] Task 3.2: Minimize host/device copies and record copy boundaries.
  - Contract baseline only: state upload/download counters distinguish resident
    state from per-tick DES event uploads.
- [x] Task 3.3: Run CPU parity for deterministic workloads.
  - Contract baseline only: mixed ABM/DES persistent-session output matches
    `CpuFallbackCompute` for a fixed seed and timestamp-ordered DES events.

## Phase 4 - Hardware benchmarking

- Task 4.1: Add GPU benchmark harness and evidence manifests.
- Task 4.2: Run on real wgpu-capable and CUDA-capable hardware.
- Task 4.3: Handoff GPU job requirements to Track 54 and scaling metrics to
  Track 55.

## Phase 5 - Closeout

- Task 5.1: Run GPU, workspace, docs-claim, and Conductor gates.
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
