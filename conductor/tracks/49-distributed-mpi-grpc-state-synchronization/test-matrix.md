# Track49 Test Matrix

Current selected runtime: actual repository Python3.15.2 and Rust1.99.1.

| Gate | Command or retained execution | Acceptance |
|---|---|---|
| Native MPI API | Build native_mpi_smoke with native-mpi; run exact emitted executable under mpiexec -n 2 and -n 4, --test-threads=1 --nocapture | Real MPI API, bounds/rejection/subsequent valid exchange |
| Native optimistic MPI2/4 | `.venv/bin/python docs/distributed/qualify_track49.py --binary ABSOLUTE_BINARY --ranks 2 --output PRIVATE_ATTEMPT` and separately --ranks 4 | Reference plus before/after SQL SIGKILL, full-allocation rejoin and serial parity;6 cases |
| Real Mac/Colab gRPC | `.venv/bin/python docs/distributed/qualify_track49_grpc.py --binary ABSOLUTE_BINARY --ssh-args PRIVATE_JSON --remote-repo /content/kairos-track49-full/kairos --output PRIVATE_ATTEMPT --cases all` | Three full real two-host cases, typed migration/IPC and actual failures |
| PDES amendment | `cargo test --locked -p kairo-ecs-pdes --all-features` | 208 pass; corruption/proof/CAS/crash regressions |
| Tonic synchronization | `cargo test --locked -p kairo-ecs-grpc --test track49_sync --all-features` | 9 pass; independent wire/schema/socket oracles |
| Strict lint | `cargo clippy --locked -p kairo-ecs-pdes -p kairo-ecs-mpi -p kairo-ecs-grpc --all-targets --all-features -- -D warnings` | Focused receipts and full hosted lane |
| Full Rust quality | `just ci`, locked doctests and actual MPI2/4 API exchange in ci-core | Exact-source hosted result, distinct from native host proof |
| Evidence and phase | `node scripts/validation/validate-hpc-parity-evidence.mjs`; phase/DAG/artifact validators | Manifest/claim structure, not substitute runtime proof |
| Strict Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | After normal commit/push |

The controllers build and verify actual emitted executables, source maps and
checked profiles. Source/commands/cwd/toolchain/input/hardware/binary/log hashes
are retained in [qualification](native-runtime-qualification-20261103.md).
No runner retries/skips relax these gates. MPI rank loss requires allocation
termination; reopened stores stay standby until complete membership joins.
Default/legacy mpi multirank_smoke and grpc two_process_smoke remain TCP tests;
mpiexec process count alone is not actual MPI transport proof. No NAS/cross-host
MPI, cluster/Slurm/GPU/scaling or whole-LP migration acceptance is inferred.
