# Distributed Deployment Guide

## Overview

KairoECS provides two distinct distributed transport implementations under `#![forbid(unsafe_code)]`:

1. **MPI Multi-Rank Transport (`kairo-ecs-mpi`)**:
   - Implemented by `MpiProcessTransport` adhering to `PdesTransport`.
   - Supports peer-to-peer event routing, rank rendezvous, global barriers, and `all_reduce_min` GVT calculation across distinct OS processes.
   - Automatically detects rank and world size from standard MPI environments (`OMPI_COMM_WORLD_RANK`, `PMIX_RANK`, `MPI_LOCALRANKID`, `PMI_RANK`).
   - Verified both under direct process invocation and natively under `mpiexec -n 4`.

2. **gRPC Socket Transport (`kairo-ecs-grpc`)**:
   - Implemented by `GrpcSocketTransport` adhering to `PdesTransport`.
   - Communicates across real TCP socket listeners with framed binary streaming.
   - Supports `KairoOptimisticEnvelopeV1` Packet 2 & 3 wire format with deterministic preflight rejection oracles (R01–R15).
   - Preserves component generations during entity migration and aggregates telemetry batches.
   - Classifies worker heartbeats into `Healthy`, `Suspect`, and `Failed`.

---

## Launch and Validation Commands

### MPI Multi-Rank Transport

To run the 4-rank Open MPI integration smoke test:

```bash
# 1. Direct cargo test (spawns 4 worker processes if not under mpiexec)
cargo test -p kairo-ecs-mpi --features mpi --test multirank_smoke

# 2. Native execution under Open MPI across 4 real ranks
cargo test -p kairo-ecs-mpi --features mpi --test multirank_smoke --no-run
mpiexec -n 4 target/debug/deps/multirank_smoke-<hash>
```

### gRPC Multi-Process Transport

To run the two-process TCP socket smoke test:

```bash
cargo test -p kairo-ecs-grpc --features grpc --test two_process_smoke
```

---

## Unsupported Network and Scheduler Configurations

To ensure deterministic scheduling and avoid split-brain states:

1. **Unsupported Network Topologies**:
   - **Asymmetric Partitioning**: Partial network partitions where rank $A$ can reach $B$ but not $C$ while $B$ can reach $C$ are not automatically healed. Transports will error with `TransportError::UnknownLogicalProcess`.
   - **Dynamic IP Migration / NAT Rebinding**: Endpoints must retain stable loopback or cluster IP bindings for the lifetime of the simulation run.
   - **Unordered Non-FIFO Transports**: UDP or connectionless datagram sockets without ordering guarantees are unsupported; both transports rely on ordered byte streams.

2. **Unsupported Scheduler Configurations**:
   - **Dynamic Resizing / Elastic LP Reallocation**: The set of logical processes ($LP$) and their initial rank mappings must remain invariant during active event simulation. Dynamic rank joining or eviction requires pausing at a global barrier and checkpointing.
   - **Unsynchronized Overcommit**: Launching more ranks than available physical cores without CPU pinning may induce thread contention and skew GVT reduction latency.

---

## Track Handoff Boundaries

- **Track 54 (Cluster & Cloud Deployment)**:
  - Consumes `MpiProcessTransport` and `conductor/hpc-evidence/manifests/track49-live-hpc-template.json` for multi-node Slurm job scripts and container runtime orchestration.
- **Track 55 (Scaling Benchmarks & Parity Profiles)**:
  - Consumes distributed timing telemetry and Arrow batch schemas from `GrpcSocketTransport` and `kairo-ecs-pdes` for weak and strong scaling profiles.
