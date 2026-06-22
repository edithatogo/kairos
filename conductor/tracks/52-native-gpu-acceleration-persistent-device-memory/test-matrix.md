# Track 52 Test Matrix

| Gate | Command | Required for |
|---|---|---|
| Backend initialization contract | `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-gpu --features wgpu-backend,cuda-backend --test backend_initialization` | Implementation |
| GPU crate tests | `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-gpu --features wgpu-backend,cuda-backend` | Implementation |
| WebGPU crate tests | `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-webgpu --features webgpu` | Review |
| Real wgpu dispatch | `cargo test -p kairo-ecs-gpu --features wgpu-backend --test real_wgpu_device` | Done |
| Real CUDA dispatch | `cargo test -p kairo-ecs-gpu --features cuda-backend --test real_cuda_device` | Done |
| Full workspace | `rustup run stable-x86_64-pc-windows-gnu cargo test --workspace --all-features` | Phase closeout |
| Phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Phase movement |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Closeout |

Strict closeout requires `RequireCleanWorkingTree` after each task commit.
