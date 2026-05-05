# GPU Compute

This directory records the optional GPU acceleration design for KairoECS. The CPU scheduler remains the default execution path; GPU compute is an opt-in accelerator behind crate-level feature flags.

Current artifacts:

- `kernel-ir.md` defines the shared compute model used by native GPU and WebGPU work.
- `backend-selection.md` records the wgpu/CUDA backend split.
- `architecture.md` defines host-to-device buffer flow.
- `memory-contract.md` defines hardware-independent memory budget and dispatch shape checks.
- `event-ordering.md` defines deterministic and nondeterministic DES event behavior.
- `hardware-requirements.md` records the hardware matrix required before performance claims are accepted.
- `benchmark-results.md` is an explicit no-results evidence file until GPU hardware runs are available.

Validation commands:

```powershell
cargo check --manifest-path crates/kairo-ecs-gpu/Cargo.toml --features wgpu-backend,cuda-backend --tests
cargo test --manifest-path crates/kairo-ecs-gpu/Cargo.toml
cargo check --manifest-path crates/kairo-ecs-gpu/Cargo.toml --no-default-features
powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\32-gpu-compute-acceleration\validate-track32.ps1 -SkipCargoTest
```
