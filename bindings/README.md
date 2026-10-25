# Language Bindings

Binding implementation roots:

- `python/`
- `r/`
- `julia/`
- `typescript/`
- `csharp/`
- `go/`

Keep package manifests out of a binding directory until its track is ready for CI to enforce real smoke tests.


## Event-log interchange surface (Track 70 Phase 2, Schema v2)

All bindings project the Track 70 event-log contract onto an interchange
surface; none claims universal zero-copy. The table records the actual copy
each binding performs and its explicitly unsupported targets.

| Binding | Schema surface | Actual copies | Explicitly unsupported targets |
| --- | --- | --- | --- |
| `python/` | `kairo_ecs.event_log.v2` (`pyarrow.Array`, `RecordBatch`) | Native `pyarrow` tables from the FFI populated export (Arrow C Data buffer hand-off, released exactly once); Python-row reconstruction removed in Task 2.2 | Minimal/browser builds (Polars stays out of those dependency graphs) |
| `typescript/` | v2 constants + row projection | Plain row objects per event (value copy); no capsule bridge | Native wasm loading and populated Arrow C Data/Stream export until Track 09 wires artifacts |
| `r/` | v2 data-frame facade + optional `arrow::Table` roundtrip | Base-R data-frame row copies; u64 columns as decimal character strings (2^53 double limit documented in-binding) | Populated FFI Arrow C Data/Stream export (native FFI artifact not configured), full u128 tick inputs above 2^53 |
| `julia/` | v2 constants + smoke-byte codec | Dependency-light escaped tabular value copy per record (decimal integer limbs) | Native Arrow.jl IPC and populated FFI export (native library artifact not configured) |
| `csharp/`, `go/` | None yet | None | Track 63 surfaces; event-log interchange not implemented |

Fake/descriptor-only exports were removed from the Python path in Task 2.2; the
FFI capability-absent (non-`native-data`) builds keep the C symbols for ABI
stability and fail explicitly with a feature-naming last error instead of
fabricating buffers or streams.
