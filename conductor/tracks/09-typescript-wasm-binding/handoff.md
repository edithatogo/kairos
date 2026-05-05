# Handoff — 09 TypeScript/Wasm Binding

## Summary

TypeScript/Wasm now has a minimal real package slice with scheduler ordering,
event-log roundtrip, and browser-smoke-safe native Wasm not-configured
contracts. The Rust wasm crate is dependency-light and exists as a local
contract crate that can be cargo-checked without `wasm-bindgen`,
`wasm-pack`, or generated artifact assumptions.

## Files changed

- `bindings/typescript/src/index.ts`
- `bindings/typescript/src/index.d.ts`
- `bindings/typescript/scripts/build.mjs`
- `bindings/typescript/package.json`
- `bindings/typescript/test/index.test.ts`
- `bindings/typescript/README.md`
- `bindings/typescript/tsconfig.json`
- `crates/kairo-ecs-wasm/Cargo.toml`
- `crates/kairo-ecs-wasm/Cargo.lock`
- `crates/kairo-ecs-wasm/src/lib.rs`
- `conductor/tracks/09-typescript-wasm-binding/test-matrix.md`
- `conductor/tracks/09-typescript-wasm-binding/handoff.md`

## Contracts consumed

- Track 01 core type and scheduler contracts.
- Track 04 Arrow event-log schema contract.
- Track 02 FFI/native loading contract shape.

## Contracts changed

- TypeScript-facing API surface and Wasm adapter boundaries only.
- No shared contract files were changed.

## Tests added

- TypeScript scheduler ordering test for `(timeTicks, priority, sequence)`.
- TypeScript event-log JSON roundtrip test for `kairo_ecs.event_log.v1` rows.
- TypeScript native Wasm `not-configured` and injected-loader tests.
- Rust unit tests in `crates/kairo-ecs-wasm` for status and event ordering.

## Validation run

- `npm ci` — pass.
- `npm run typecheck` — pass.
- `npm test` — pass.
- `npm run build` — pass.
- `cargo check --manifest-path crates/kairo-ecs-wasm/Cargo.toml` — pass.
- `cargo check --tests --manifest-path crates/kairo-ecs-wasm/Cargo.toml` — pass.
- `cargo test --manifest-path crates/kairo-ecs-wasm/Cargo.toml` — blocked by local Windows linker selection; Git's `usr\bin\link.exe` returned `couldn't create signal pipe, Win32 error 5`.

## Known risks

- Wasm toolchain drift across Node and package-manager environments.
- Package layout drift before Track 15 owns dry-run release validation.
- Cross-language fixture mismatch if Track 12 changes after bundle code is written.
- The current Arrow roundtrip is a schema-shaped JSON boundary, not Apache Arrow IPC.

## Integration notes

- Keep this track focused on local TypeScript/Wasm validation.
- Do not add npm publish automation or registry credentials here.
- Replace the injected-loader contract with a generated wasm-pack loader once the
  native artifact path is ready.
- No release, registry, or remote publication side effects were performed.

