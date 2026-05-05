# Handoff: Track 32 GPU Compute Acceleration

## Status

Initial scaffold implemented and tightened. Native GPU backends now expose explicit `*-backend-not-configured` contracts instead of silently falling back to CPU work. The crate facade, buffer/transfer layers, WGSL shader scaffolds, CPU fallback parity harnesses, and GPU compute documentation exist.

## Summary

Track 32 is building toward GPU-accelerated simulation compute for KairoECS. The current `kairo-ecs-gpu` crate provides a dependency-free facade, CPU fallback contract, feature-gated wgpu/CUDA backend types, and explicit unavailable responses for real backend dispatch. GPU acceleration is optional, gated behind cargo feature flags, and non-blocking for headless release. The 10x+ speedup and 10M-entity memory targets remain future hardware-validated goals, not current claims.

## Files created in this track

- `conductor/tracks/32-gpu-compute-acceleration/spec.md`
- `conductor/tracks/32-gpu-compute-acceleration/plan.md`
- `conductor/tracks/32-gpu-compute-acceleration/agent-contract.md`
- `conductor/tracks/32-gpu-compute-acceleration/risk-register.md`
- `conductor/tracks/32-gpu-compute-acceleration/test-matrix.md`
- `conductor/tracks/32-gpu-compute-acceleration/handoff.md`
- `crates/kairo-ecs-gpu/Cargo.toml`
- `crates/kairo-ecs-gpu/src/lib.rs`
- `crates/kairo-ecs-gpu/src/compute.rs`
- `crates/kairo-ecs-gpu/src/buffer.rs`
- `crates/kairo-ecs-gpu/src/transfer.rs`
- `crates/kairo-ecs-gpu/src/wgpu/backend.rs`
- `crates/kairo-ecs-gpu/src/backends/cuda_backend.rs`
- `crates/kairo-ecs-gpu/src/shaders/abm_step.wgsl`
- `crates/kairo-ecs-gpu/src/shaders/des_dispatch.wgsl`
- `crates/kairo-ecs-gpu/tests/parity.rs`
- `crates/kairo-ecs-gpu/tests/parity_des.rs`
- `docs/gpu-compute/README.md`
- `docs/gpu-compute/kernel-ir.md`
- `docs/gpu-compute/backend-selection.md`
- `docs/gpu-compute/architecture.md`
- `docs/gpu-compute/event-ordering.md`
- `docs/gpu-compute/hardware-requirements.md`
- `docs/gpu-compute/benchmark-results.md`
- `docs/gpu-compute/maintainer-notes.md`

## Contracts consumed

- Track 01 — `crates/kairo-ecs-core/`, `crates/kairo-ecs-state/`, `crates/kairo-ecs-types/` (read-only, consumed via stable API).
- Track 02 — `crates/kairo-ecs-ffi/` (read-only, consumed for host-GPU buffer transfer primitives).
- Track 03 — ABM behavioral model semantics (read-only reference for kernel design).
- Track 04 — Arrow-based event log (read-only, consumed for parity test fixtures).
- Track 12 — Benchmark harness (read-only, consumed for speedup measurement integration).

## Contracts produced

- `crates/kairo-ecs-gpu/` — dependency-free default GPU facade with feature-gated backend-not-configured contracts.
- `docs/gpu-compute/kernel-ir.md` — shared kernel IR.
- `docs/gpu-compute/backend-selection.md` — backend tradeoffs.
- `docs/gpu-compute/event-ordering.md` — deterministic and nondeterministic DES scheduling rules.
- `docs/gpu-compute/hardware-requirements.md` — hardware evidence matrix.
- `docs/gpu-compute/benchmark-results.md` — benchmark evidence file that explicitly makes no speedup claim yet.

## Validation

- Passed: `cargo check --manifest-path crates/kairo-ecs-gpu/Cargo.toml --no-default-features`
- Passed: `cargo check --manifest-path crates/kairo-ecs-gpu/Cargo.toml --features wgpu-backend,cuda-backend --tests`
- Passed after formatting: `cargo fmt --manifest-path crates/kairo-ecs-gpu/Cargo.toml`
- Blocked: `cargo test --manifest-path crates/kairo-ecs-gpu/Cargo.toml` because this shell resolves `link.exe` to Git's `usr\bin\link.exe`, which exits with `couldn't create signal pipe, Win32 error 5`; `rust-lld` also lacks Windows SDK import libraries in this environment.

## Release gates affected

- **gpu-crate-compiles** — GPU crate compiles with feature. Blocking for PRs touching the GPU crate.
- **gpu-feature-isolation** — No GPU deps leak into default build. Blocking for all PRs.
- **cpu-gpu-parity** — GPU output matches CPU for same seed. Blocking for GPU kernel PRs.
- **gpu-speedup-threshold** — >=10x speedup on 1M-agent ABM. Informational only; becomes blocking at beta.
- **gpu-memory-budget** — Under 1 GB for 10M entities. Informational only; becomes blocking at RC.

All GPU gates are informational when no GPU hardware is present in CI. Only `gpu-feature-isolation` runs on every PR regardless of hardware.

## Risks and unresolved questions

- The shared kernel IR design must balance expressiveness against portability. WGSL and CUDA have fundamentally different memory models (binding groups vs raw pointers). The IR may need to be the lowest common denominator, limiting optimization.
- GPU parity testing on CI is blocked by lack of GPU hardware. Initial parity testing must be manual or on self-hosted runners. This delays the feedback loop for kernel changes.
- Nondeterministic workgroup scheduling means the GPU DES dispatch path is not strictly equivalent to the CPU path for all workloads. The parity test must carefully scope which scenarios are valid.
- Platform fragmentation across Metal, CUDA, and Vulkan means maintaining N backend-specific code paths for N backends. The `GpuCompute` trait abstraction helps but does not eliminate this.
- The `gpu` feature flag strategy (`#[cfg(feature = "gpu")]`) must be rigorously enforced — a single un-gated import of `wgpu` leaks GPU into every downstream crate.
