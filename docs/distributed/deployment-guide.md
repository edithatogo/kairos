# Distributed Deployment Guide

MPI mode targets HPC clusters:

- install OpenMPI, MPICH, or MS-MPI before enabling the real MPI backend;
- map one or more LPs to each rank;
- run with a scheduler such as Slurm once integration tests exist.

gRPC mode targets cloud or mixed-network deployments:

- run one worker service per node;
- configure peer endpoints or service discovery;
- use TLS and explicit request timeouts before production use;
- coordinate heartbeats through LP 0 or a dedicated coordinator service.

Current status: crates compile as transport scaffolds only. Real MPI and gRPC
runtime backends are pending later Track 35 tasks.

Validation commands:

```powershell
cargo test --manifest-path crates/kairo-ecs-mpi/Cargo.toml --features mpi
cargo test --manifest-path crates/kairo-ecs-grpc/Cargo.toml --features grpc
```
