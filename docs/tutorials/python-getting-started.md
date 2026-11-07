# Python Getting Started

Python orchestrates the Rust core through `NativeEngine`. The former
pure-Python `Scheduler` is unsupported immediately; construction fails instead
of silently running another implementation. See [ADR 0010](../adr/0010-rust-only-execution.md).

## Prerequisites

Follow [installation](../install.md) to build the Rust C ABI library, install the
source binding in a qualified Python environment and set
`KAIRO_ECS_FFI_LIBRARY` to the absolute library path. The adapter requires the
matching ABI and complete native symbol set.

## Schedule and execute in Rust

```python
from kairo_ecs import NativeEngine

with NativeEngine() as engine:
    first = engine.schedule_at(5, priority=0, kind=1)
    assert first != 0
    assert engine.run_for(1) == 1
    stats = engine.stats()
    assert stats["current_time_ticks"] == 5
    assert stats["dispatched_events"] == 1
    assert stats["pending_events"] == 0
```

`first` is the opaque handle returned by Rust. The context manager releases the
native engine. `run_for` reports how many native events were dispatched; these
operations do not execute a Python model or Python event queue.

The C ABI time interface is `u64` ticks, narrower than Rust's `u128` `SimTime`.
Keep inputs within the adapter's documented range. Use the Rust API directly
when a required operation is not exposed by this adapter.

## Native acceptance

Read [the binding README](../../bindings/python/README.md) and run its native
acceptance suite with the freshly built library. Retain the library checksum,
source revision, Python version and command output. Run the ordinary package
tests as well: explicit failures for missing libraries and retired constructors
are part of the contract.

Compilation, import and DTO checks alone cannot establish native scheduling.
The old `Scheduler.cancel` examples describe the retired facade; they must not
be used as evidence for the native cancellation contract. Tests must exercise
the Rust-owned handle and state through the supported adapter.

## Models and data

- [Model-building tutorial](model-building.md) explains model cards and validation.
- [M/M/1 queue](../../examples/des/mm1_queue/README.md) and [emergency department flow](../../examples/hybrid/emergency_department_flow/README.md) provide domain context.
- [Arrow schema reference](../arrow/schema-reference.md) describes data exchange.
- [Current capabilities](../current-state.md) separates native implementation from model validity.

The demonstration `VecEnv` and PettingZoo constructors remain unsupported until
a Rust model adapter performs reset/step/state transitions and passes independent
acceptance tests. Do not interpret synthetic observations as simulation output.
