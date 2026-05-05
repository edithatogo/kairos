# Agent Contract: distributed-agent

## Track

Track 35: Distributed Simulation (MPI/gRPC)

## Owned paths

- `conductor/tracks/35-distributed-simulation-mpi-grpc/`
- `crates/kairo-ecs-mpi/`
- `crates/kairo-ecs-grpc/`
- `docs/distributed/`
- Track-specific artifacts named in `plan.md`

## Required handoff

- Summary of distributed architecture (MPI and gRPC transports, entity migration, fault tolerance).
- `LpTransport` trait specification with implementations for thread channels, MPI, and gRPC.
- Entity migration protocol specification with at-most-once semantics.
- Distributed telemetry aggregation design and merge strategy.
- Deployment guide for HPC (MPI) and cloud (gRPC) environments.
- Benchmark results: 2-node scaling versus single-node PDES.
- Fault tolerance test results for gRPC mode.
- Follow-up items for Track 34 (PDES transport abstraction) and Track 04 (distributed telemetry).

## Prohibited changes without ADR

- Modifying `crates/kairo-ecs-pdes/` scheduler algorithm — scheduling is owned by Track 34.
- Modifying `crates/kairo-ecs-core/` scheduler internals — owned by Track 01.
- Modifying the `LogicalProcess` trait — owned by Track 34.
- Modifying the Arrow telemetry schema — owned by Track 04.
- Changing the entity migration protocol after stabilization without backward compatibility.
- Introducing MPI or gRPC code paths that execute when their respective feature flags are disabled.

## Gate contract

### distributed-state-parity
- **Input**: 2-node distributed simulation output, single-node PDES simulation output for the same partitioned world.
- **Output**: Pass if final component state and entity graph are identical. Fail with divergent entities/components.
- **Blocking**: Yes for `mpi` and `grpc` feature flags. Not release-gating for single-node mode.

### entity-migration-integrity
- **Input**: Entity migration test output (pre-migration and post-migration component snapshots).
- **Output**: Pass if all component data is preserved after migration (byte-level comparison). Fail if any component diverges.
- **Blocking**: Yes for `mpi` and `grpc` feature flags. Not release-gating for single-node mode.

### grpc-fault-tolerance
- **Input**: Fault tolerance test run (kill non-leader worker mid-simulation).
- **Output**: Pass if simulation completes with remaining workers and produces valid final state. Fail if simulation crashes or hangs.
- **Blocking**: Yes for `grpc` feature flag only (not applicable to MPI mode). Not release-gating for single-node mode.

### distributed-telemetry-merge
- **Input**: Aggregated Arrow record batches from distributed run, single-node Arrow record batches.
- **Output**: Pass if aggregated batches contain the same data as single-node collection (content equality, ignoring ordering). Fail on data loss or corruption.
- **Blocking**: Yes for `mpi` and `grpc` feature flags. Not release-gating for single-node mode.
