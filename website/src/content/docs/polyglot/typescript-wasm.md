---
title: TypeScript and Wasm
description: Load a generated Rust/Wasm engine; unavailable modules fail explicitly.
---

TypeScript executes the Rust core through generated WebAssembly. Independent
JavaScript schedulers are unsupported. Supply the generated module explicitly:

```ts
import { WasmBridgeLoader } from "@kairo-ecs/typescript";

const engine = await WasmBridgeLoader.loadWasmEngine({
  wasmModulePath: "/generated/kairo_ecs_wasm.js",
});
try {
  engine.schedule_at(10n, 0, 1);
  console.log(engine.run_for(1n));
  console.log(engine.stats_json());
} finally {
  engine.free();
}
```

The URL is an example deployment path. Generate and serve the module using the
[binding's build instructions](https://github.com/edithatogo/kairos/blob/main/bindings/typescript/README.md).
The loader initializes and validates the native engine; no module means an
explicit load error. Rust `u64` parameters/results use JavaScript `bigint`.

## Support boundary

The example covers native scheduling, bounded execution and stats. It does not
establish arbitrary browser model execution, WebGPU acceleration or time-travel
restoration. Snapshot-shaped metadata and history counters are insufficient
evidence for causal provenance or reversible state.

The former `createSchedulerFacade` and `TimeTravelInspectorClient` are retired.
See the [Wasm tutorial](https://github.com/edithatogo/kairos/blob/main/docs/tutorials/wasm-getting-started.md),
[migration guide](https://github.com/edithatogo/kairos/blob/main/docs/release/migration.md)
and [capability register](https://github.com/edithatogo/kairos/blob/main/conductor/capabilities.json).
Actual generated-module and browser acceptance remains distinct from TypeScript
unit tests or package publication.
