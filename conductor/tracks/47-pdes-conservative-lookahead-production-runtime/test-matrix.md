# Track 47 Test Matrix

| Gate | Command | Required for |
|---|---|---|
| PDES crate tests | `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-pdes --features pdes` | Implementation |
| Track 47 integration evidence | `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-pdes --features pdes --test track47_conservative_runtime` | Implementation |
| Sequential parity | `pwsh -NoProfile -File conductor/tracks/34-pdes-parallel-execution/validate-track34.ps1 -RunTests` | Review |
| Bench target check | `rustup run stable-x86_64-pc-windows-gnu cargo check --benches -p kairo-ecs-pdes --features pdes` | Review |
| Full workspace | `rustup run stable-x86_64-pc-windows-gnu cargo test --workspace --all-features` | Phase closeout |
| Clippy | `rustup run stable-x86_64-pc-windows-gnu cargo clippy --workspace --all-targets --all-features -- -D warnings` | Phase closeout |
| Phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Phase movement |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Closeout |

Strict closeout requires `RequireCleanWorkingTree` after each task commit.
