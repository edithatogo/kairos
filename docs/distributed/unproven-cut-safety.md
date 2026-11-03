# Unproved distributed cuts fail closed

Maturity: safety prerequisite; CAP-48.1 remains open.

The scheduler-facing `PdesTransport::all_reduce_min` now returns `Tick::ZERO`
for TonicTransport, GrpcSocketTransport and MpiProcessTransport. These live
network backends do not implement a complete consistent cut of model state,
queued/in-flight positive and anti-messages, retained cancellation/retirement
obligations and restart/session fencing. Responsive peers and a complete set of
scalar proposals do not prove that cut. An inbox can receive an earlier event
between the initial sample and reduction completion.

A no-peer listener is not a closed single participant: it can still accept an
independent client's remote event. It therefore also returns zero. Closed
in-memory emulators retain their local minima. NativeMpiExchange remains an
explicit raw MPI collective API and is not connected to the scheduler trait.

## Compatibility decision

`reduce_reported_minimum` preserves each network backend's old contribution
arithmetic for diagnostics only, including local buffer sampling, bounded waits
and missing-participant zero behavior. Existing numeric/round-freshness tests
continue against this explicitly named API. Do not use its result for model
advancement, fossil collection, committed GVT or checkpoint authority. The wire
fields and protocol versions remain unchanged; names containing GVT in the
legacy protocol do not certify an accounting proof.

Network event delivery remains available. `PdesScheduler` keeps its existing
monotonic `max(old_gvt, returned_floor)` and LP clock behavior; the change
prevents new unproved distributed GVT progress. It does not undo an unsafe floor
previously committed by older code. This deliberate progress limitation must not
be bypassed by passing a diagnostic minimum into the scheduler or an owned
runtime. The native owned group-cut and retirement verification APIs are unchanged.

## Required replacement

A future positive network floor requires reviewed complete ownership/membership,
model/queue and channel accounting, acknowledged admission/retirement, failure
semantics, authenticated session/epoch fencing and durable restart state. A
cooperative two-phase inbox sample alone is not atomic distributed agreement;
lost acknowledgements, independently injected traffic and resumed senders can
invalidate it. Keep that design separate from this fail-closed correction.

CAP-48.1 still requires actual two-host rollback, anti-message behavior, safe
GVT, injected delay/duplication/worker failure, restart and deterministic parity
against the qualified serial baseline. The configured NAS SSH route timed out
on the 3 November readback; no remote test ran or native state changed.

## Focused verification

```sh
cargo test --locked -p kairo-ecs-grpc -p kairo-ecs-mpi --test unproven_cut_safety
cargo test --locked -p kairo-ecs-pdes -p kairo-ecs-grpc -p kairo-ecs-mpi --all-features
cargo clippy --locked -p kairo-ecs-pdes -p kairo-ecs-grpc -p kairo-ecs-mpi --all-targets --all-features -- -D warnings
```

The first command tests responsive network participants and no-peer listeners,
requiring zero through the scheduler interface. Broader checks retain ordinary
message delivery, buffered arithmetic, contribution-round freshness, native MPI
preview behavior and local rollback/replay tests. These checks do not prove a
live two-host distributed runtime or authorize a phase/status advance.
