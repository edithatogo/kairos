# Rust Getting Started

Rust is the only supported simulation core. This tutorial uses checked-in source
and the pinned toolchain; it does not depend on unpublished registry packages.

## Scheduler contract

The [core scheduler](../../crates/kairo-ecs-core/src/lib.rs) provides event
ordering by time, priority and insertion sequence; opaque generational event IDs;
cancellation of pending events; and bounded stepping. Shared event types live
in [kairo-ecs-types](../../crates/kairo-ecs-types/src/lib.rs).

## Minimal scheduler example

Use this code in a Rust crate with path dependencies on `kairo-ecs-core` and
`kairo-ecs-types` from the same repository revision:

```rust
use kairo_ecs_core::Scheduler;
use kairo_ecs_types::{EventKind, ScheduleRequest, SimTime, StepOutcome};

fn main() {
    let mut scheduler = Scheduler::new();
    let event_id = scheduler.schedule(ScheduleRequest {
        at: SimTime::from_ticks(10),
        priority: 0,
        kind: EventKind::custom(1),
        entity: None,
    });

    assert!(scheduler.cancel(event_id));
    assert_eq!(scheduler.pending_events(), 0);
    assert_eq!(scheduler.step(), StepOutcome::Empty);
}
```

`SimTime` uses unsigned 128-bit fixed ticks. A model assigns their physical units
and records that choice. Identical event inputs and ordering rules support
repeatable dispatch; application-level replay additionally depends on model,
RNG version, seed policy and numerical behaviour.

## Run the source tests

From the repository root:

```bash
cargo test --locked -p kairo-ecs-core -p kairo-ecs-types
```

The scheduler's checked-in tests exercise duplicate cancellation, already
cancelled/dispatched events, equal-time ordering and run limits. Use these as
executable references and verify API changes against the same revision.

`cargo check --workspace` is useful for compilation checks, but it does not run
tests or verify linking, runtime behaviour, hardware support or release readiness.

## Build a model

- [M/M/1 queue](../../examples/des/mm1_queue/README.md) for events and queueing.
- [Factory bottleneck](../../examples/des/factory_bottleneck/README.md) for resources and throughput.
- [Model-building tutorial](model-building.md) for a model card and validation plan.
- [Replay and seeds](../trustworthy-simulation/replay-and-seeds.md) for reproducibility expectations.
- [Benchmark policy](../benchmarks/benchmark-policy.md) for measurement rules.

Domain applications should depend on Kairos through public APIs and versioned
schemas. Keep application policies, datasets and empirical model validation in
the consuming repository; see the [development programme](../../conductor/frontier-programme-20261107.md).
