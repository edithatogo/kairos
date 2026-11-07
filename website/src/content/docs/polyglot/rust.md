---
title: Rust
description: The supported simulation core, source-based quickstart, and explicit optional-feature boundaries.
---

Rust is the only supported simulation core. It owns scheduling, simulation
state, random streams and model execution. Host-language packages adapt this
core through a native ABI or Rust-compiled WebAssembly.

## Start from the pinned source

```bash
git clone https://github.com/edithatogo/kairos.git
cd kairos
cargo test --locked -p kairo-ecs-core -p kairo-ecs-types
```

Use the repository's `rust-toolchain.toml` and lockfile. In a crate with path
dependencies on `kairo-ecs-core` and `kairo-ecs-types`, a minimal example is:

```rust
use kairo_ecs_core::Scheduler;
use kairo_ecs_types::{EventKind, ScheduleRequest, SimTime, StepOutcome};

fn main() {
    let mut scheduler = Scheduler::new();
    let id = scheduler.schedule(ScheduleRequest {
        at: SimTime::from_ticks(10),
        priority: 0,
        kind: EventKind::custom(1),
        entity: None,
    });
    assert!(scheduler.cancel(id));
    assert_eq!(scheduler.step(), StepOutcome::Empty);
}
```

Event ordering uses time, priority and insertion sequence. `SimTime` is an
unsigned 128-bit tick count; models define and record the unit. The
[complete tutorial](https://github.com/edithatogo/kairos/blob/main/docs/tutorials/rust-getting-started.md)
explains cancellation, source tests and model-building next steps.

## Choose a capability deliberately

| Need | Source area | Qualification boundary |
| --- | --- | --- |
| Core scheduling and state | `kairo-ecs-core`, `types`, `state`, `rng` | Model replay also depends on model version, RNG stream version and numerical rules. |
| DES and ABM primitives | `kairo-ecs-des`, `kairo-ecs-abm` | Validate application semantics and composed behaviour separately. |
| Data exchange | `kairo-ecs-arrow` | Optional native dependencies, formats and ownership require consumer-specific checks. |
| Parallel/distributed execution | `kairo-ecs-pdes`, `mpi`, `grpc` | Bounded native evidence exists; choose workloads using measured results. |
| GPU execution | `kairo-ecs-gpu`, `webgpu` | Particular kernels and device paths are qualified, not arbitrary model offload. |
| Scientific methods | Game-theory, inference and FMI crates | Method names are not independent correctness or interoperability evidence. |

See the [crate inventory](https://github.com/edithatogo/kairos/blob/main/crates/README.md),
[current-state review](https://github.com/edithatogo/kairos/blob/main/docs/current-state.md)
and [capability register](https://github.com/edithatogo/kairos/blob/main/conductor/capabilities.json).
The alpha APIs may change. Keep domain models in the consuming application and
pin its Kairos dependency to a reviewed version.
