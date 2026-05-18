# Distributed Telemetry Aggregation

Telemetry aggregation is transport-specific but schema-preserving.

MPI mode:

- each rank serializes Arrow IPC buffers from its local LPs;
- each telemetry envelope includes `source_lp`, `tick_start`, and `tick_end`;
- root gathers buffers with `MPI_Gather`;
- root validates tick ranges and non-empty payloads before concatenating record
  batches because LP partitions are disjoint.

gRPC mode:

- workers stream telemetry batches to a coordinator service;
- each streamed batch includes `source_lp`, `tick_start`, `tick_end`, and Arrow
  IPC payload bytes;
- the coordinator validates schema compatibility, tick ranges, and non-empty
  payloads;
- batches are concatenated and emitted as a consolidated stream.

Ordering is not a correctness condition. Content parity with single-node
collection is the acceptance criterion.

Current local smoke coverage:

- `MpiTelemetryBatch::validate` and `GrpcTelemetryBatch::validate` reject empty
  payloads and non-monotonic tick ranges.
- These validators intentionally do not inspect Arrow schemas yet; Track 04 owns
  schema compatibility once the runtime Arrow dependency is wired.
- `local_two_rank_contract_proof` and `local_two_node_contract_proof` each merge
  two local telemetry envelope counts after validating non-empty payloads and
  monotonic tick ranges. This proves the local merge contract shape without
  claiming multi-process MPI gather or gRPC stream aggregation.

## Local contract evidence

- MPI telemetry envelope contract: `MpiTelemetryBatch` in
  `crates/kairo-ecs-mpi/src/lib.rs`.
- gRPC telemetry envelope contract: `GrpcTelemetryBatch` in
  `crates/kairo-ecs-grpc/src/lib.rs`.
- Service-level telemetry stream contract in protobuf:
  `TelemetryBatch` in `crates/kairo-ecs-grpc/proto/simulation.proto`.
- Both contracts are compile-time checks only in Track 35 currently; there is no
  multi-process aggregation runtime path yet.

Validation command for current Track 35 scaffolding:

```powershell
cargo test --manifest-path crates/kairo-ecs-mpi/Cargo.toml --features mpi telemetry_batch
cargo test --manifest-path crates/kairo-ecs-grpc/Cargo.toml --features grpc telemetry_batch
```

Expected output:

```text
test result: ok. ...
```
