# Track 32: GPU Compute Acceleration

## Purpose

Provide GPU-accelerated simulation compute via CUDA, Metal, and Vulkan compute shaders. Target 10-100x speedup for ABM particle updates and DES event dispatch running on GPU hardware. GPU acceleration is optional and gated behind a cargo feature flag — the CPU scheduler remains the default and always-available path.

## Why this track exists

KairoECS targets large-scale agent-based and discrete-event simulations. A 1M-agent ABM scenario strains even an optimized CPU scheduler. GPUs offer orders of magnitude more parallelism for the data-parallel workloads central to ABM particle updates. This track delivers a GPU compute backend that shares kernel designs with the CPU path, producing deterministic identical output for the same random seed, and scales to workloads that would be impractical on CPU alone.

## Primary subagent

`gpu-compute-agent`

## Parallelization model

This track depends on the core scheduler contract (Track 01) and the FFI layer (Track 02) for host-GPU data transfer. It starts once those tracks produce stable API surfaces. The GPU crate lives alongside existing crates and does not modify them. The track communicates with Track 09 (Wasm) and Track 05 (viz) through contract-level agreements rather than code dependencies.

## Inputs

- `crates/kairo-ecs-core/` + `crates/kairo-ecs-state/` — world state, entity IDs, component layout from Track 01.
- `crates/kairo-ecs-ffi/` — host-GPU buffer transfer via FFI from Track 02.
- `conductor/tracks/04-analyst-kairo-ecs-arrow/` — Arrow-based event log for parity testing.
- `conductor/tracks/12-conformance-testing-benchmarks/` — benchmark harness for speedup measurement.
- ABM particle kernel semantics from Track 03 behavioral layer.

## Outputs

- `crates/kairo-ecs-gpu/` — Rust crate with wgpu compute shaders (Metal/Vulkan/DX12 cross-platform) + cudarc for CUDA.
- `docs/gpu-compute/` — GPU API design doc, kernel portability guide, CUDA vs wgpu tradeoffs.
- CPU-vs-GPU parity test suite proving identical output for same random seed.
- Benchmark results documenting speedup (1M+ agent ABM scenario, 10M+ entity stress test).
- Cargo feature flag `gpu` that gates all GPU code paths.

## Owned paths

- `crates/kairo-ecs-gpu/`
- `docs/gpu-compute/`
- `conductor/tracks/32-gpu-compute-acceleration/`

## Blocked paths

- `crates/kairo-ecs-core/` — owned by Track 01.
- `crates/kairo-ecs-state/` — owned by Track 01.
- `crates/kairo-ecs-types/` — owned by Track 01.
- `crates/kairo-ecs-ffi/` — owned by Track 02.
- `benches/` and `crates/kairo-ecs-bench/` — owned by Track 12.

## Acceptance criteria

1. GPU kernel produces identical output to CPU path for the same random seed, validated by parity test suite.
2. 10x+ speedup on a 1M-agent ABM scenario (particle update + neighbor check) vs single-threaded CPU.
3. GPU memory overhead stays under 1 GB for 10M entities (buffer allocations, staging buffers, workgroup metadata).
4. The `gpu` cargo feature compiles all GPU code; without the feature, zero GPU dependency enters the build.
5. Parity test passes on at least one GPU backend (CUDA or wgpu with Vulkan/Metal).
6. Benchmark script in `docs/gpu-compute/` reproduces the speedup claim on documented hardware.

## Release implications

- GPU acceleration is **optional and non-blocking** for headless release. The CPU scheduler is always available.
- GPU features are gated behind `cfg(feature = "gpu")` and never affect default build behavior.
- GPU crate is not included in the minimal release profile; downstream users opt in.
- A release may ship with GPU support labeled experimental if parity testing is incomplete.

## Non-goals

- GPU rendering or visualization (Track 05 viz handles that).
- Replacing the CPU scheduler — GPU is an accelerator, not a scheduler replacement.
- Requiring GPU hardware for basic operation — the project must remain fully functional on CPU.
- Targeting integrated GPUs or mobile GPUs in the initial scope.
- Writing GPU kernels for every possible DES event variant — focus on the ABM particle kernel and a representative DES dispatch kernel.

## Quality gates

Use the gates in `conductor/quality-gates.md`. Track-specific gates:
- `gpu-crate-compiles` — the `kairo-ecs-gpu` crate compiles with the `gpu` feature on target platforms.
- `gpu-feature-isolation` — without the `gpu` feature, no GPU dependency appears in `cargo tree`.
- `cpu-gpu-parity` — parity test passes: GPU output matches CPU output for same seed.
- `gpu-speedup-threshold` — benchmark shows >=10x speedup on 1M-agent ABM vs single-threaded CPU.
- `gpu-memory-budget` — memory overhead under 1 GB for 10M entities.
