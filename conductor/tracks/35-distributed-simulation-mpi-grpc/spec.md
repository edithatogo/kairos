# Track 35: Distributed Simulation (MPI/gRPC)

## Purpose

Scale KairoECS simulation beyond a single machine. MPI transport for HPC clusters (low-latency, RDMA-capable), gRPC transport for cloud deployments (tolerant of latency, works across subnets). Entity migration protocol, distributed Arrow telemetry aggregation, and fault tolerance for gRPC mode.

## Why this track exists

Track 34 enables parallel execution within a single machine. For simulations that exceed single-node memory (large agent populations, high-fidelity environments) or require compute beyond one machine, the LP model must extend across networked nodes. This track builds the MPI and gRPC communication backends, entity migration, and fault tolerance so that partitioned worlds can span clusters or cloud instances.

## Primary subagent

`distributed-agent`

## Parallelization model

Depends on Track 34 (PDES — shares the `LogicalProcess` trait and event exchange model) and Track 04 (Arrow telemetry schema). Builds on the LP abstraction: Track 34 LPs communicate via in-process channels; Track 35 replaces the channel layer with MPI or gRPC. Does not modify the PDES scheduler algorithm — only the transport. Entity migration and distributed telemetry are new capabilities not present in Track 34.

## Inputs

- `crates/kairo-ecs-pdes/` — `LogicalProcess` trait, event exchange protocol, GVT algorithm from Track 34.
- `docs/pdes/` — LP model, event exchange protocol documentation from Track 34.
- `crates/kairo-ecs-arrow/` — Arrow schema and telemetry framework from Track 04.
- `benches/` — benchmark harness from Track 12.
- `benches/pdes/` — PDES benchmark suite from Track 34 (scaling baseline).

## Outputs

- `crates/kairo-ecs-mpi/` — MPI communication layer using `rsmpi` crate:
  - MPI-based event exchange implementing Track 34's exchange protocol.
  - MPI-based GVT synchronization (all-reduce for minimum timestamp).
  - MPI bootstrap: rank assignment, LP-to-rank mapping.
- `crates/kairo-ecs-grpc/` — gRPC communication layer:
  - Protobuf service definition for event exchange, entity migration, and telemetry aggregation.
  - gRPC-based event exchange with configurable timeouts and retry.
  - gRPC-based GVT synchronization.
  - Fault tolerance: worker failure detection, reconnection, graceful degradation.
- `docs/distributed/` — design documentation:
  - Architecture overview: MPI vs. gRPC mode selection.
  - Entity migration protocol specification.
  - Distributed telemetry aggregation design.
  - Deployment guide (HPC cluster vs. cloud).
- Entity migration protocol:
  - Serialize/deserialize entity + component state for transfer between LPs on different nodes.
  - Ownership transfer handshake (source LP, destination LP, coordinator acknowledgment).
  - Migration consistency guarantees (at-most-once delivery, idempotent apply).
- Distributed Arrow telemetry aggregation:
  - Per-node telemetry collection using Track 04 Arrow schema.
  - Aggregation protocol: gRPC streaming or MPI gather.
  - Merge strategy for Arrow record batches across nodes.

## Owned paths

- `crates/kairo-ecs-mpi/`
- `crates/kairo-ecs-grpc/`
- `docs/distributed/`
- `conductor/tracks/35-distributed-simulation-mpi-grpc/`

## Blocked paths

- `crates/kairo-ecs-pdes/` — owned by Track 34 (LP trait, scheduler algorithm).
- `crates/kairo-ecs-core/` — owned by Track 01 (scheduler internals).
- `crates/kairo-ecs-arrow/` — owned by Track 04 (Arrow schema and telemetry).
- `docs/pdes/` — owned by Track 34 (LP and PDES documentation).

## Acceptance criteria

- 2-node simulation produces identical final state to single-node PDES for same partitioned world.
- Entity migration preserves all component state (verified by component-wise comparison after migration).
- MPI mode: event exchange latency overhead < 100us per message (excluding network latency) on InfiniBand.
- gRPC mode: simulation continues with degraded throughput when one worker fails (non-leader).
- Distributed telemetry produces Arrow record batches identical to single-node collection (modulo ordering).
- Both MPI and gRPC modes gated behind feature flags `mpi` and `grpc`.

## Release implications

Non-blocking for single-machine release. Distributed mode gated behind feature flags `mpi` and `grpc`. MPI support requires system `libmpi` — not bundled. gRPC support is self-contained via `tonic`/`prost`. Distributed mode is an opt-in capability for advanced users with cluster or cloud infrastructure.

## Non-goals

- Automatic cluster provisioning or orchestration (Kubernetes operator, Slurm integration).
- Cross-datacenter WAN optimization.
- Byzantine fault tolerance or consensus protocols.
- Real-time migration (migration while LP is processing events).
- Replacing the PDES scheduler algorithm — Track 34 owns scheduling; this track owns transport.
