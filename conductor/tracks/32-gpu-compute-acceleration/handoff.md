# Handoff: Track 32 GPU Compute Acceleration

Last updated: 2026-05-11

## Status

Initial scaffold implemented and tightened. Native GPU backends now expose explicit `*-backend-not-configured` contracts instead of silently falling back to CPU work. The crate facade, buffer/transfer layers, host-side execution plan helpers, WGSL shader scaffolds, CPU fallback parity harnesses, GPU compute documentation, hardware-independent memory/dispatch contracts, and local feature-isolation validator exist.

## Summary

Track 32 is building toward GPU-accelerated simulation compute for KairoECS. The current `kairo-ecs-gpu` crate provides a dependency-free facade, CPU fallback contract, feature-gated wgpu/CUDA backend types, explicit unavailable responses for real backend dispatch, `GpuBackendCapabilities`, `GpuState::footprint()`, host-side `GpuExecutionPlan` helpers for ABM and DES, `DispatchShape`, and `TRACK32_TARGET_MEMORY_BUDGET`. GPU acceleration is optional, gated behind cargo feature flags, and non-blocking for headless release. The 10x+ speedup and 10M-entity memory targets remain future hardware-validated goals, not current claims.

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
- `crates/kairo-ecs-gpu/tests/contract_smoke.rs`
- `docs/gpu-compute/README.md`
- `docs/gpu-compute/kernel-ir.md`
- `docs/gpu-compute/backend-selection.md`
- `docs/gpu-compute/architecture.md`
- `docs/gpu-compute/memory-contract.md`
- `docs/gpu-compute/event-ordering.md`
- `docs/gpu-compute/hardware-requirements.md`
- `docs/gpu-compute/benchmark-results.md`
- `docs/gpu-compute/maintainer-notes.md`
- `conductor/tracks/32-gpu-compute-acceleration/validate-track32.ps1`

## Contracts consumed

- Track 01 — `crates/kairo-ecs-core/`, `crates/kairo-ecs-state/`, `crates/kairo-ecs-types/` (read-only, consumed via stable API).
- Track 02 — `crates/kairo-ecs-ffi/` (read-only, consumed for host-GPU buffer transfer primitives).
- Track 03 — ABM behavioral model semantics (read-only reference for kernel design).
- Track 04 — Arrow-based event log (read-only, consumed for parity test fixtures).
- Track 12 — Benchmark harness (read-only, consumed for speedup measurement integration).

## Contracts produced

- `crates/kairo-ecs-gpu/` — dependency-free default GPU facade with feature-gated backend-not-configured contracts.
- `crates/kairo-ecs-gpu/` — hardware-independent footprint, dispatch-shape, and backend-capability contracts for CPU-only validation.
- `docs/gpu-compute/kernel-ir.md` — shared kernel IR.
- `docs/gpu-compute/memory-contract.md` — memory budget and dispatch contract.
- `docs/gpu-compute/backend-selection.md` — backend tradeoffs.
- `docs/gpu-compute/event-ordering.md` — deterministic and nondeterministic DES scheduling rules.
- `docs/gpu-compute/hardware-requirements.md` — hardware evidence matrix.
- `docs/gpu-compute/benchmark-results.md` — benchmark evidence file that explicitly makes no speedup claim yet.

## Validation

- Passed: `cargo check --manifest-path crates/kairo-ecs-gpu/Cargo.toml --no-default-features`
- Passed: `cargo check --manifest-path crates/kairo-ecs-gpu/Cargo.toml --features wgpu-backend,cuda-backend --tests`
- Passed: `cargo tree --manifest-path crates/kairo-ecs-gpu/Cargo.toml --no-default-features` with no forbidden GPU dependency entries.
- Passed: `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\32-gpu-compute-acceleration\validate-track32.ps1 -SkipCargoTest`
- Passed after formatting: `cargo fmt --manifest-path crates/kairo-ecs-gpu/Cargo.toml`
- Optional runtime gate: `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\32-gpu-compute-acceleration\validate-track32.ps1 -RunRuntimeTests` remains host-dependent and should be used only on a machine with a working linker/runtime path.

## Release gates affected

- **gpu-crate-compiles** — GPU crate compiles with feature. Blocking for PRs touching the GPU crate.
- **gpu-feature-isolation** — No GPU deps leak into default build. Blocking for all PRs.
- **gpu-parity-check** — GPU output matches CPU for same seed. Blocking for GPU kernel PRs.
- **gpu-benchmark-threshold** — >=10x speedup on 1M-agent ABM. Informational only; becomes blocking at beta.
- Memory budget under 1 GB for 10M entities remains an acceptance criterion, not a central gate.

All GPU gates are informational when no GPU hardware is present in CI. Only `gpu-feature-isolation` runs on every PR regardless of hardware.

## Risks and unresolved questions

- The shared kernel IR design must balance expressiveness against portability. WGSL and CUDA have fundamentally different memory models (binding groups vs raw pointers). The IR may need to be the lowest common denominator, limiting optimization.
- GPU parity testing on CI is blocked by lack of GPU hardware. Initial parity testing must be manual or on self-hosted runners. This delays the feedback loop for kernel changes.
- CPU-only validation can now catch memory footprint and dispatch-shape drift, but it cannot prove backend-specific alignment, allocation overhead, or real device limits.
- Nondeterministic workgroup scheduling means the GPU DES dispatch path is not strictly equivalent to the CPU path for all workloads. The parity test must carefully scope which scenarios are valid.
- Platform fragmentation across Metal, CUDA, and Vulkan means maintaining N backend-specific code paths for N backends. The `GpuCompute` trait abstraction helps but does not eliminate this.
- The `gpu` feature flag strategy (`#[cfg(feature = "gpu")]`) must be rigorously enforced — a single un-gated import of `wgpu` leaks GPU into every downstream crate.
- Low-cost smoke routes are documented in `docs/gpu-compute/free-testing-routes.md`: use GitHub macOS or the M1 MacBook Pro for Metal-adjacent checks, and NVIDIA NIM for NVIDIA-GPU-backed library compatibility smoke when an endpoint is available. These routes do not replace parity or benchmark evidence.

## Files changed

No additional file list was recorded by this Conductor hygiene update. Use the track plan, spec, and git history for implementation-specific file evidence.


## Contracts changed

No contract changes were recorded by this Conductor hygiene update.


## Tests added

No tests were added by this Conductor hygiene update.


## Known risks

No new risks were introduced by this Conductor hygiene update.


## Follow-up issues

No additional follow-up issues were recorded by this Conductor hygiene update.


## Integration notes

No additional integration notes were recorded by this Conductor hygiene update.
## Phase closeout evidence

Clean closeout is still blocked on this host, but the validation record is now narrower and more precise: the crate scaffold, CPU-only feature isolation, documentation, `cargo check --manifest-path crates/kairo-ecs-gpu/Cargo.toml --no-default-features`, `cargo check --manifest-path crates/kairo-ecs-gpu/Cargo.toml --features wgpu-backend,cuda-backend --tests`, `cargo tree --manifest-path crates/kairo-ecs-gpu/Cargo.toml --no-default-features`, `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\32-gpu-compute-acceleration\validate-track32.ps1 -SkipCargoTest`, and `cargo fmt --manifest-path crates/kairo-ecs-gpu/Cargo.toml` all passed. The remaining blockers are the missing GPU-capable runner or workstation needed to produce executable parity and benchmark evidence; runtime parity is intentionally an explicit opt-in gate through `-RunRuntimeTests`. Before this track can move to `Done`, record `$conductor-review` findings, accepted fixes, deferred or blocked fixes, validation commands, cleanup state, commit SHA or explicit push blocker, pushed ref, strict `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` result, and next-phase decision here.

## Next-phase decision

Remain `In Review`. The host-side planning and compile-only gates are in place,
but real GPU parity, performance, and benchmark evidence still need hardware.
