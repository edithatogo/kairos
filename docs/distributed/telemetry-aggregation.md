# Distributed Telemetry Aggregation

Telemetry aggregation is transport-specific but schema-preserving.

MPI mode:

- each rank serializes Arrow IPC buffers from its local LPs;
- root gathers buffers with `MPI_Gather`;
- root concatenates record batches because LP partitions are disjoint.

gRPC mode:

- workers stream telemetry batches to a coordinator service;
- the coordinator validates schema compatibility;
- batches are concatenated and emitted as a consolidated stream.

Ordering is not a correctness condition. Content parity with single-node
collection is the acceptance criterion.

Validation command for current Track 35 scaffolding:

```powershell
cargo test --manifest-path crates/kairo-ecs-grpc/Cargo.toml --features grpc
```
