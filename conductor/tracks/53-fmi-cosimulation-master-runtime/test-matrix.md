# Track 53 Test Matrix

| Gate | Command | Required for |
|---|---|---|
| FMI runtime tests | `cargo test --locked -p kairo-ecs-fmi --all-features --lib --tests` | Implementation |
| FMU archive tests | `cargo test --locked -p kairo-ecs-fmi --features fmi2 --test fmu_archive_oracles` | Review |
| Native vendor archive and package | `KAIRO_REFERENCE_FMUS_DIR=/absolute/path/to/verified/reference cargo test --locked -p kairo-ecs-fmi --features fmi2,fmi3 --test maturity_external_fmu bouncing_ball -- --ignored` | FMI 2 and 3 co-simulation |
| OpenModelica FMI 2 time event | `KAIRO_OPENMODELICA_FMU=/absolute/path/to/KairosDecay.fmu cargo test --locked -p kairo-ecs-fmi --features fmi2 --test maturity_external_fmu openmodelica -- --ignored` | Independent FMI 2 case |
| Full workspace | `rustup run stable-x86_64-pc-windows-gnu cargo test --workspace --all-features` | Phase closeout |
| Phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Phase movement |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Closeout |

Strict closeout requires `RequireCleanWorkingTree` after each task commit.
