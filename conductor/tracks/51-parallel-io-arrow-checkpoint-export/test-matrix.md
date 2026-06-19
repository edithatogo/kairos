# Track 51 Test Matrix

| Gate | Command | Required for |
|---|---|---|
| Arrow tests | `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-arrow --features parallel-io` | Implementation |
| HDF5 roundtrip | `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-arrow --features hdf5` | Done |
| ADIOS2 roundtrip | `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-arrow --features adios2` | Done |
| Restart parity | `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-arrow --test checkpoint_restart` | Review |
| Full workspace | `rustup run stable-x86_64-pc-windows-gnu cargo test --workspace --all-features` | Phase closeout |
| Phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Phase movement |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Closeout |

Strict closeout requires `RequireCleanWorkingTree` after each task commit.
