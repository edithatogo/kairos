# Track 32 Plan: GPU Compute Acceleration

## Phase 0 — Contract alignment with Track 01 and Track 02

### Task 0.1 — Inventory core API surfaces
- Document every entry point from Track 01 that the GPU path must reproduce: component iteration, entity query, ABM neighbor search, DES event dispatch.
- Record data layout assumptions (SoA, columnar from Arrow) that GPU kernels must match.
- Confirm that state is serializable into GPU-compatible flat buffers.

### Task 0.2 — FFI data transfer contract
- Define the host-to-GPU buffer copy protocol using Track 02 FFI primitives.
- Specify memory ownership: host allocates, GPU borrows during compute, host reads back results.
- Define zero-copy paths where driver supports it (CUDA unified memory, Vulkan host-visible buffers).

### Task 0.3 — Lock the owned surface
- All new code lives in `crates/kairo-ecs-gpu/` and `docs/gpu-compute/`.
- Do not modify `kairo-ecs-core`, `kairo-ecs-state`, `kairo-ecs-types`, or `kairo-ecs-ffi`.
- GPU features gated behind `cfg(feature = "gpu")`.

## Phase 1 — GPU API design

### Task 1.1 — Design cross-backend kernel IR
- Define a shared intermediate representation for compute kernels that maps to both WGSL (via wgpu) and CUDA (via cudarc).
- The IR abstracts: workgroup dispatch, buffer bindings, reduction operations, random number generation.
- Document IR in `docs/gpu-compute/kernel-ir.md`.

### Task 1.2 — Select and justify backends
- **wgpu** (Vulkan/Metal/DX12/WebGPU native) for cross-platform support.
- **cudarc** for CUDA-only maximum throughput on NVIDIA hardware.
- Justify: wgpu gives broad reach; cudarc gives top performance on the most common HPC GPU.
- Document tradeoffs in `docs/gpu-compute/backend-selection.md`.

### Task 1.3 — Design the GPU compute facade
- `GpuCompute` trait with `run_abm_step`, `run_des_step`, `upload_state`, `download_state`.
- Backend-agnostic API; feature flags select wgpu or cudarc at compile time.
- Integrate with `RngManager` for seed-reproducible random number generation on GPU.

## Phase 2 — Scaffold kairo-ecs-gpu crate

### Task 2.1 — Crate skeleton
- `crates/kairo-ecs-gpu/Cargo.toml` with `gpu` feature, optional dependencies on `wgpu` and `cudarc`.
- `src/lib.rs` with feature-gated re-exports.
- `src/compute.rs` containing the `GpuCompute` trait definition.

### Task 2.2 — Buffer management layer
- `src/buffer.rs` — typed GPU buffer abstraction: allocate, upload, download, zero-copy where possible.
- `src/transfer.rs` — host-GPU transfer orchestration using Track 02 FFI primitives.

### Task 2.3 — WGSL kernel scaffolding
- `src/shaders/` — directory for WGSL compute shaders.
- `src/backends/wgpu_backend.rs` — wgpu backend implementing `GpuCompute`.

### Task 2.4 — CUDA kernel scaffolding
- `src/backends/cuda_backend.rs` — cudarc backend implementing `GpuCompute`.
- `src/kernels/` — shared Rust code for kernel logic that compiles to both WGSL and CUDA PTX.

## Phase 3 — Implement kernels

### Task 3.1 — ABM particle update kernel
- WGSL compute shader for agent position/velocity update, neighbor lookup, collision detection.
- CUDA kernel mirroring the same logic.
- Shared random number generator (PCG-family) implemented in both WGSL and CUDA.

### Task 3.2 — DES event dispatch kernel
- GPU-parallel event queue processing: sort by timestamp, dispatch to workgroups, apply state mutations.
- Handle event dependencies via atomic flags or multi-pass dispatch.
- Document nondeterministic scheduling behavior at `docs/gpu-compute/event-ordering.md`.

### Task 3.3 — CPU-vs-GPU parity test
- `tests/parity.rs` — run a fixed-seed ABM step on CPU and GPU, assert identical world state.
- `tests/parity_des.rs` — same for DES event dispatch with deterministic event ordering.
- Test harness runs on CI (CPU-only fallback path) and reports GPU results when available.

## Phase 4 — Cross-track integration

### Task 4.1 — Benchmark suite integration
- Wire GPU path into Track 12 benchmark harness behind feature flag.
- Run `abm_1m` benchmark with both CPU and GPU backends.
- Publish results in `docs/gpu-compute/benchmark-results.md`.

### Task 4.2 — CI integration
- Add `gpu-crate-compiles` gate to `conductor/quality-gates.md`.
- Add `gpu-feature-isolation` gate checking `cargo tree` for unintended GPU dependencies.
- GPU benchmarks run only on self-hosted runners with GPU hardware; CI gates are informational on CPU-only runners.

### Task 4.3 — Documentation
- `docs/gpu-compute/README.md` — overview, setup, backend selection.
- `docs/gpu-compute/architecture.md` — host-GPU data flow, kernel dispatch model.
- `docs/gpu-compute/hardware-requirements.md` — tested GPUs, driver versions, platform matrix.

## Phase 5 — Handoff and closeout

### Task 5.1 — Prepare maintainer notes
- How to add a new GPU kernel.
- How to update the kernel IR when CPU semantics change.
- How to run parity tests on a specific GPU backend.
- How to update benchmark baselines after intentional kernel changes.

### Task 5.2 — Cross-track handoffs
- Notify Track 01 (core) of GPU-compatible data layout requirements.
- Notify Track 02 (FFI) of host-GPU transfer patterns that may benefit from optimized paths.
- Notify Track 09 (Wasm) of kernel designs shared with WebGPU effort (Track 33).
- Notify Track 12 (benchmarks) of GPU benchmark harness integration.
- Notify Track 15 (packaging) that GPU crate is feature-gated and optional.

### Task 5.3 — Update the risk register
- Mark resolved risks as mitigated.
- Escalate any backend that fails parity testing.
- Document known limitations for each backend (e.g., wgpu workgroup size limits on Metal).
