# Track 52 Test Matrix

| Gate | Command | Required for |
|---|---|---|
| Backend initialization contract | `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-gpu --features wgpu-backend,cuda-backend --test backend_initialization` | Implementation |
| GPU crate tests | `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-gpu --features wgpu-backend,cuda-backend` | Implementation |
| WebGPU crate tests | `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-webgpu --features webgpu` | Review |
| Local CPU parity evidence manifest | `node scripts/validation/validate-hpc-parity-evidence.mjs` | Phase 4 scaffold evidence |
| Live-template blocker guard | `node scripts/validation/validate-hpc-live-template-blockers.mjs` | Review and Done |
| Free compute route boundary | `node scripts/validation/validate-free-compute-routes.mjs` | Review |
| Real Metal dispatch | On Apple Metal with Rust 1.99.1, `cargo test --locked -p kairo-ecs-gpu --features wgpu-backend --test maturity_device_model --test native_metal_des` | Local native model qualification |
| Real CUDA dispatch | On the Windows CUDA host with Rust 1.99.1, `cargo test --locked -p kairo-ecs-gpu --features cuda-backend --test cuda_native_model -- --ignored --nocapture` | Local native model qualification |
| CUDA bridge recovery | `cargo test --locked -p kairo-ecs-cuda --features cuda-12000 --lib -- --ignored --nocapture` | Native particle recovery |
| Matched CUDA timing | `cargo test --locked -p kairo-ecs-gpu --features cuda-backend --release --test cuda_mixed_profile -- --ignored --nocapture` | Local timing and replay evidence |
| CUDA device trace | [Nsight capture and copy counts](../../execution/scientific/s11-cuda-completion.md#device-kernel-and-copy-trace) | Local kernel and transfer evidence |
| Live wgpu hardware manifest | Completed `conductor/hpc-evidence/manifests/track52-live-gpu-hardware-template.json` copy promoted to `evidence_class: live-hpc`, with raw artifact checksum, adapter/device metadata, and `waiver.status: none`; validate with `node scripts/validation/validate-hpc-parity-evidence.mjs` | Done |
| Live CUDA hardware manifest | Completed `conductor/hpc-evidence/manifests/track52-live-gpu-hardware-template.json` copy promoted to `evidence_class: live-hpc`, with raw artifact checksum, CUDA device/runtime metadata, and `waiver.status: none`; validate with `node scripts/validation/validate-hpc-parity-evidence.mjs` | Done |
| Full workspace | `rustup run stable-x86_64-pc-windows-gnu cargo test --workspace --all-features` | Phase closeout |
| Phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Phase movement |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Closeout |

Strict closeout requires `RequireCleanWorkingTree` after each task commit.
CPU-only parity, unavailable-device reports, and local persistent-memory
contracts remain scaffold gates. They cannot satisfy the real wgpu/CUDA
hardware gates without live manifests and raw artifacts.
