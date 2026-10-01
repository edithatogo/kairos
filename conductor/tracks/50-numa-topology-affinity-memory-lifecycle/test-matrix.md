# Track 50 Test Matrix

| Gate | Command | Required for |
|---|---|---|
| Core/state tests | `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-core -p kairo-ecs-state --features numa` | Implementation |
| FFI layout tests | `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-ffi --features numa` | Review |
| Miri safety | `cargo miri test -p kairo-ecs-core` | Done when available |
| Full workspace | `rustup run stable-x86_64-pc-windows-gnu cargo test --workspace --all-features` | Phase closeout |
| Phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Phase movement |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Closeout |

Strict closeout requires `RequireCleanWorkingTree` after each task commit.
