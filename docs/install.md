# Installing KairoECS

KairoECS is an alpha Rust simulation engine. The
[v0.4.0-alpha.1 source release](https://github.com/edithatogo/kairos/releases/tag/v0.4.0-alpha.1)
was published on 18 October 2026; default-branch APIs may be newer. A source tag
does not imply that all registry packages or native wheels are published.

**Rust is the only supported simulation core.** Language packages are adapters
to Rust native code or Rust-compiled WebAssembly. Missing native capabilities
fail explicitly. Older pure-language preview schedulers are unsupported under
[ADR 0010](adr/0010-rust-only-execution.md).

## Build and test the Rust core

Install Rust through rustup, then use the toolchain selected by the checked-in
`rust-toolchain.toml`. Keep the lockfile and record the source revision used:

```bash
git clone https://github.com/edithatogo/kairos.git
cd kairos
git rev-parse HEAD
cargo test --locked -p kairo-ecs-core -p kairo-ecs-state
cargo test --locked -p kairo-ecs-des -p kairo-ecs-abm
```

These commands cover the core learning path. Optional MPI, GPU, FMI and data
features have their own system-library and feature requirements; a default test
run does not qualify them. See [current capabilities](current-state.md), the
[Rust quickstart](tutorials/rust-getting-started.md) and [crate inventory](../crates/README.md).
For a released source snapshot, check out its tag and follow that tag's docs.
The native-only adapter examples below describe the updated default branch.

## Python: load the actual Rust library

The source package exposes `NativeEngine`, a narrow wrapper around the Rust C
ABI. Build the shared library from the same revision as the adapter:

```bash
cargo rustc --locked --release -p kairo-ecs-ffi --lib --crate-type cdylib
```

Locate the produced library under `target/release` (Cargo may place it in
`target/release/deps`). Its platform name is `libkairo_ecs_ffi.so` on Linux,
`libkairo_ecs_ffi.dylib` on macOS or `kairo_ecs_ffi.dll` on Windows. Set
`KAIRO_ECS_FFI_LIBRARY` to its **absolute path**. A path to a directory, a version
probe or a stub library is insufficient: the loader checks the ABI and required
symbols.

Use the qualified Python environment described below and install the local
binding with that interpreter:

```bash
uv pip install --python .venv/bin/python -e bindings/python
```

Then run:

```python
from kairo_ecs import NativeEngine

with NativeEngine() as engine:
    event_handle = engine.schedule_at(5, priority=0, kind=1)
    assert event_handle != 0
    assert engine.run_for(1) == 1
    assert engine.stats()["dispatched_events"] == 1
```

The event handle comes from Rust; the host must treat it as opaque. This proves
only the operations exercised against the installed library. The retired
`Scheduler` facade and demonstration `VecEnv`/PettingZoo surfaces are unsupported.
See the [Python tutorial](tutorials/python-getting-started.md) and
[binding README](../bindings/python/README.md) for native acceptance and errors.

## TypeScript: supply a generated Rust/Wasm module

Build the TypeScript adapter using its checked-in lockfile and supported Node
runtime. Build the `kairo-ecs-wasm` Rust crate with its export feature using the
binding's documented toolchain, and provide the generated JavaScript module URL:

```ts
import { WasmBridgeLoader } from "@kairo-ecs/typescript";

const engine = await WasmBridgeLoader.loadWasmEngine({
  wasmModulePath: "/generated/kairo_ecs_wasm.js",
});
try {
  engine.schedule_at(5n, 0, 1);
  console.log(engine.run_for(1n));
  console.log(engine.stats_json());
} finally {
  engine.free();
}
```

The path is an example deployment location, not a bundled artefact. Rust `u64`
exports use JavaScript `bigint`. Loading without a generated module fails;
`createSchedulerFacade` and the former host-only inspector are unsupported.
See [Wasm and TypeScript](tutorials/wasm-getting-started.md) and the
[binding README](../bindings/typescript/README.md). A successful TypeScript build
does not establish browser execution, native Wasm parity or package publication.

## Other language adapters

| Surface | Current execution boundary | Source guidance |
| --- | --- | --- |
| R | Package metadata and data helpers can be inspected. Host-only scheduling is unsupported pending native qualification. | [R README](../bindings/r/README.md) |
| Julia | DTO and event-log helpers do not constitute a simulation engine. Native availability needs execution evidence. | [Julia README](../bindings/julia/README.md) |
| C# | Raw `NativeMethods` maps the C ABI; the former managed scheduler is unsupported. A built managed assembly is not an installed-native acceptance result. | [C# README](../bindings/csharp/README.md) |
| Go | Metadata/data helpers remain separate from execution. The former Go scheduler is unsupported pending a qualified native adapter. | [Go README](../bindings/go/README.md) |

Do not use an old package's local simulation fallback to work around a missing
native operation. Follow the [migration guide](release/migration.md) and
[capability register](../conductor/capabilities.json).

## Qualified Python runtime

The selected tooling default is CPython 3.15.2. When `.venv` is absent, use
`uv venv --python 3.15.2 .venv`, then verify `.venv/bin/python --version`
(Windows: `.venv\Scripts\python.exe`). Install the relevant hash-locked tooling
with `uv pip sync --python .venv/bin/python scripts/bootstrap-python-tools.lock`.
Do not replace an environment in use.

A necessary alternate profile requires a concrete consumer/reason, exact
constraints and actual native/runtime/CI qualification under the
[dependency policy](../conductor/dependency-value-policy.md). Selecting a tooling
interpreter does not qualify every scientific wheel or native adapter on it.

## Troubleshooting

- A missing Rust toolchain or system library is a build failure to resolve; it
  does not justify a host-language simulation fallback.
- `NativeEngine` load errors require checking the absolute path, platform
  architecture, ABI version and complete symbol set of the built library.
- Missing Wasm modules require generating and serving the Rust artefacts at the
  configured URL. Do not substitute the retired in-memory scheduler.
- For optional hardware backends, use their documented probe and acceptance
  suite and retain the hardware, feature and source metadata.

The [release gates](../conductor/release-gates.md) define the installed-consumer
checks needed before broader support is advertised.
