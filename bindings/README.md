# Language bindings: Rust core only

Kairos execution is implemented in Rust. Host languages may provide package
metadata, DTOs, data conversion and adapters to compiled Rust. They must not
implement an alternative scheduler, RNG, simulation state machine or synthetic
model output. This policy applies immediately, including alpha packages.

The old Python, R, TypeScript, Go and C# scheduler implementations are retired.
Their public entry points fail explicitly. Existing users must handle this
breaking change; there is no compatibility mode that runs a non-Rust core.

## Current execution boundary

| Binding | Executable Rust path | Explicitly unavailable |
| --- | --- | --- |
| [Python](python/README.md) | `NativeEngine` loads a selected C ABI v1 shared library and owns a Rust engine handle | Legacy `Scheduler`; synthetic `ParallelVecEnv` / PettingZoo; model execution, RNG and full trace APIs absent from ABI v1 |
| [TypeScript](typescript/README.md) | `WasmBridgeLoader.loadWasmEngine` loads a supplied generated Rust Wasm module and checks ABI v1 | JavaScript scheduler; synthetic time-travel inspector; packaged artifact and full history qualification |
| [C#](csharp/README.md) | `NativeMethods` raw P/Invoke plus `KairoEcsEngineHandle`, using the Rust C ABI shared library | Managed scheduler; prebundled native runtime assets |
| [Go](go/README.md) | No qualified runtime adapter yet | `NewEngine` and `NewNativeEngine` return `ErrNativeNotConfigured`; cgo header compilation does not enable execution |
| [R](r/README.md) | No qualified runtime adapter yet | All scheduler operations raise `kairoecs_native_required` |
| [Julia](julia/README.md) | No qualified runtime adapter yet | Engine execution and populated native Arrow export |

An import, file existence, header compilation, or fixture parse does not prove
native execution. Binding tests distinguish metadata/data conversion, negative
native-required controls, and tests that actually load Rust. A native acceptance
job must fail when its required artifact is missing.

## Event-log data conversion

Data conversion remains supported independently of simulation. The following
surfaces do not produce evidence that a Rust engine generated their input.

| Binding | Current conversion | Copy and capability limits |
| --- | --- | --- |
| Python | Schema v2 DTOs, smoke-byte codec, PyArrow table/capsule conversion | Builds Arrow arrays from Python columns; PyArrow capsules are real but do not prove Rust FFI handoff or end-to-end zero-copy |
| TypeScript | Schema v2 constants and JSON payload roundtrip | Copies plain objects; u64 fields use decimal strings; no native Arrow bridge |
| R | Schema v2 data-frame normalization and optional `arrow::Table` roundtrip | Copies values; external u64 values remain decimal strings; no native Arrow bridge |
| Julia | Schema v2 DTOs, ordering of existing records, escaped tabular smoke codec | Copies values; the smoke codec is not Arrow IPC and does not run a simulator |
| C#, Go | Event/stats DTOs | Populated event-log interchange is not implemented |

See the [runtime policy inventory](runtime-policy.json) for machine-readable
capability boundaries. Rust/Wasm artifact provenance, ABI compatibility,
platform coverage and actual execution remain release gates.
