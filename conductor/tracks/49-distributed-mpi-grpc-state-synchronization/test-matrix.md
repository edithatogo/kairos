# Track 49 Test Matrix

| Gate | Command | Required for |
|---|---|---|
| MPI crate tests | `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-mpi --features mpi` | Implementation |
| gRPC crate tests | `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-grpc --features grpc` | Implementation |
| Real MPI launch | `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-mpi --features mpi --test multirank_smoke --no-run && mpiexec -n 4 rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-mpi --features mpi --test multirank_smoke` | Done |
| Real gRPC launch | `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-grpc --features grpc --test two_process_smoke` | Done |
| Full workspace | `rustup run stable-x86_64-pc-windows-gnu cargo test --workspace --all-features` | Phase closeout |
| Phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Phase movement |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Closeout |

Strict closeout requires `RequireCleanWorkingTree` after each task commit.

## Native MPI preview (2 November 2026)

Use Rust 1.99.1 with an actual MPI development installation and libclang. Build
`cargo test --locked -p kairo-ecs-mpi --features native-mpi --test native_mpi_smoke --no-run`;
resolve the exact emitted test executable, then run it with `mpiexec -n 2` and
`mpiexec -n 4`, both with `--test-threads=1 --nocapture`. Require every rank to
pass ring/null exchange, u128 minimum, invalid/oversize collective rejection and
subsequent valid exchange. Also run `cargo test --locked -p kairo-ecs-mpi --all-features`
and strict all-target/all-feature Clippy. A single-rank ordinary Cargo test is not
the 2/4-rank gate. This does not replace CAP-48.1/49.2 two-host recovery/parity.
