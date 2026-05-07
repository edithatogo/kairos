# Test Matrix: Track 35 Distributed Simulation (MPI/gRPC)

| Check | Alpha | Beta | RC | 1.0 |
|---:|---:|---:|---:|---:|
| Track docs exist under `conductor/tracks/35-distributed-simulation-mpi-grpc/` | yes | yes | yes | yes |
| `crates/kairo-ecs-mpi/` dependency-free emulator compiles with `--features mpi --tests` | yes | yes | yes | yes |
| `crates/kairo-ecs-grpc/` dependency-free emulator compiles with `--features grpc --tests` | yes | yes | yes | yes |
| Single-node build is unaffected when `mpi` and `grpc` features are disabled | yes | yes | yes | yes |
| Track 34 `PdesTransport` boundary is documented with ThreadChannel, MPI, and gRPC scaffold implementations | yes | yes | yes | yes |
| MPI protocol emulator compiles message round-trip and GVT reduction checks under `cargo check --tests` | yes | yes | yes | yes |
| gRPC protocol emulator compiles message round-trip and GVT reduction checks under `cargo check --tests` | yes | yes | yes | yes |
| MPI local protocol validators cover rank uniqueness, stable tags, migration envelope, and telemetry envelope | yes | yes | yes | yes |
| gRPC local protocol validators cover peer/config validation, migration envelope, telemetry envelope, and heartbeat failure classification | yes | yes | yes | yes |
| Track 35 offline validator checks compile, placeholder-transport boundaries, and production-use caveats | yes | yes | yes | yes |
| Entity migration protocol is documented in `docs/distributed/entity-migration-protocol.md` | yes | yes | yes | yes |
| Distributed telemetry aggregation design is documented in `docs/distributed/telemetry-aggregation.md` | yes | yes | yes | yes |
| Deployment guide exists in `docs/distributed/deployment-guide.md` | no | yes | yes | yes |
| End-to-end 2-node MPI test passes — final state matches single-node PDES | no | yes | yes | yes |
| End-to-end 2-node gRPC test passes — final state matches single-node PDES | no | yes | yes | yes |
| Entity migration preserves all component state (byte-level comparison) | no | no | yes | yes |
| gRPC fault tolerance: non-leader worker failure does not crash simulation | no | no | yes | yes |
| gRPC fault tolerance: simulation produces valid final state after worker failure | no | no | yes | yes |
| Distributed telemetry aggregation produces Arrow batches matching single-node content | no | no | yes | yes |
| MPI event exchange latency overhead < 100us per message (excluding network) | no | no | no | yes |
| `distributed-state-parity` gate exists in `conductor/quality-gates.md` | no | yes | yes | yes |
| `entity-migration-integrity` gate exists in `conductor/quality-gates.md` | no | yes | yes | yes |
| `grpc-fault-tolerance` gate exists in `conductor/quality-gates.md` | no | yes | yes | yes |
| `distributed-telemetry-merge` gate exists in `conductor/quality-gates.md` | no | yes | yes | yes |
| All distributed gates pass on CI with `--features mpi,grpc` | no | no | yes | yes |
| MPI mode works with OpenMPI and MPICH on Linux | no | no | yes | yes |
| MPI mode works with MS-MPI on Windows | no | no | no | yes |
| Distributed mode does not block single-machine release | yes | yes | yes | yes |

## Focused Validation Commands

| Command | Result | Evidence |
|---|---|---|
| `pwsh -NoProfile -File conductor/tracks/35-distributed-simulation-mpi-grpc/validate-track35.ps1` | Local offline gate | Checks MPI/gRPC emulator compilation and verifies docs/code still label real transport networking as future work. |
| `pwsh -NoProfile -File conductor/tracks/35-distributed-simulation-mpi-grpc/validate-track35.ps1 -RunTests` | Optional runtime gate | Runs MPI/gRPC unit tests when the local linker/toolchain can execute Rust test binaries. |
## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, phase-closeout ledger evidence, cleaned commit/push evidence, and blocker recording. At actual closeout, run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.