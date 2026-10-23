# TypeScript/Wasm Binding

Track 09 owns this binding surface.

This package currently exposes the browser-smoke-safe TypeScript facade for the
future KairoECS Wasm artifact. Native Wasm loading is an explicit
`not-configured` contract until generated artifacts are wired in.

Current facade:

- scheduler ordering by `(timeTicks, priority, sequence)`;
- event-log rows shaped to the Track 70 Schema v2 contract
  (`kairo_ecs.event_log.v2`): 14 typed columns with generational-ID
  `event_id_index`/`event_id_gen` and `entity_id_index`/`entity_id_gen`
  pairs and full-range `time_ticks_hi`/`time_ticks_lo` limbs;
- JSON roundtrip guard for the Arrow event-log boundary (only schema-v2
  payloads are accepted);
- `nativeWasmStatus()` / `loadNativeWasm()` contracts for native loading.

**Documented copies and unsupported targets (Track 70 Phase 2, Task 2.3):**

- The facade runs without a native engine, so every event-log projection is
  a **row copy** (plain JavaScript objects), not a zero-copy Arrow buffer
  hand-off; there is no native Arrow C Data Interface capsule bridge in this
  package yet.
- The pure-TypeScript facade cannot observe engine-side ID recycling, so
  event/entity handles are projected with **generation `0`**, explicitly:
  real generational IDs come from the native engine surface (Rust FFI /
  Python binding).
- Entity projections are **null-together** per the v2 contract: both
  `entity_id_index` and `entity_id_gen` are set together or both are null.
- u64 values are projected as decimal strings for JSON precision safety;
  u32 generations use `number`.
- Browser: the facade is browser-safe (no Node APIs); native Wasm loading
  and any populated Arrow export remain **unsupported** until Track 09
  wires generated wasm-pack artifacts.

Package root:

- `package.json`
- `package-lock.json`
- `tsconfig.json`
- `src/index.ts`
- `test/index.test.ts`
- `test/conformance.test.ts`
- `scripts/browser-smoke.mjs`

Local validation:

- `npm ci`
- `npm run typecheck`
- `npm test`
- `npm run test:browser`
- `npm run build`

`npm test` runs Vitest over the Node facade and conformance fixture bridge.
`npm run test:browser` builds the package and launches a headless Chromium smoke
test against the built ESM bundle over a temporary localhost server.
