# Track 52 Test Matrix

| Gate | Command | Required for |
|---|---|---|
| Backend initialization contract | `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-gpu --features wgpu-backend,cuda-backend --test backend_initialization` | Implementation |
| GPU crate tests | `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-gpu --features wgpu-backend,cuda-backend` | Implementation |
| WebGPU crate tests | `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-webgpu --features webgpu` | Review |
| Local CPU parity evidence manifest | `node scripts/validation/validate-hpc-parity-evidence.mjs` | Phase 4 scaffold evidence |
| Live-template blocker guard | `node scripts/validation/validate-hpc-live-template-blockers.mjs` | Review and Done |
| Free compute route boundary | `node scripts/validation/validate-free-compute-routes.mjs` | Review |
| Real wgpu dispatch | `cargo test -p kairo-ecs-gpu --features wgpu-backend --test real_wgpu_device` | Done |
| Real CUDA dispatch | `cargo test -p kairo-ecs-gpu --features cuda-backend --test real_cuda_device` | Done |
| Live wgpu hardware manifest | Completed `conductor/hpc-evidence/manifests/track52-live-gpu-hardware-template.json` copy promoted to `evidence_class: live-hpc`, with raw artifact checksum, adapter/device metadata, and `waiver.status: none`; validate with `node scripts/validation/validate-hpc-parity-evidence.mjs` | Done |
| Live CUDA hardware manifest | Completed `conductor/hpc-evidence/manifests/track52-live-gpu-hardware-template.json` copy promoted to `evidence_class: live-hpc`, with raw artifact checksum, CUDA device/runtime metadata, and `waiver.status: none`; validate with `node scripts/validation/validate-hpc-parity-evidence.mjs` | Done |
| Full workspace | `rustup run stable-x86_64-pc-windows-gnu cargo test --workspace --all-features` | Phase closeout |
| Phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Phase movement |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Closeout |

Strict closeout requires `RequireCleanWorkingTree` after each task commit.
CPU-only parity, unavailable-device reports, and local persistent-memory
contracts remain scaffold gates. They cannot satisfy the real wgpu/CUDA
hardware gates without live manifests and raw artifacts.
