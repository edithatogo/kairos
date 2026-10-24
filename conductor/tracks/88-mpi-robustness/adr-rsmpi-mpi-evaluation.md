# ADR: Evaluation and Rejection of `rsmpi` Native MPI Dependency

**Status:** Decided — **REJECT (NO-GO)**  
**Date:** 2026-10-25  
**Track:** 88-mpi-robustness  
**Deciders:** `distributed-agent`, Principal Architecture Reviewer  

---

## Context

Track 88 was chartered as a conditional evaluation track to determine whether Kairos's distributed PDES message transport should adopt `rsmpi` (`mpi = "0.8.2"` crate) to interface with native Message Passing Interface (MPI) runtimes on HPC supercomputers.

The repository's [dependency-value-policy.md](../../dependency-value-policy.md) requires:
1. An active accepted production consumer.
2. Demonstrated robustness or maintainability gain with no unacceptable regression.
3. Strict enforcement of Stop Criteria defined in [spec.md](./spec.md).

## Decision

We **REJECT** the adoption of `rsmpi` / `mpi` and retain the native pure-Rust `#![forbid(unsafe_code)]` architecture:
1. `crates/kairo-ecs-mpi` remains a pure-Rust multi-rank contract emulator.
2. `crates/kairo-ecs-grpc` (`TonicTransport`) remains the production distributed multi-node transport, providing HTTP/2 multiplexing, Jefferson Time Warp rollback support, and GVT consensus.

## Rationale

1. **Stop Criterion Triggered:** Track 88 [spec.md](./spec.md) defines an explicit stop condition:
   *"Stop criterion: No installation for socket-only tests or absent HPC consumer; no CPU/socket fixture promoted to MPI acceptance."*
   There is currently no active HPC supercomputing deployment or MPI consumer in Kairos or CareOps-Sim.
2. **Toolchain & CI Brittleness:** Native MPI requires platform-specific C runtimes (`libopenmpi-dev`, `mpich`, or MS-MPI). Adding `rsmpi` would break zero-dependency builds on developer workstations and standard CI runners (`ubuntu-latest`, `macos-15`, `windows-latest`).
3. **Safety & Concurrency Invariant Preservation:** `crates/kairo-ecs-mpi` maintains `#![forbid(unsafe_code)]`. `rsmpi` relies on raw C FFI pointers, increasing the risk of memory corruption and hard segmentation faults.
4. **Pure-Rust Distributed Infrastructure Already Mature:** The successful qualification of Tonic 0.14 (Track 83) and Tokio (Track 82) delivers high-throughput, cross-platform distributed discrete-event simulation with full protobuf typing and zero external C services.

## Requalification Triggers

This decision may only be revisited if:
1. An explicit supercomputing research project requires native InfiniBand / Slurm / OpenMPI collectives.
2. A dedicated isolated crate feature (e.g. `kairo-ecs-mpi-native`) is introduced without polluting the core cross-platform workspace.
