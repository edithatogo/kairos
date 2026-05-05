# TypeScript/Wasm Binding

Track 09 owns this binding surface.

This package currently exposes the browser-smoke-safe TypeScript facade for the
future KairoECS Wasm artifact. Native Wasm loading is an explicit
`not-configured` contract until generated artifacts are wired in.

Current facade:

- scheduler ordering by `(timeTicks, priority, sequence)`;
- event-log rows shaped to `kairo_ecs.event_log.v1`;
- JSON roundtrip guard for the Arrow event-log boundary;
- `nativeWasmStatus()` / `loadNativeWasm()` contracts for native loading.

Package root:

- `package.json`
- `package-lock.json`
- `tsconfig.json`
- `src/index.ts`
- `test/index.test.ts`

Local validation:

- `npm ci`
- `npm run typecheck`
- `npm test`
- `npm run build`
