# Test Matrix: Track 32 GPU Compute Acceleration

Rule: a check is marked complete only when an artifact exists and the validation command is recorded here.

| Check | Status | Artifact | Validation |
|---|---|---|---|
| Track docs exist and render cleanly | complete | `conductor/tracks/32-gpu-compute-acceleration/*.md` | `rg -n "GPU Compute" conductor/tracks/32-gpu-compute-acceleration` |
| `crates/kairo-ecs-gpu/` crate skeleton exists with `Cargo.toml` | complete | `crates/kairo-ecs-gpu/Cargo.toml`, `src/lib.rs` | `cargo check --manifest-path crates/kairo-ecs-gpu/Cargo.toml --no-default-features` |
| `gpu` cargo feature exists and gates backend modules | complete | `crates/kairo-ecs-gpu/Cargo.toml`, `src/lib.rs` | `cargo check --manifest-path crates/kairo-ecs-gpu/Cargo.toml --no-default-features` |
| Default build has zero GPU dependencies | complete | dependency-free default manifest | `cargo check --manifest-path crates/kairo-ecs-gpu/Cargo.toml --no-default-features` |
| `GpuCompute` trait defined with ABM, DES, upload, and download methods | complete | `crates/kairo-ecs-gpu/src/compute.rs` | `cargo check --manifest-path crates/kairo-ecs-gpu/Cargo.toml --no-default-features` |
| Host-side execution plan helpers exist for ABM and DES | complete | `crates/kairo-ecs-gpu/src/compute.rs`, `crates/kairo-ecs-gpu/src/lib.rs`, `docs/gpu-compute/memory-contract.md` | `cargo check --manifest-path crates/kairo-ecs-gpu/Cargo.toml --no-default-features`; `cargo check --manifest-path crates/kairo-ecs-gpu/Cargo.toml --features wgpu-backend,cuda-backend --tests` |
| Buffer management layer exists and has unit tests | complete | `crates/kairo-ecs-gpu/src/buffer.rs` | `cargo check --manifest-path crates/kairo-ecs-gpu/Cargo.toml --tests` |
| Transfer planning layer exists and has unit tests | complete | `crates/kairo-ecs-gpu/src/transfer.rs` | `cargo check --manifest-path crates/kairo-ecs-gpu/Cargo.toml --tests` |
| Hardware-independent memory footprint contract exists | complete | `crates/kairo-ecs-gpu/src/compute.rs`, `crates/kairo-ecs-gpu/tests/contract_smoke.rs`, `docs/gpu-compute/memory-contract.md` | `cargo check --manifest-path crates/kairo-ecs-gpu/Cargo.toml --no-default-features`; `cargo check --manifest-path crates/kairo-ecs-gpu/Cargo.toml --tests` |
| Backend capability contract is explicit for fallback and feature-gated stubs | complete | `src/compute.rs`, `src/wgpu/backend.rs`, `src/backends/cuda_backend.rs` | `cargo check --manifest-path crates/kairo-ecs-gpu/Cargo.toml --features wgpu-backend,cuda-backend --tests` |
| Local feature-isolation validator exists | complete | `conductor/tracks/32-gpu-compute-acceleration/validate-track32.ps1` | `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\32-gpu-compute-acceleration\validate-track32.ps1 -SkipCargoTest` |
| WGSL ABM shader scaffold exists | complete | `src/shaders/abm_step.wgsl` | shader source inspected by file presence; naga compile pending backend dependency |
| WGSL DES shader scaffold exists | complete | `src/shaders/des_dispatch.wgsl` | shader source inspected by file presence; naga compile pending backend dependency |
| wgpu backend module is feature-gated | complete | `src/wgpu/backend.rs` | `cargo check --manifest-path crates/kairo-ecs-gpu/Cargo.toml --no-default-features` |
| CUDA backend module is feature-gated | complete | `src/backends/cuda_backend.rs` | `cargo check --manifest-path crates/kairo-ecs-gpu/Cargo.toml --no-default-features` |
| Feature-gated native backends report explicit not-configured errors | complete | `src/wgpu/backend.rs`, `src/backends/cuda_backend.rs` | `cargo check --manifest-path crates/kairo-ecs-gpu/Cargo.toml --features wgpu-backend,cuda-backend --tests` |
| CPU-vs-GPU ABM parity contract harness compiles | complete | `crates/kairo-ecs-gpu/tests/parity.rs`, CPU fallback contract | `cargo check --manifest-path crates/kairo-ecs-gpu/Cargo.toml --tests` |
| CPU-vs-GPU DES parity contract harness compiles | complete | `crates/kairo-ecs-gpu/tests/parity_des.rs`, CPU fallback contract | `cargo check --manifest-path crates/kairo-ecs-gpu/Cargo.toml --tests` |
| Kernel IR design doc exists | complete | `docs/gpu-compute/kernel-ir.md` | `rg -n "Kernel IR" docs/gpu-compute/kernel-ir.md` |
| Memory/dispatch contract doc exists | complete | `docs/gpu-compute/memory-contract.md` | `rg -n "GPU Memory and Dispatch Contract" docs/gpu-compute/memory-contract.md` |
| Backend selection rationale exists | complete | `docs/gpu-compute/backend-selection.md` | `rg -n "wgpu|CUDA" docs/gpu-compute/backend-selection.md` |
| Event ordering doc exists | complete | `docs/gpu-compute/event-ordering.md` | `rg -n "Event Ordering" docs/gpu-compute/event-ordering.md` |
| Hardware requirements matrix exists | complete | `docs/gpu-compute/hardware-requirements.md` | `rg -n "Backend" docs/gpu-compute/hardware-requirements.md` |
| Benchmark results file exists without unsupported speedup claims | complete | `docs/gpu-compute/benchmark-results.md` | `rg -n "not yet available|not publish speedup" docs/gpu-compute/benchmark-results.md` |
| GPU feature build with real backend dependencies | blocked | none yet | blocked until `wgpu`/`cudarc` dependencies are introduced and registry/network access is approved |
| Hardware parity on CUDA or wgpu | blocked | none yet | blocked until a GPU runner or local GPU validation environment is available |
| 10x speedup benchmark | blocked | none yet | blocked until Track 12 benchmark integration is available on GPU-capable runners |
## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, phase-closeout ledger evidence, cleaned commit/push evidence, and blocker recording. At actual closeout, run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.
