# Test Matrix — 09 TypeScript/Wasm Binding

## Required tests

- `npm ci` to validate the local package install state.
- `npm test` or `pnpm test` for the TypeScript binding surface.
- `npm run typecheck` or `pnpm run typecheck` for the TypeScript binding surface.
- `npm run test:conformance` or equivalent when Track 12 fixtures are wired in.
- `npm pack` to validate package contents before any future registry work.
- `wasm-pack test --node` only if the binding uses wasm-pack as its local validation path.

## Future-surface controls

- Do not add npm publishing, provenance signing, or registry credentials here.
- Do not expand into other language bindings or release-engineering surfaces.
- Do not widen the track beyond TypeScript/Wasm adapter validation.
- Stop at local bundle and fixture validation until Track 12 owns parity and Track 15 owns package dry-runs.

- `wasmtime` smoke test: load `kairo-ecs-wasm` module on `wasm32-wasip2` target and run a 10K event simulation.

## CI command

```bash
npm ci && npm run typecheck && npm test
```

