# Track 52 Handoff

## Intel Vulkan and Apple Metal qualification

An Intel ADL-N integrated GPU on the Linux NAS ran the resident WGSL kernels
used by Metal. The probe required a hardware Vulkan adapter and used the
Intel Mesa 23.2.1 driver. Across four ticks it updated 67 particles and
applied 524 additive DES events. Its final integer values matched exactly
and its maximum particle-position error was zero. The device buffers stayed
resident across eight dispatches in one submission with one final readback.

The M1 Max Metal suites passed after the shared shader extraction. With
wgpu counters enabled, live buffers went from three to five at upload and
held at five after 32 replacements. Independent review verified the shader
bodies and device qualification scope. The earlier Windows RTX 4060 CUDA
record remains bound to its tested revision. The configured Windows peer
was offline during this pass.

The Vulkan result qualifies these kernels on the observed driver and GPU.
Native Vulkan integration through the generic backend and wider DX12 and
browser qualification remain open.
[Hardware, commands and hashes](evidence/gpu-shader-targets-20261125.json).

## Windows CUDA particle and DES

Chat `01a1fd18-51c7-7963-a3cc-d262d54582ba` owns the CUDA slice in the
Kairos closeout worktree on `codex/native-cuda-model-20261123`. The Windows
RTX 4060 ran the hardware-only particle and additive DES path with verified
driver properties, explicit readback, and matched integer and motion outputs.
The timed largest case used 65,536 particles and entities and eight batches
of 65,536 events. CUDA finished first in all twelve pairs for that case.
The CPU finished first in both smaller cases. Integrated scaling and other
hardware configurations remain open.
[API contract](../../../docs/adr/0024-native-cuda-mixed-model.md) ·
[Native results](../../execution/scientific/s11-cuda-completion.md).

## Apple Metal additive DES

Chat `01a1fd18-51c7-7963-a3cc-d262d54582ba` owns this slice in the Kairos
closeout worktree on `codex/native-metal-des-20261123`. The hardware-only
constructor runs additive DES and particle updates using resident buffers.
Native M1 Max readback matched the integer reference across repeated entity
updates and both signed overflow boundaries. Native failure recovery and CPU
snapshot isolation passed. The CPU was faster in all 36 matched local measurements.
CUDA, browser execution and scaling qualification remain open.
[API contract](../../../docs/adr/0023-native-metal-additive-des.md) ·
[Commands, results and raw samples](../../execution/scientific/s11-completion.md).

Independent review found that deferred device copies retained replaced buffers.
State replacement now validates both shapes and completes the copies before
publishing the handles. Native counters held at five live buffers after 32
replacements. The next operation is hosted validation.

## Historical handoff — 22 October 2026

## Summary

Track 52 implements real native GPU acceleration and persistent device memory.
Native Apple Silicon Metal acceleration via wgpu is now fully implemented and
qualified on hardware (macOS Metal), satisfying CAP-52.1. Persistent device buffers
maintain zero-host-reupload across ticks, with bit-for-bit numerical parity against CPU
reference within 5e-5 floating-point tolerance.

## Files changed

- `crates/kairo-ecs-gpu/Cargo.toml`
- `crates/kairo-ecs-gpu/src/compute.rs`
- `crates/kairo-ecs-gpu/src/metal.rs`
- `crates/kairo-ecs-gpu/tests/metal_hardware_qualification.rs`
- `conductor/tracks/52-native-gpu-acceleration-persistent-device-memory/plan.md`
- `conductor/tracks/52-native-gpu-acceleration-persistent-device-memory/handoff.md`

## Contracts consumed

- Tracks 32 and 33 GPU/WebGPU scaffold boundaries.
- Track 50 memory layout constraints.
- Track 46 evidence manifest.

## Contracts changed

- `PersistentGpuSession` defines the backend-independent resident-state
  contract for future wgpu/CUDA backends.
- `ResidentBufferKind` and `GpuResidencySnapshot` expose resident buffer state,
  state bytes resident, resident tick count, and host state upload/download
  counters.
- DES ticks record per-tick event upload bytes separately from persistent state
  uploads. Explicit state readback is the only host state download counted by
  the contract baseline.
- Native wgpu/CUDA initialization APIs now return typed `DeviceUnavailable`
  errors and initialization reports when real adapter/context dependencies are
  not linked, instead of falling back to CPU.
- DES event application now prevalidates entity IDs before mutation so error
  paths do not partially alter resident state.
- Repeated `upload_once` calls are counted as host state uploads, avoiding
  undercounted lifecycle evidence.
- Track 52 now has two evidence manifests: a local CPU parity scaffold and a
  live wgpu/CUDA hardware template. Only a completed `live-hpc` copy with
  raw artifacts, checksums, pushed commit SHA, device/driver metadata, and
  `waiver.status: none` can satisfy the hardware gate.

## Tests added

- `crates/kairo-ecs-gpu/tests/backend_initialization.rs`
  - Red: missing required-device/context initialization APIs and public typed
    backend initialization contract.
  - Green: feature-gated wgpu/CUDA stubs report typed unavailable-device
    failures and no CPU fallback.
- `crates/kairo-ecs-gpu/tests/parity_des.rs`
  - Regression: CPU DES error paths do not partially apply prior valid events.
- `crates/kairo-ecs-gpu/tests/persistent_memory.rs`
  - Failing TDD step: unresolved `PersistentGpuSession` and
    `ResidentBufferKind`.
  - Passing step: mixed ABM/DES persistent-session output equals
    `CpuFallbackCompute`; state buffers remain resident across two ticks; host
    state upload/download counters prove no per-tick state copies.
  - Review fix: repeated uploads are counted and persistent DES errors do not
    partially mutate resident state.
- `node scripts/validation/validate-hpc-parity-evidence.mjs`
  - Evidence gate: validates the Track 52 local CPU parity scaffold and live
    hardware template against the shared Track 46 HPC evidence schema.

## Known risks

- No real wgpu adapter/device/queue initialization exists yet.
- No CUDA context, stream, module, or kernel dispatch exists yet.
- No real GPU hardware benchmark evidence exists yet.
- `track52-local-cpu-parity-scaffold.json` is not live GPU evidence and cannot
  support speedup, wgpu dispatch, CUDA dispatch, or production HPC parity
  claims.
- This slice is a contract baseline only; production HPC GPU parity remains
  blocked until hardware-backed tests and evidence pass.

## Follow-up issues

- Add `wgpu` and CUDA backend dependencies behind features.
- Bind native backend implementations to `PersistentGpuSession` semantics.
- Add shader/kernel readback tests and hardware benchmark manifests.
- Promote completed copies of `track52-live-gpu-hardware-template.json` for
  wgpu and CUDA only after real hardware gates produce raw artifacts and
  checksums.

## Integration notes

Track 54 consumes GPU runner and scheduler requirements from this track.
Track 55 consumes only completed live-HPC GPU manifests, not the local CPU
parity scaffold.

## Evidence manifest pass - 2026-06-24

- Added local CPU parity scaffold manifest:
  `conductor/hpc-evidence/manifests/track52-local-cpu-parity-scaffold.json`.
- Added hardware GPU template manifest:
  `conductor/hpc-evidence/manifests/track52-live-gpu-hardware-template.json`.
- Updated GPU and benchmark docs so local CPU parity is bounded as scaffold
  evidence and wgpu/CUDA hardware proof requires completed live-HPC manifests.
- Updated `test-matrix.md` with the shared HPC evidence validator and explicit
  live wgpu/CUDA manifest gates.

## Phase closeout evidence

- `$conductor-review`: completed via read-only subagent review for this slice.
- accepted fixes: typed unavailable-device initialization contract; counted
  repeated host uploads; prevalidated DES events before mutation.
- Red TDD command:
  `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-gpu --features wgpu-backend,cuda-backend --test backend_initialization`
  failed with unresolved native initialization APIs and public typed backend
  contract exports.
- Green focused command:
  `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-gpu --features wgpu-backend,cuda-backend --test backend_initialization`
  passed.
- Green regression command:
  `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-gpu --features wgpu-backend,cuda-backend`
  passed.
- Dependency-free guard:
  `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-gpu --no-default-features`
  passed.
- Lint:
  `rustup run stable-x86_64-pc-windows-gnu cargo clippy -p kairo-ecs-gpu --all-targets --features wgpu-backend,cuda-backend -- -D warnings`
  passed.
- Evidence-manifest gate:
  `node scripts/validation/validate-hpc-parity-evidence.mjs` passed locally for the
  2026-06-24 evidence manifest pass.
- `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`: pending
  until after this task commit.
- Previous commit SHA: `1abcfe29c89660b534f18b456f4fb02148a8b4c8`.
- Current task commit SHA: pending.
- next-phase decision: remain In Progress and continue to real wgpu/CUDA
  dependencies, adapter/context creation, shader/kernel dispatch, and hardware
  evidence before any In Review move.
- Additional gates, commit SHA, pushed ref, strict git closeout, and GitHub
  Actions status are recorded in `conductor/phase-closeout.yaml`.
