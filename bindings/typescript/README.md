# TypeScript / Rust Wasm binding

Track 09 owns this binding. JavaScript simulation cores are unsupported
immediately. `SchedulerFacade`, `createSchedulerFacade` and the synthetic
`TimeTravelInspectorClient` throw `UnsupportedCoreError`.

## Load a generated Rust module

Build `crates/kairo-ecs-wasm` with its `wasm-export` feature and the appropriate
wasm-bindgen/wasm-pack target. The generated module and `.wasm` artifact must be
available to the consumer; this npm surface does not bundle a qualified native
artifact yet.

```typescript
import { WasmBridgeLoader } from "@kairo-ecs/typescript";

const engine = await WasmBridgeLoader.loadWasmEngine({
  wasmModulePath: new URL("./generated/kairo_ecs_wasm.js", import.meta.url).href,
});
try {
  engine.schedule_at(5n, 0, 1);
  console.log(engine.run_for(1n));
  console.log(engine.stats_json());
} finally {
  engine.free();
}
```

The loader imports the supplied module, runs its default initializer when
present, checks the generated `WasmEngine` methods and ABI version 1, and
returns that engine. It does not substitute an inspector or a JavaScript
scheduler. Missing modules, initialization failures and incompatible ABI shapes
raise `NativeWasmNotConfiguredError`.

`loadNativeWasm({ load: () => import(moduleUrl) })` provides the equivalent
explicit loader interface. `nativeWasmStatus` is discovery metadata: even a
provided loader reports `configured-unverified` until actually loaded. ABI
checks are not artifact provenance or complete release qualification; consume
Rust artifacts from a trusted, pinned build.

Current supported method names mirror Rust: `schedule_at`, `step`, `run_for`,
`current_time_ticks`, `stats_json`, `reset`, `ffi_version` and `free`. Use bigint
for u64 inputs. Rich event logs, model adapters, populated Arrow exports and
verified time-travel/provenance remain unfinished capabilities. The retired
synthetic inspector must not be used as evidence of these features.

## Metadata and data conversion

Schema v2 constants and `roundTripArrowEventLog` remain available without Wasm.
The latter copies JSON-compatible data supplied by the caller. It does not
execute events or infer native event generations; decimal strings preserve
u64 values. There is no Arrow C Data bridge in this package.

## Validation

```bash
npm ci
npm run typecheck
npm test
npm run test:policy
npm run test:browser
```

These commands cover metadata, data conversion and explicit failure controls.
The browser smoke verifies these same boundaries; it is not a native Wasm
acceptance test.

For actual Rust Wasm acceptance, first build the generated Node-compatible
module, set `KAIRO_ECS_WASM_MODULE` to its file path, and run:

```bash
npm run test:native
```

This gate fails if the artifact is missing, checks real scheduling/time/counts
and reset, and has no synthetic replacement. Historical
`bench/results-node22-20k.json` measured the retired JavaScript scheduler and
cannot substantiate current Rust performance. Its old benchmark entry point now
fails explicitly.
