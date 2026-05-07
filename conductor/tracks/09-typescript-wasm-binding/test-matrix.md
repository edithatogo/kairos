# Test Matrix — 09 TypeScript/Wasm Binding

## Required tests

- `npm ci` to validate the local package install state. Last run: pass, 2026-05-07.
- `npm run typecheck` for the TypeScript binding surface. Last run: pass, 2026-05-07.
- `npm test` for scheduler/event-log/native-loader contracts. Last run: pass, 2026-05-07.
- `npm run build` to validate generated JS and declarations. Last run: pass, 2026-05-07.
- `npm pack --dry-run` to validate package contents before any future registry work. Last run: pass with npm cache/log write access, 2026-05-07.
- `cargo fmt --package kairo-ecs-wasm --check` for the Rust wasm wrapper. Last run: pass, 2026-05-07.
- `cargo check --manifest-path crates/kairo-ecs-wasm/Cargo.toml` for the default Rust wasm wrapper contract. Last run: pass, 2026-05-07.
- `cargo test --manifest-path crates/kairo-ecs-wasm/Cargo.toml` for the default Rust wasm wrapper contract. Last run: pass, 2026-05-07.
- `cargo check --manifest-path crates/kairo-ecs-wasm/Cargo.toml --features wasm-export` for the generated `wasm-bindgen` export layer. Last run: blocked, 2026-05-07, because build scripts for `wasm-bindgen` dependencies hit Git's `usr\bin\link.exe` and fail with Win32 error 5.
- `npm run test:conformance` or equivalent when Track 12 fixtures are wired in.
- `wasm-pack test --node` only once the `wasm-export` feature can compile on the local runner.
- `node tests/conformance/track07_13_hardening_check.mjs` verifies this track no longer claims package publishing ownership or unimplemented server runtime support.
- `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\06-python-binding-310-314\validate-bindings06-11.ps1` verifies TypeScript cancellation parity, package metadata, and no native-runtime overclaiming. Last run: pass, 2026-05-08.

## Future-surface controls

- Do not add npm publishing, provenance signing, or registry credentials here.
- Do not expand into other language bindings or release-engineering surfaces.
- Do not widen the track beyond TypeScript/Wasm adapter validation.
- Stop at local bundle and fixture validation until Track 12 owns parity and Track 15 owns package dry-runs.

## Focused local validation

- Keep the checked-in validation to `npm` scripts, `cargo check`, and the Track 07-13 hardening check until a runtime runner is added in a later track.
- The TypeScript scheduler facade now exposes `cancel(eventId)` and preserves cancelled events in snapshots and event-log rows.
- The TypeScript event-log payload now includes Track 04 schema version, field metadata, 12-byte little-endian handle hex, and 16-byte little-endian tick hex.

## CI command

```bash
npm ci && npm run typecheck && npm test && npm run build && npm pack --dry-run
```

```bash
cargo check --manifest-path crates/kairo-ecs-wasm/Cargo.toml
cargo test --manifest-path crates/kairo-ecs-wasm/Cargo.toml
```

