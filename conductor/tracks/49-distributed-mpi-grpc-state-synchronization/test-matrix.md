# Track 49 Test Matrix

| Gate | Command | Required for |
|---|---|---|
| MPI crate tests | `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-mpi --features mpi` | Implementation |
| gRPC crate tests | `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-grpc --features grpc` | Implementation |
| Real MPI launch | `mpiexec -n 4 cargo test -p kairo-ecs-mpi --features mpi --test multirank_smoke` | Done |
| Real gRPC launch | `cargo test -p kairo-ecs-grpc --features grpc --test two_process_smoke` | Done |
| Full workspace | `rustup run stable-x86_64-pc-windows-gnu cargo test --workspace --all-features` | Phase closeout |
| Phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Phase movement |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Closeout |

Strict closeout requires `RequireCleanWorkingTree` after each task commit.
