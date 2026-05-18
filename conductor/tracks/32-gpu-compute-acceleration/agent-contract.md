# Agent Contract: gpu-compute-agent

## Track

Track 32: GPU Compute Acceleration

## Owned paths

- `conductor/tracks/32-gpu-compute-acceleration/`
- `crates/kairo-ecs-gpu/`
- `docs/gpu-compute/`
- Track-specific artifacts named in `plan.md`

## Required handoff

- Summary of implemented GPU backends and their platform coverage.
- Parity test results for each backend (pass/fail, hardware tested).
- Benchmark speedup numbers vs CPU for the 1M-agent ABM scenario.
- Kernel IR document describing the shared compute model.
- Known limitations per backend (workgroup size, memory caps, unsupported operations).
- Follow-up items for core (Track 01), FFI (Track 02), Wasm (Track 09), and benchmarks (Track 12) subagents.

## Prohibited changes without ADR

- Modifying `crates/kairo-ecs-core/`, `crates/kairo-ecs-state/`, or `crates/kairo-ecs-types/` (owned by Track 01).
- Modifying the C ABI surface in `crates/kairo-ecs-ffi/` or `include/` (owned by Track 02).
- Modifying Arrow schema definitions (owned by Track 04).
- Changing the CPU scheduler's determinism guarantees or random seed behavior.
- Requiring GPU hardware for compilation (the `gpu` feature must be optional).
- Publishing GPU benchmarks as representative of CPU-only performance.
- Making any compatibility promises about GPU kernel output across different GPU vendors or driver versions.

## Gate contract

### gpu-crate-compiles
- **Input**: `crates/kairo-ecs-gpu/` source with `gpu` feature path available.
- **Output**: Pass if `cargo check --manifest-path crates/kairo-ecs-gpu/Cargo.toml --no-default-features` and `cargo check --manifest-path crates/kairo-ecs-gpu/Cargo.toml --features wgpu-backend,cuda-backend --tests` pass.
- **Blocking**: Yes for PRs that touch `crates/kairo-ecs-gpu/`. Informational only for other PRs.

### gpu-feature-isolation
- **Input**: `Cargo.toml`, `cargo tree` output without `gpu` feature.
- **Output**: Pass if no GPU dependency (`wgpu`, `cudarc`, `naga`, GPU-related sys crates) appears in the dependency tree. Fail with the offending dependency path.
- **Blocking**: Yes for PR merge — prevents accidental GPU dependency leakage into default builds.

### gpu-parity-check
- **Input**: Fixed random seed, ABM scenario definition, `GpuCompute` implementation.
- **Output**: Pass if running the same scenario through CPU and GPU paths produces identical world state. Fail with the first differing entity/component/value.
- **Blocking**: Yes for PRs that touch GPU kernel code. Informational on CPU-only CI (runs only when GPU hardware is available).

### gpu-benchmark-threshold
- **Input**: 1M-agent ABM benchmark, CPU baseline, GPU timing.
- **Output**: Pass if GPU wall-clock time achieves >=10x speedup over single-threaded CPU. Fail with actual speedup ratio.
- **Blocking**: No for PR merge (informational gate). Becomes blocking at beta when GPU hardware is provisioned.

### gpu-memory-budget
- **Input**: 10M-entity scenario, GPU memory profiling.
- **Output**: Pass if peak GPU memory allocation is under 1 GB. Fail with peak allocation and breakdown by buffer type.
- **Blocking**: No for PR merge (informational gate). Becomes blocking at RC.
