# Track 52 Plan: Native GPU Acceleration with Persistent Device Memory

## Phase 0 - TDD baseline

- [x] Task 0.1: Add failing backend initialization tests for real device paths
  and explicit unavailable-device errors.
  - Red: `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-gpu --features wgpu-backend,cuda-backend --test backend_initialization` failed with missing native initialization constants, public backend kind re-export, and required-device/context methods.
  - Green: same command passed after adding typed unavailable-device initialization reports for feature-gated wgpu and CUDA stubs, with no silent CPU fallback.
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

- [x] Task 1.1: Add real `wgpu` dependency behind `wgpu-backend` (exercising Apple Silicon Metal natively on macOS via `macos-local-apple-silicon-metal-mpi-hwloc` and Vulkan/DX12 on Linux/Windows).
- [x] Task 1.2: Implement adapter/device/queue initialization with explicit hardware adapter metadata reporting.
- [x] Task 1.3: Dispatch WGSL compute shaders and validate persistent buffer readback parity on local macOS Metal and GitHub Actions `macos-metal-smoke`.

## Phase 2 - CUDA backend

- [x] Task 2.1: Add native CUDA binding (`cudarc` / driver API) dependency behind `cuda-backend` via dedicated `crates/kairo-ecs-cuda` bridge crate.
- [x] Task 2.2: Implement context, stream, module, and kernel dispatch (verifiable at zero cost via `free-cloud-cuda-sandboxes-colab-kaggle-lightning` on Kaggle/Colab NVIDIA T4 GPUs when local host is macOS).
- [x] Task 2.3: Validate device error handling and cleanup.

## Phase 3 - Persistent memory and parity

- [x] Task 3.1: Keep agent/component buffers resident across ticks.
  - Contract baseline and Metal/CUDA implementations: `PersistentGpuSession` keeps flat particle and
    entity-value buffers resident across ABM and DES ticks with zero-host-reupload device residency.
- [x] Task 3.2: Minimize host/device copies and record copy boundaries.
  - Contract baseline and Metal/CUDA implementations: state upload/download counters distinguish resident
    state from per-tick DES event uploads.
- [x] Task 3.3: Run CPU parity for deterministic workloads.
  - Contract baseline and Metal/CUDA implementations: mixed ABM/DES persistent-session output matches
    `CpuFallbackCompute` for a fixed seed and timestamp-ordered DES events.

## Phase 4 - Hardware benchmarking

- [x] Task 4.1: Add GPU benchmark harness and evidence manifests.
- [x] Task 4.2: Run on real wgpu-capable and CUDA-capable hardware.
- [x] Task 4.3: Handoff GPU job requirements to Track 54 and scaling metrics to
  Track 55.

## Phase 5 - Closeout

- [x] Task 5.1: Run GPU, workspace, docs-claim, and Conductor gates.
- [x] Task 5.2: Run `$conductor-review` and apply accepted fixes.
- [x] Task 5.3: Push and verify GitHub Actions.

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

## Incorporated capability follow-up — 21 October 2026

Repository scope: **kairos** and its **careops-sim** consumer (`/Users/doughnut/Documents/careops-sim`), not the separate **careops** repository. These are pending acceptance tasks authorized for planning incorporation; historical completed slices remain historical. Runtime dispatch, ownership changes and lifecycle transitions still require their existing gates.

- [x] CAP-52.1: Qualify native Mac Metal kernel submission, buffer readback and CPU/reference parity with actual device provenance; reject CpuFallback as device acceptance.
- [x] CAP-52.2: Independently verify the separate Pleiades CUDA computer OS/GPU/driver/toolkit/access; the historical bare pleiades SSH alias addresses the NAS. Qualify real kernels, memory limits, cancellation/cleanup and separate initialization/transfer/compute timing before promotion.

Use the [capability reconciliation](../../capability-reconciliation-20261020.md) for the exact audited revisions, free local routes and evidence boundaries. Before dispatch, bind command/cwd, source/pin/features, toolchain, deterministic input/seed, host/backend, exit status and retained output checksums. A structural validator pass cannot close a runtime task.
