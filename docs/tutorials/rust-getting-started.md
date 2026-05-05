# Rust Getting Started

This tutorial introduces the current Rust scheduler surface without relying on
unpublished packages. Work from a local repository checkout.

## What exists now

The core scheduler lives in `crates/kairo-ecs-core/src/lib.rs`. It provides:

- deterministic ordering by simulation time, priority, and insertion sequence;
- explicit `EventId` values for scheduled events;
- cancellation for still-pending events;
- bounded stepping through `step`, `run_for`, `run_until`, and
  `run_until_or_for`.

The current Rust learning path is source-first. If your environment cannot run
native Rust tests because of local linker setup, `cargo check --workspace` still
proves the workspace compiles far enough for API review.

## First read

1. Open `crates/kairo-ecs-core/src/lib.rs`.
2. Find `Scheduler`, `ScheduleRequest`, `StepOutcome`, and the scheduler tests.
3. Read the cancellation tests before writing model code. They define the
   expected behavior for unknown, duplicate, cancelled, and already-dispatched
   event ids.

## Minimal scheduler shape

The checked-in tests are the safest executable examples for the current Rust
surface. The pattern is:

```rust
use kairo_ecs_core::{ScheduleRequest, Scheduler, SimTime, StepOutcome};

let mut scheduler = Scheduler::new(0);
let event_id = scheduler.schedule(ScheduleRequest {
    at: SimTime::from_ticks(10),
    priority: 0,
    kind: 1,
    entity: None,
});

assert!(scheduler.cancel(event_id));
assert_eq!(scheduler.pending_events(), 0);
assert_eq!(scheduler.step(), StepOutcome::Empty);
```

Treat this as an API-reading guide until the public Rust crate and example
commands are promoted by the packaging and release tracks.

## Next examples

- [M/M/1 queue](../../examples/des/mm1_queue/README.md) for DES vocabulary.
- [Factory bottleneck](../../examples/des/factory_bottleneck/README.md) for
  resource and throughput vocabulary.
- [Replay and seeds](../trustworthy-simulation/replay-and-seeds.md) for
  reproducibility expectations.
- [Benchmark policy](../benchmarks/benchmark-policy.md) for evidence boundaries.

## Validation

Use the repository gates that match the current maturity:

```powershell
cargo check --workspace
powershell -NoProfile -ExecutionPolicy Bypass -File docs/tutorials/validate-tutorials.ps1
```

Do not treat a successful check as proof of production readiness. It proves the
source-backed learning path still points at real artifacts.
