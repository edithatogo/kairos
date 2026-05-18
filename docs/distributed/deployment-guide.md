# Distributed Deployment Guide

MPI mode targets HPC clusters:

- install OpenMPI, MPICH, or MS-MPI before enabling the real MPI backend;
- map one or more LPs to each rank;
- set the `mpi` feature and add a real transport runtime backend when available;
- run with a scheduler such as Slurm once integration tests exist.

gRPC mode targets cloud or mixed-network deployments:

- run one worker service per node;
- configure peer endpoints or service discovery;
- use TLS and explicit request timeouts before production use;
- coordinate heartbeats through LP 0 or a dedicated coordinator service.
- classify workers as healthy, suspect, or failed from heartbeat age before removing
  them from the active worker set.

## Feature and hardware blockers (current slice)

- `feature = "mpi"` is a compile gate for contract surfaces only; local builds do
  not require MPI headers/libraries.
- Real `rsmpi` runtime wiring requires:
  - system MPI (`OpenMPI`, `MPICH`, or `MS-MPI`) on each worker;
  - rank/LP launch integration in the scheduler launcher;
  - Linux/cluster profile validation.
- `feature = "grpc"` is a compile gate for contract surfaces only; local builds do
  not require `tonic`, `prost`, or certificate material.
- Real gRPC runtime wiring requires:
  - generated protobuf build outputs,
  - endpoint discovery/registration,
  - mTLS or TLS endpoint security,
  - long-running coordinator service behavior.

## Local proof commands (no runtime transport)

```powershell
pwsh -NoProfile -File conductor/tracks/35-distributed-simulation-mpi-grpc/validate-track35.ps1
```

Expected output:

```text
Track 35 validator passed.
```

Optional runtime smoke commands (blocked until host/runtime dependencies are present):

```powershell
# placeholders for future runtime integration checks
mpirun -np 2 cargo run --manifest-path ...
grpcurl describe kairo.ecs.simulation.v1.SimulationTransport
```

Expected future output once real runtime backends exist:

```text
MPI ranks complete the distributed smoke run without timeout.
grpcurl lists kairo.ecs.simulation.v1.SimulationTransport methods.
```

## Current status in Track 35 scope

Current status: crates compile as transport scaffolds only. Real MPI and gRPC
runtime backends are pending later Track 35 tasks.

Local protocol validators are available now:

- MPI: rank/LP assignment uniqueness, stable message tags, migration envelope
  shape, and telemetry envelope shape.
- gRPC: timeout relationships, peer endpoint shape, duplicate/self-peer
  detection, migration envelope shape, telemetry envelope shape, and heartbeat
  failure classification.
- Local two-node contract proof helpers are available now:
  `local_two_rank_contract_proof` for MPI and `local_two_node_contract_proof`
  for gRPC. They exercise local event exchange, migration validation, telemetry
  merge counts, and non-leader failure classification without claiming real MPI
  or real gRPC runtime coverage.

Validation commands:

```powershell
pwsh -NoProfile -File conductor/tracks/35-distributed-simulation-mpi-grpc/validate-track35.ps1 -RunTests
```

Expected output when the GNU Rust runtime toolchain is available:

```text
test result: ok. ...
Track 35 validator passed.
```
