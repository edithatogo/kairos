# LogicalProcess Trait

`LogicalProcess` is the unit of PDES partitioning. Each LP owns a `WorldSegment`,
processes local events up to a requested tick, emits remote events for other LPs,
receives inbound remote events, and advances to the latest safe time chosen by
the scheduler.

The scaffold lives in `crates/kairo-ecs-pdes/src/lib.rs` and defines:

- `LpId`: stable logical-process identifier.
- `WorldSegment`: LP-owned entity partition metadata.
- `RemoteEvent`: `(source_lp, dest_lp, tick, event_payload)`.
- `LogicalProcess`: lifecycle trait for init, local processing, remote exchange,
  inbound receipt, and safe-time advancement.

Contract:

- `process_local_events(until)` must not process events after `until`.
- `schedule_remote_event()` may only return events that respect the LP lookahead.
- `receive_remote_events(events)` must enqueue events without immediately
  violating local event ordering.
- `advance_to(tick)` is called only after GVT or a transport-safe lower bound has
  been computed.

The current scheduler contract is intentionally strict at the boundary:

- `PdesScheduler::add_lp(...)` returns `Result` and rejects duplicate LP IDs,
  mismatched `WorldSegment::id`, self-neighboring LP declarations, and duplicate
  neighbor entries.
- `PdesScheduler::step_until(...)` returns `Result` and fails if transport
  routing receives or sends to a logical-process ID not registered in the
  scheduler transport.
- `PdesTransport` implementers must return `TransportError::UnknownLogicalProcess`
  for any destination/source outside the known transport participants.

The intent is to make topology and routing failures deterministic, local, and
easier to test before any MPI or runtime backend is introduced.

Validation command:

```powershell
cargo test --manifest-path crates/kairo-ecs-pdes/Cargo.toml --features pdes
```
