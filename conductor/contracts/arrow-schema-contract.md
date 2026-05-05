# Arrow Schema Contract

## Purpose

KairoECS uses Apache Arrow to avoid per-event object transfer into Python/R/Julia/TypeScript/C#/Go.

## Initial streams

```text
kairo_ecs.event_log.v1
kairo_ecs.metric_sample.v1
kairo_ecs.entity_snapshot.v1
kairo_ecs.resource_snapshot.v1
kairo_ecs.conformance_result.v1
```

## event_log_v1 fields

| Field | Type | Notes |
|---|---|---|
| `run_id` | Utf8 | deterministic run/session ID |
| `event_id` | UInt64 or FixedSizeBinary | serialized event handle |
| `entity_id` | UInt64 nullable | serialized entity handle |
| `time_ticks` | UInt128 fallback as FixedSizeBinary/Decimal128 | fixed simulation time |
| `time_scale` | Utf8 | e.g. nanoseconds |
| `priority` | Int32 | scheduler priority |
| `sequence` | UInt64 | stable ordering sequence |
| `event_kind` | Utf8 or dictionary | event classification |
| `status` | Utf8/dictionary | dispatched/cancelled/skipped/error |
| `payload_ref` | Utf8 nullable | external payload reference |

## Compatibility

1. Additive nullable fields are compatible within the same major schema.
2. Field removal or type change requires new schema major version.
3. Every binding must include an Arrow roundtrip test.
