# Stream Schema

The initial event-log stream is `kairo_ecs.event_log.v1`.

| Field | Type in scaffold | Contract source |
|---|---|---|
| `run_id` | `String` | Arrow `Utf8` |
| `event_id` | `Option<u64>` | Arrow `UInt64` or fixed binary |
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
