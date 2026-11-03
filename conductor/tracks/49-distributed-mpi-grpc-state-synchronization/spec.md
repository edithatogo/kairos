# Track 49: Distributed MPI/gRPC State Synchronization

## Purpose

Replace placeholder distributed transport surfaces with real MPI and gRPC
state synchronization, multi-rank event exchange, entity migration, failure
classification, telemetry merge, and rollback-aware message handling.

## Maturity

Done for the bounded experimental native scope: actual MPI2/4 and Mac/Colab
gRPC failure/restart qualification accepted3November2026. See native-runtime-qualification-20261103.md. NAS/cluster/scaling/release behavior remains unqualified.

## Inputs

- `crates/kairo-ecs-mpi/`, `crates/kairo-ecs-grpc/`, and `docs/distributed/`
  from Track 35.
- Production LP contract from Track 47.
- Anti-message contract from Track 48 when optimistic mode is enabled.
- Evidence manifest from Track 46.

## Outputs

- Real `rsmpi` signed-record transport behind `native-mpi`; legacy `mpi` TCP remains distinct.
- Real `tonic`/`prost` service/client; opt-in external-time-warp and telemetry-ipc qualify signed runtime and standard IPC.
- Multi-rank and two-node runtime tests.
- Entity migration serialization and restore tests.
- Distributed telemetry merge and failure evidence.

## Owned paths

- `crates/kairo-ecs-mpi/`
- `crates/kairo-ecs-grpc/`
- `docs/distributed/`
- `conductor/tracks/49-distributed-mpi-grpc-state-synchronization/`

## Blocked paths

- PDES runtime semantics owned by Tracks 47 and 48.
- Arrow batch writer internals owned by Track 51.
- Slurm/cloud launch infrastructure owned by Track 54.

## Dependencies

Tracks 35, 47, and 48.

## Parallel-safe tracks

Track 54 may draft Slurm/container launch wrappers after this track defines
the MPI and gRPC command contracts.

## Acceptance criteria

- MPI tests run across at least 2 and 4 real ranks.
- gRPC tests run across at least two OS processes with real sockets.
- Distributed final state matches single-process reference scenarios.
- Entity migration preserves component generation and pending event metadata.
- Failure tests classify transient, permanent, and partition-like failures.

## Quality gates

- `real-mpi-multirank`
- `real-grpc-node-smoke`
- `distributed-state-parity`
- `entity-migration-integrity`
- `distributed-telemetry-merge`
- `hpc-evidence-manifest`
- `phase-closeout-check`

## Release implications

This track is release-critical for distributed simulation claims. Placeholder
transport behavior must remain explicitly labelled until this track closes.
