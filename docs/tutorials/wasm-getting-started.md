# Wasm And TypeScript Getting Started

TypeScript is an adapter to Rust-compiled WebAssembly. The former in-memory
scheduler and host-only time-travel inspector are unsupported immediately under
[ADR 0010](../adr/0010-rust-only-execution.md).

## Build the native module

Follow the [TypeScript binding README](../../bindings/typescript/README.md) for
the supported Node runtime, lockfile and Rust/Wasm build steps. The Rust export
source is [kairo-ecs-wasm](../../crates/kairo-ecs-wasm/src/lib.rs). Build it with
its `wasm-export` feature and serve the generated JavaScript and Wasm files.

The configured URL below is an example deployment location, not a file shipped
with the source package:

```ts
import { WasmBridgeLoader } from "@kairo-ecs/typescript";

const engine = await WasmBridgeLoader.loadWasmEngine({
  wasmModulePath: "/generated/kairo_ecs_wasm.js",
});
try {
  const eventHandle = engine.schedule_at(10n, 0, 1);
  console.log(eventHandle, engine.run_for(1n));
  console.log(engine.stats_json());
} finally {
  engine.free();
}
```

The loader initializes the generated module and checks its engine surface and
ABI. Rust `u64` values use JavaScript `bigint`; do not convert event handles to
floating-point numbers. An unavailable or incompatible module fails explicitly.
A status of `not-configured` means no native runtime is configured; it never
enables a host-language scheduler fallback.

## Qualification boundaries

The supported learning path is scheduling, bounded execution and native stats.
The presence of a snapshot-shaped object or a history counter does not establish
state restoration, reversible execution or causal provenance. The earlier
`createSchedulerFacade` and `TimeTravelInspectorClient` APIs are retired.

Run the binding's type, unit and native browser/Wasm acceptance checks for the
actual generated artefact. Preserve the Rust revision, generated module hash,
browser/Node version and execution output. A mocked loader or passing JavaScript
unit test proves only its own boundary.

## Continue

- [Installation](../install.md) and [current capabilities](../current-state.md).
- [Playground intent](../community/playground.md) and [headless snapshot example](../../examples/viz/headless-snapshot/Cargo.toml).
- [Telemetry example](../../examples/telemetry/README.md) and [debug trace format](../debugging/trace-format.md).
- [Track 109](../../conductor/tracks/109-rust-only-runtime/plan.md) for complete native adapter acceptance.

Registry publication and browser/hardware support remain release-gated.
