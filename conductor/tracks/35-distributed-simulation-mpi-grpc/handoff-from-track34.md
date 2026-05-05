# Handoff from Track 34 to Track 35

## Track 34 artifacts available

Track 34 now provides a validated compile-check scaffold in
`crates/kairo-ecs-pdes/`.

The public contract includes:

```rust
pub trait LogicalProcess {
    fn init(&mut self, lp_id: LpId, world_segment: &WorldSegment);
    fn process_local_events(&mut self, until: Tick);
    fn schedule_remote_event(&mut self) -> Vec<RemoteEvent>;
    fn receive_remote_events(&mut self, events: Vec<RemoteEvent>);
    fn advance_to(&mut self, tick: Tick);
    fn local_time(&self) -> Tick;
    fn lookahead(&self) -> SimDuration;
}
```

Track 35 should implement transport backends against:

```rust
pub trait PdesTransport {
    fn send(&mut self, dest: LpId, message: PdesMessage);
    fn recv(&mut self, lp_id: LpId) -> Vec<PdesMessage>;
    fn barrier(&mut self);
    fn all_reduce_min(&mut self, timestamp: Tick) -> Tick;
}
```

## Event exchange protocol

- Events are `(source_lp, dest_lp, tick, event_payload)` messages.
- An LP at local time `T` may only schedule events for remote LPs at time
  `>= T + L` where `L` is the LP's declared lookahead.
- Null messages carry `(source_lp, dest_lp, local_time + lookahead)` and are the
  CMB deadlock-avoidance signal.
- Track 35 must preserve these semantics regardless of MPI or gRPC transport.

## GVT contract

GVT is:

```text
min(local_time_of_all_LPs, min_timestamp_of_all_inflight_messages)
```

The Track 34 scaffold exposes `PdesTransport::all_reduce_min` as the replacement
point for distributed GVT:

- MPI: `MPI_Allreduce(MPI_MIN)`.
- gRPC: coordinator aggregation over worker proposals.

## Current blockers

- Track 34 sequential parity and stress tests are not implemented yet.
- Track 35 may proceed with transport implementation, but should not mark
  distributed parity complete until Track 34 has a real parity fixture.
