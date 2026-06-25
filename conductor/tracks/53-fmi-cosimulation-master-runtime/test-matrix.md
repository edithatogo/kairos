# Track 53 Test Matrix

| Gate | Command | Required for |
|---|---|---|
| FMI runtime tests | `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-fmi --features fmi-runtime,fmi2,fmi3` | Implementation |
| FMU archive tests | `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-fmi --test fmu_archive --features fmi-runtime` | Review |
| OpenModelica roundtrip | `omc --version && cargo test -p kairo-ecs-fmi --test openmodelica_roundtrip --features fmi-runtime` | Done |
| Full workspace | `rustup run stable-x86_64-pc-windows-gnu cargo test --workspace --all-features` | Phase closeout |
| Phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Phase movement |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Closeout |

Strict closeout requires `RequireCleanWorkingTree` after each task commit.
