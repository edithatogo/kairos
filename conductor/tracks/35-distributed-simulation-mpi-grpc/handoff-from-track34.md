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
    fn knows_lp(&self, lp_id: LpId) -> bool;
    fn send(&mut self, dest: LpId, message: PdesMessage) -> Result<(), TransportError>;
    fn recv(&mut self, lp_id: LpId) -> Result<Vec<PdesMessage>, TransportError>;
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
- The reduced value must include pending in-flight event timestamps, not just
  local LP time proposals, so GVT cannot advance past a queued remote event.

## Current blockers

- Track 34 now has deterministic sequential parity and 8-LP, 10,000-tick stress
  fixtures covered by the GNU-toolchain runtime gate.
- Runtime execution of those tests now passes via
  `pwsh -NoProfile -File conductor/tracks/34-pdes-parallel-execution/validate-track34.ps1 -RunTests`
  under `stable-x86_64-pc-windows-gnu`.
- Track 35 may use the Track 34 fixtures for protocol-level parity work, but
  should not mark distributed runtime parity complete until real MPI/gRPC
  backends and 2-node execution checks exist.
