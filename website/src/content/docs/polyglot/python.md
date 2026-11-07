---
title: Python
description: Python orchestration through the actual Rust native engine, with explicit unsupported-operation boundaries.
---

Python's `NativeEngine` forwards a narrow scheduling surface to the Rust C ABI.
The earlier pure-Python `Scheduler`, demonstration `VecEnv` and PettingZoo
execution are unsupported. A missing native library is an error.

## Load and run Rust

Follow [installation](https://github.com/edithatogo/kairos/blob/main/docs/install.md)
to build the matching library, install the binding and set
`KAIRO_ECS_FFI_LIBRARY` to its absolute path. The loader checks ABI version and
required symbols.

```python
from kairo_ecs import NativeEngine

with NativeEngine() as engine:
    handle = engine.schedule_at(5, priority=0, kind=1)
    assert handle != 0
    assert engine.run_for(1) == 1
    assert engine.stats()["dispatched_events"] == 1
```

Handles and simulation state belong to Rust. The context manager releases the
native engine. Package import, metadata and DTO tests prove their own narrower
scope; installed-library tests must exercise actual execution.

## Scope and next steps

- [Python tutorial](https://github.com/edithatogo/kairos/blob/main/docs/tutorials/python-getting-started.md) and [binding README](https://github.com/edithatogo/kairos/blob/main/bindings/python/README.md).
- [Rust-only migration](https://github.com/edithatogo/kairos/blob/main/docs/release/migration.md) and [capability register](https://github.com/edithatogo/kairos/blob/main/conductor/capabilities.json).
- [Model-building guidance](https://github.com/edithatogo/kairos/blob/main/docs/tutorials/model-building.md) and [validation datasets](https://github.com/edithatogo/kairos/blob/main/conductor/validation-datasets.md).

Arbitrary model callbacks, simulation-backed vectorized RL, native wheels and
every scientific dependency combination are not implied by this narrow example.
Use the explicitly qualified runtime and adapter matrix for a release.
