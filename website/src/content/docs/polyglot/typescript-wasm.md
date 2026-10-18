---
title: TypeScript and Wasm
description: WebAssembly simulation engine, copy-on-write time-travel inspector, and WebGPU browser studio.
---

The TypeScript and WebAssembly surface (`crates/kairo-ecs-wasm` and `bindings/typescript/`) brings deterministic simulation and copy-on-write time-travel debugging to Node.js 22+ and modern browsers.

## Wasm & TypeScript Capabilities

| Class / Method | Crate / Package | Description |
| --- | --- | --- |
| `WasmSimulation` | `kairo-ecs-wasm` | Deterministic DES/ABM execution compiled to `wasm32-unknown-unknown` |
| `WasmTimeTravelInspector` | `kairo-ecs-wasm` | Copy-on-write (`CowWorldSnapshot`) timeline scrubbing, entity inspection, and state diffing (`diff_frames(t1, t2)`) |
| `KairoTimelineScrubber` | `bindings/typescript` | Browser UI controller for step-back, branch-from-tick, and breakpoint evaluation |

## Example

```typescript
import { WasmTimeTravelInspector } from "@kairoecs/wasm";

const inspector = new WasmTimeTravelInspector();
inspector.recordTick(1, [{ entityId: 1, health: 100, infected: false }]);
inspector.recordTick(2, [{ entityId: 1, health: 85, infected: true }]);

const diff = inspector.diffFrames(1, 2);
console.log("Changed entities between tick 1 and 2:", diff);
```

Canonical sources:

- `bindings/typescript/README.md`
- `docs/tutorials/wasm-getting-started.md`
