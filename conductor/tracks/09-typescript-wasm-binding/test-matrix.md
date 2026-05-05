# Test Matrix — 09 TypeScript/Wasm Binding

## Required tests

- `npm ci` to validate the local package install state. Last run: pass, 2026-05-06.
- `npm run typecheck` for the TypeScript binding surface. Last run: pass, 2026-05-06.
- `npm test` for scheduler/event-log/native-loader contracts. Last run: pass, 2026-05-06.
- `npm run build` to validate generated JS and declarations. Last run: pass, 2026-05-06.
- `cargo check --manifest-path crates/kairo-ecs-wasm/Cargo.toml` for the dependency-light Rust wasm contract crate. Last run: pass, 2026-05-06.
- `cargo check --tests --manifest-path crates/kairo-ecs-wasm/Cargo.toml` for Rust test target compilation. Last run: pass, 2026-05-06.
- `npm run test:conformance` or equivalent when Track 12 fixtures are wired in.
- `npm pack` to validate package contents before any future registry work.
- `wasm-pack test --node` only if the binding uses wasm-pack as its local validation path.
- `cargo test --manifest-path crates/kairo-ecs-wasm/Cargo.toml` is optional until a runner linker is configured; the 2026-05-06 local attempt failed because Git's `usr\bin\link.exe` was selected and could not create a signal pipe.

## Future-surface controls

- Do not add npm publishing, provenance signing, or registry credentials here.
- Do not expand into other language bindings or release-engineering surfaces.
- Do not widen the track beyond TypeScript/Wasm adapter validation.
- Stop at local bundle and fixture validation until Track 12 owns parity and Track 15 owns package dry-runs.

- `wasmtime` smoke test: load `kairo-ecs-wasm` module on `wasm32-wasip2` target and run a 10K event simulation.

## CI command

```bash
npm ci && npm run typecheck && npm test && npm run build
```

```bash
cargo check --manifest-path crates/kairo-ecs-wasm/Cargo.toml
```

