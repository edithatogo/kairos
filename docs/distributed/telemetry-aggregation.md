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

Validation command for current Track 35 scaffolding:

```powershell
cargo test --manifest-path crates/kairo-ecs-grpc/Cargo.toml --features grpc
```
