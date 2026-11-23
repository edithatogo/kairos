# Time-Travel Trace Format

The offline line encoding uses schema `kairo.ecs.trace.v1`. In memory, an
`EventTrace` stores the following records.

- snapshots of the state after all deltas at each snapshot tick
- event deltas keyed by dispatched event id, priority, sequence, entity, and event kind
- state changes as deterministic key/value maps

The default trace has no snapshots. `reconstruct_at(tick)` applies every
delta through the requested tick, starting with the latest end-of-tick
snapshot at or before that tick when one exists. Event-by-event inspection
uses an earlier snapshot when the cursor precedes later events at the same
tick. This is caller-supplied state playback. The
[native debugger](cli-reference.md) executes Rust model transitions from
scheduler and model checkpoints.

`validate_trace_lines` checks the schema header, record kind, tick ordering,
event-kind encoding and basic delta field shape. The line encoding omits
priority and entity fields. Arrow IPC trace serialization remains open.

Forward compatibility rules

- Readers must ignore unknown columns.
- Writers must keep `schema`, `tick`, `event_id`, `sequence`, and `kind`.
- Snapshot ticks are sparse; replay reconstructs from the nearest snapshot at or before the requested tick.
