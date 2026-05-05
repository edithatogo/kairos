# Stream Schema

The initial event-log stream is `kairo_ecs.event_log.v1`.

| Field | Type in scaffold | Contract source |
|---|---|---|
| `schema_version` | `u16` | Track 04 crate schema version |
| `run_id` | `String` | Arrow `Utf8` |
| `event_id` | `Option<u64>`, required by validator | Arrow `UInt64` or fixed binary |
| `entity_id` | `Option<u64>` | Arrow nullable `UInt64` |
| `time_ticks` | `u128` | Arrow Decimal128/fixed binary fallback |
| `time_scale` | `String` | Arrow `Utf8` |
| `priority` | `i32` | Arrow `Int32` |
| `sequence` | `u64` | Arrow `UInt64` |
| `event_kind` | `String` | Arrow `Utf8` or dictionary |
| `status` | `StreamStatus` | Arrow dictionary-compatible string |
| `payload_ref` | `Option<String>` | Arrow nullable `Utf8` |

The scaffold keeps this schema explicit in code so broker adapters can be tested
against a single field-name contract before Arrow IPC support lands.

## Local Validators

The dependency-free validator rejects:

- unknown event-log stream names;
- schema versions other than `1`;
- blank `run_id` values;
- missing `event_id` values;
- `time_scale` values other than `ticks`;
- blank `event_kind` values;
- blank `payload_ref` values when a payload reference is present.

The in-memory scaffold adapter also rejects duplicate or decreasing `sequence`
values for the same `run_id`. That check is a local ordering guard only; real
broker adapters still need partition, acknowledgement, and replay tests before
they can claim runtime ordering guarantees.
