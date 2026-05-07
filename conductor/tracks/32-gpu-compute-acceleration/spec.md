# Track 32: GPU Compute Acceleration

## Purpose

Provide a GPU-compute acceleration path for CUDA and wgpu-backed Metal, Vulkan,
and DX12 compute shaders. The current checked-in slice is a dependency-free
contract scaffold with explicit unavailable-backend responses; the 10-100x ABM
and DES speedup target remains a hardware-validated future goal. GPU
acceleration is optional and gated behind cargo feature flags; the CPU scheduler
remains the default and always-available path.

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

- `crates/kairo-ecs-gpu/` — Rust crate with a dependency-free facade, CPU fallback contract, feature-gated backend stubs, and WGSL shader scaffolds.
- `docs/gpu-compute/` — GPU API design docs, kernel portability guide, CUDA vs wgpu tradeoffs, memory contract, and explicit no-results benchmark boundary.
- CPU-vs-GPU parity harness scaffolding that currently validates fallback/reference contracts; real GPU parity remains blocked until a backend and GPU runner exist.
- Benchmark evidence file documenting that no speedup result is currently available.
- Cargo feature flags `gpu`, `wgpu-backend`, and `cuda-backend` that gate optional GPU code paths without adding default GPU dependencies.

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

1. GPU kernel produces identical output to CPU path for the same random seed, validated by a parity test suite on a real backend.
2. 10x+ speedup on a 1M-agent ABM scenario (particle update + neighbor check) vs single-threaded CPU, with hardware, driver, command, and raw output recorded.
3. GPU memory overhead stays under 1 GB for 10M entities (buffer allocations, staging buffers, workgroup metadata), with hardware-backed measurement recorded.
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
- `gpu-parity-check` — parity test passes: GPU output matches CPU output for same seed.
- `gpu-benchmark-threshold` — benchmark shows >=10x speedup on 1M-agent ABM vs single-threaded CPU.

Memory overhead under 1 GB for 10M entities remains an acceptance criterion in the spec, not a central quality gate.
