# Time-Travel Trace Format

Trace files use schema `kairo.ecs.trace.v1`. The implementation scaffold records:

- tick-aligned snapshots keyed by simulation tick
- event deltas keyed by dispatched event id, priority, sequence, entity, and event kind
- state changes as deterministic key/value maps

The Rust crate currently exposes an offline line encoding for smoke tests. `validate_trace_lines` checks the schema header, record kind, tick ordering, event-kind encoding, and basic delta field shape before replay smoke tests use the trace in memory. Arrow IPC serialization remains the integration target for the Track 04 telemetry bridge and must preserve the same logical fields.

Forward compatibility rules:

- Readers must ignore unknown columns.
- Writers must keep `schema`, `tick`, `event_id`, `sequence`, and `kind`.
- Snapshot ticks are sparse; replay reconstructs from the nearest snapshot at or before the requested tick.
