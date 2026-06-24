# Track 48 Test Matrix

| Gate | Command | Required for |
|---|---|---|
| Time Warp tests | `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-pdes --features time-warp` | Implementation |
| Conservative regression | `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-pdes --features pdes` | Review |
| Benchmark compile | `rustup run stable-x86_64-pc-windows-gnu cargo check --benches -p kairo-ecs-pdes --features time-warp` | Review |
| Local Time Warp evidence manifest | `node scripts/validation/validate-hpc-parity-evidence.mjs` | Evidence boundary |
| Full workspace | `rustup run stable-x86_64-pc-windows-gnu cargo test --workspace --all-features` | Phase closeout |
| Phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Phase movement |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Closeout |

Strict closeout requires `RequireCleanWorkingTree` after each task commit.
The local evidence manifest gate validates manifest shape and claim boundaries
only. It is not a distributed optimistic rollback proof and must not be used to
advance Track 48 to `Done` without live distributed rollback artifacts.
