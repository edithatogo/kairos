# Test Matrix: Track 35 Distributed Simulation (MPI/gRPC)

## Proof tiers used in this track

- **Local compile checks**: dependency-free protocol contracts and placeholders.
- **Runtime execution checks**: two-node or fault-injection runs that require
  real MPI/gRPC backends and executable test binaries.

Until real transport runtimes are introduced, this track records runtime checks as
`blocked` with the concrete blocker.

| Check | Alpha | Beta | RC | 1.0 |
|---:|---:|---:|---:|---:|
| Track docs exist under `conductor/tracks/35-distributed-simulation-mpi-grpc/` | yes | yes | yes | yes |
| `crates/kairo-ecs-mpi/` dependency-free emulator compiles with `--features mpi --tests` | yes | yes | yes | yes |
| `crates/kairo-ecs-grpc/` dependency-free emulator compiles with `--features grpc --tests` | yes | yes | yes | yes |
| `MpiContractEnvelope` / `GrpcContractEnvelope` message-contract types compile and validate | yes | yes | yes | yes |
| Single-node build is unaffected when `mpi` and `grpc` features are disabled | yes | yes | yes | yes |
| Track 34 `PdesTransport` boundary is documented with ThreadChannel, MPI, and gRPC scaffold implementations | yes | yes | yes | yes |
| MPI protocol emulator compiles message round-trip and GVT reduction checks under `cargo check --tests` | yes | yes | yes | yes |
| gRPC protocol emulator compiles message round-trip and GVT reduction checks under `cargo check --tests` | yes | yes | yes | yes |
| MPI local protocol validators cover rank uniqueness, stable tags, migration envelope, and telemetry envelope | yes | yes | yes | yes |
| gRPC local protocol validators cover peer/config validation, migration envelope, telemetry envelope, and heartbeat failure classification | yes | yes | yes | yes |
| Dependency-free MPI local two-rank contract proof covers event exchange, migration validation, telemetry merge count, and GVT floor | no | yes | yes | yes |
| Dependency-free gRPC local two-node contract proof covers event exchange, migration validation, telemetry merge count, and non-leader failure classification | no | yes | yes | yes |
| Track 35 offline validator checks compile, placeholder-transport boundaries, and production-use caveats | yes | yes | yes | yes |
| Entity migration protocol is documented in `docs/distributed/entity-migration-protocol.md` | yes | yes | yes | yes |
| Distributed telemetry aggregation design is documented in `docs/distributed/telemetry-aggregation.md` | yes | yes | yes | yes |
| Deployment guide exists in `docs/distributed/deployment-guide.md` | no | yes | yes | yes |
| End-to-end 2-node MPI test passes — final state matches single-node PDES | blocked | yes | yes | yes |
| End-to-end 2-node gRPC test passes — final state matches single-node PDES | blocked | yes | yes | yes |
| Entity migration preserves all component state (byte-level comparison) | blocked | no | yes | yes |
| gRPC fault tolerance: non-leader worker failure does not crash simulation | blocked | no | yes | yes |
| gRPC fault tolerance: simulation produces valid final state after worker failure | blocked | no | yes | yes |
| Distributed telemetry aggregation produces Arrow batches matching single-node content | blocked | no | yes | yes |
| MPI event exchange latency overhead < 100us per message (excluding network) | no | no | no | yes |
| `distributed-state-parity` gate exists in `conductor/quality-gates.md` | no | yes | yes | yes |
| `entity-migration-integrity` gate exists in `conductor/quality-gates.md` | no | yes | yes | yes |
| `grpc-fault-tolerance` gate exists in `conductor/quality-gates.md` | no | yes | yes | yes |
| `distributed-telemetry-merge` gate exists in `conductor/quality-gates.md` | no | yes | yes | yes |
| All distributed gates pass on CI with `--features mpi,grpc` | blocked | no | yes | yes |
| MPI mode works with OpenMPI and MPICH on Linux | blocked | no | yes | yes |
| MPI mode works with MS-MPI on Windows | no | no | no | yes |
| Distributed mode does not block single-machine release | yes | yes | yes | yes |

## Focused Validation Commands

| Command | Result | Evidence |
|---|---|---|
| `pwsh -NoProfile -File conductor/tracks/35-distributed-simulation-mpi-grpc/validate-track35.ps1` | Local offline gate | Checks MPI/gRPC emulator compilation, local two-node contract proof docs, and verifies docs/code still label real transport networking as future work. |
| `pwsh -NoProfile -File conductor/tracks/35-distributed-simulation-mpi-grpc/validate-track35.ps1 -RunTests` | Optional runtime gate | Runs MPI/gRPC unit tests when the local linker/toolchain can execute Rust test binaries. |
| `cargo check --manifest-path crates/kairo-ecs-mpi/Cargo.toml --features mpi --tests` | Local contract gate | verifies transport trait signatures and compile-time protocol envelope validation |
| `cargo check --manifest-path crates/kairo-ecs-grpc/Cargo.toml --features grpc --tests` | Local contract gate | verifies peer/config envelopes, migration envelopes, telemetry envelopes, and heartbeat classifier |
## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, phase-closeout ledger evidence, cleaned commit/push evidence, and blocker recording. At actual closeout, run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.
