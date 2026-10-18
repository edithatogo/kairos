---
title: GPU and WebGPU
description: Apple Silicon Metal unified-memory compute, persistent GPU sessions, WGPU/CUDA backends, and WebGPU WGSL pipelines.
---

KairoECS provides two complementary GPU acceleration crates (`kairo-ecs-gpu` and `kairo-ecs-webgpu`) that preserve bit-for-bit deterministic parity with CPU simulation kernels while supporting zero-copy unified memory and multi-core workgroup execution.

## GPU Backend Matrix (`kairo-ecs-gpu` & `kairo-ecs-webgpu`)

| Backend / Session | Crate & Feature | Execution Model | Unified Memory / Zero-Copy | Determinism Contract |
| --- | --- | --- | --- | --- |
| `MetalBackend` | `kairo-ecs-gpu` (`metal-backend`) | Apple Silicon unified-memory compute (`apple-metal-unified-compute`) + MSL kernel (`abm_step`) + `std::thread::scope` multi-core workgroup dispatch | `supports_unified_memory: true`, `supports_zero_copy_borrow: true` | Bit-for-bit identical to `CpuFallbackCompute` and `PersistentGpuSession` |
| `PersistentGpuSession` | `kairo-ecs-gpu` (`gpu`) | Persistent device-resident `ParticleState` buffers with `run_abm_tick` and multi-threaded `run_abm_tick_parallel` | Zero-copy `borrow_positions_zero_copy()` slice view | Deterministic `deterministic_jitter(seed, index)` integration |
| `WgpuBackend` | `kairo-ecs-gpu` (`wgpu-backend`) | Cross-platform WebGPU/Vulkan/Metal/DX12 compute adapter + explicit `initialize_required_device()` probe | Host-staged buffer upload/download | Verified against `CpuFallbackCompute` reference |
| `CudaBackend` | `kairo-ecs-gpu` (`cuda-backend`) | NVIDIA CUDA compute adapter + explicit `initialize_required_context()` hardware probe | Device buffer staging | Verified against `CpuFallbackCompute` reference |
| `WebGpuComputePipeline` | `kairo-ecs-webgpu` (`webgpu`) | Browser/native WGSL compute shader pipeline (`@compute @workgroup_size(64)`) | Staged storage buffers | Validated in headless Chromium WebGPU lanes |

## Apple Silicon Metal & Persistent Session Example

```rust
use kairo_ecs_gpu::{
    GpuCompute, GpuState, MetalBackend, ParticleState, PersistentGpuSession,
};

let particles = (0..4096)
    .map(|i| ParticleState {
        position: [i as f32, 0.0],
        velocity: [1.0, 0.5],
    })
    .collect();
let initial_state = GpuState { particles };

// 1. Apple Silicon Metal unified-memory multi-core workgroup execution
let mut metal = MetalBackend::with_threads(8);
let adapter = MetalBackend::probe_host_adapter();
assert!(metal.capabilities().supports_unified_memory);
metal.upload_state(&initial_state).unwrap();
metal.run_abm_step(0.1, 42).unwrap();
let metal_out = metal.download_state().unwrap();

// 2. Persistent GPU session with parallel workgroup dispatch and zero-copy borrow
let mut session = PersistentGpuSession::new(64);
session.upload_initial_state(&initial_state).unwrap();
let stats = session.run_abm_tick_parallel(0.1, 42, 8).unwrap();
assert_eq!(stats.workgroups_dispatched, 64);
assert_eq!(session.borrow_positions_zero_copy(), &metal_out.particles[..]);
```

## Canonical Sources

- `docs/gpu-compute/README.md`
- `docs/gpu-compute/free-testing-routes.md`
- `docs/gpu-compute/webgpu-comparison.md`
- `docs/gpu-compute/webgpu-wgsl-subset.md`
