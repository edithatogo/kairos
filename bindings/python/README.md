# Python binding

Track 06 owns this binding. Simulation execution requires a compiled Rust C ABI
v1 shared library. The Python scheduler and synthetic vector/PettingZoo
environments are retired immediately; they raise `UnsupportedNativeOperation`.
The package can still be imported for DTOs and data conversion without a native
artifact. Such an import reports `metadata_only`, not engine readiness.

## Rust engine

Build the FFI shared library from the repository root with the pinned Rust
and Python toolchains. Until the crate supplies a packaged shared artifact, the
source build route is:

```bash
cargo rustc --locked --release -p kairo-ecs-ffi --lib --crate-type cdylib --message-format=json-render-diagnostics
```

Select the emitted shared-library filename from the `compiler-artifact` JSON
message for `kairo-ecs-ffi`: a `.so` (Linux), `.dylib` (macOS), or `.dll`
(Windows). Cargo output paths can include a hash or `deps/` directory, so CI
should use that emitted path instead of guessing a filename. Set `KAIRO_ECS_FFI_LIBRARY` to its **absolute file path**, or
pass that path to `NativeEngine`. The constructor loads every required symbol
and checks ABI version 1. Missing, invalid or incompatible artifacts raise
`NativeBackendUnavailable`; no fallback executes.

```python
from kairo_ecs import NativeEngine

with NativeEngine() as engine:
    event = engine.schedule_at(5, priority=-1, kind=7)
    assert engine.run_for(1) == 1
    assert engine.current_time_ticks == 5
    assert engine.stats()["dispatched_events"] == 1
```

`NativeEngine` owns and frees its Rust handle. `schedule_at` and
`schedule_after` return opaque integer event handles; `cancel` returns whether a
pending native event was cancelled. `step` reports whether Rust dispatched an
event. `run_for`, `run_until`, and `run_until_or_for` return native dispatched
counts. `reset`, `stats` and the current time delegate to Rust. Calls after
`close` raise `NativeExecutionError`; close is idempotent.

ABI v1 accepts u64 ticks/handles, i32 priorities and u32 event-kind codes. The
adapter rejects out-of-range values instead of truncating them. It does not
provide string event kinds, full dispatched records, RNG draws, a model plugin,
or vector environments. The old `Scheduler` trace API is not emulated in
Python. These capabilities need Rust APIs and qualification before being
supported.

## Data conversion

`EventLogBatch` accepts explicit event DTOs and provides Schema v2 conversion.
PyArrow tables and C Data capsules are backed by real PyArrow arrays; those
arrays are built from host columns. These tests verify data conversion, not a
populated Rust FFI Arrow handoff or end-to-end zero-copy.

## Validation

Metadata, retirement controls and data conversion (from `bindings/python`):

```bash
python -m pytest tests -q
python -m build
python -m pip check
```

**Required native execution gate**, from the repository root after setting the
absolute library path:

```bash
PYTHONPATH=bindings/python python -m unittest discover -s bindings/python/native_tests -v
```

This native suite has no missing-artifact skip and no mock engine. It exercises
scheduling, cancellation, bounded runs, reset, exact u64 ticks, errors and
lifetime closure against Rust. It must run separately from host-only tests in
native CI. Passing host-only tests does not qualify a native release.
