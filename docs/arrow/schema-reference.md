# Arrow Schema Reference

KairoECS uses Apache Arrow for schema-stable telemetry interchange across language boundaries.

## Schema versioning

Current schema version: `1`

Schema version is carried in the stream name: `kairo_ecs.<stream>.v<version>`

### Compatibility rules
1. Additive nullable fields are compatible within the same major schema version.
2. Field removal or type change requires a new major schema version.
3. Every binding MUST include an Arrow roundtrip test.

## Streams

| Stream | Purpose | Status |
|---|---|---|
| `kairo_ecs.event_log.v1` | Per-event dispatch log | Defined |
| `kairo_ecs.metric_sample.v1` | Periodic metric snapshots | Planned |
| `kairo_ecs.entity_snapshot.v1` | Entity/component state snapshot | Planned |
| `kairo_ecs.resource_snapshot.v1` | Resource/queue state snapshot | Planned |
| `kairo_ecs.conformance_result.v1` | Conformance test output | Planned |

## event_log_v1

### Schema fields

| Field | Arrow Type | Nullable | Description |
|---|---|---|---|
| `run_id` | Utf8 | No | Deterministic run/session identifier |
| `event_id` | FixedSizeBinary(12) | No | Serialized EventId (u64 index + u32 generation) |
| `entity_id` | FixedSizeBinary(12) | Yes | Serialized EntityId, null for system events |
| `time_ticks` | FixedSizeBinary(16) | No | u128 simulation time ticks |
| `time_scale` | Utf8 | No | Time unit (default: "ticks") |
| `priority` | Int32 | No | Scheduler priority |
| `sequence` | UInt64 | No | Monotonic insertion sequence |
| `event_kind` | Utf8 | No | Event classification string |
| `status` | Utf8 | No | dispatch status: dispatched, cancelled, skipped, error |
| `payload_ref` | Utf8 | Yes | External payload reference |

### Event status values

| Value | Meaning |
|---|---|
| `dispatched` | Event was executed |
| `cancelled` | Event was cancelled before dispatch |
| `skipped` | Event was skipped (e.g., entity despawned) |
| `error` | Event dispatch failed |

### Roundtrip test

Every binding must:
1. Create an event log with sample data
2. Serialize to Arrow IPC format
3. Deserialize and verify all fields match

```bash
# Python
pytest bindings/python/tests/test_arrow.py

# R
Rscript -e 'testthat::test_file("bindings/r/tests/testthat/test-arrow.R")'

# Julia
julia --project=bindings/julia -e 'include("bindings/julia/test/test_arrow.jl")'

# TypeScript
cd bindings/typescript && npm test -- --testPathPattern arrow

# C#
dotnet test bindings/csharp/Kairo.ECS.sln --filter Category=Arrow

# Go
cd bindings/go && go test -run Arrow
```

## Reading telemetry

### Python (PyArrow)
```python
import pyarrow as pa
import pyarrow.ipc as ipc

with pa.ipc.open_file("telemetry.arrow") as reader:
    table = reader.read_all()
    print(table.to_pandas())
```

### R (arrow)
```r
library(arrow)
table <- read_ipc_file("telemetry.arrow")
print(table)
```

### Julia (Arrow.jl)
```julia
using Arrow
table = Arrow.Table("telemetry.arrow")
```

### TypeScript (apache-arrow)
```typescript
import { tableFromIPC } from "apache-arrow";
const table = tableFromIPC(fs.readFileSync("telemetry.arrow"));
console.log(table.toArray());
```

## Schema validation

```bash
# Validate event_log_v1 schema
cat schemas/arrow/event_log_v1.schema.json | python -m json.tool
```

## Related documents

- [Arrow schema contract](../../conductor/contracts/arrow-schema-contract.md)
- [Streaming architecture](../streaming/architecture.md)
- [Stream schema](../streaming/stream-schema.md)
