# Test Matrix — 09 TypeScript/Wasm Binding

## Required tests

- `npm ci` to validate the local package install state. Last run: pass, 2026-05-08.
- `npm run typecheck` for the TypeScript binding surface using `tsc --noEmit`. Last run: pass, 2026-05-08.
- `npm test` for scheduler/event-log/native-loader contracts and fixture bridge tests using Vitest. Last run: pass, 2026-05-08.
- `npm run test:browser` to build the package and smoke the browser ESM bundle in headless Chromium. Last run: pass with browser-launch approval, 2026-05-08.
- `npm run build` to validate generated JS and declarations. Last run: pass, 2026-05-08.
- `npm pack --dry-run` to validate package contents before any future registry work. Last run: pass with npm cache/log write access, 2026-05-08.
- `cargo fmt --package kairo-ecs-wasm --check` for the Rust wasm wrapper. Last run: pass, 2026-05-08.
- `cargo check --manifest-path crates/kairo-ecs-wasm/Cargo.toml` for the default Rust wasm wrapper contract. Last run: pass, 2026-05-08.
- `cargo +stable-x86_64-pc-windows-gnu test --manifest-path crates/kairo-ecs-wasm/Cargo.toml` for the default Rust wasm wrapper contract. Last run: pass, 2026-05-08, 3 unit tests and 0 doctests.
- `cargo check --manifest-path crates/kairo-ecs-wasm/Cargo.toml --features wasm-export` for the generated `wasm-bindgen` export layer. Last run: blocked, 2026-05-07, because build scripts for `wasm-bindgen` dependencies hit Git's `usr\bin\link.exe` and fail with Win32 error 5.
- The Wasm export crate declares Rust 1.77, matching the current `wasm-bindgen` MSRV. Required CI checks the locked `wasm32-unknown-unknown` target with `wasm-export` on Rust 1.77.
- Track 12 fixture bridge is covered by `npm test` through `test/conformance.test.ts`.
- `wasm-pack test --node` only once the `wasm-export` feature can compile on the local runner.
- `node tests/conformance/track07_13_hardening_check.mjs` verifies this track no longer claims package publishing ownership or unimplemented server runtime support.
- `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\06-python-binding-310-314\validate-bindings06-11.ps1` verifies TypeScript cancellation parity, package metadata, and no native-runtime overclaiming. Last run: pass, 2026-05-08.

## Future-surface controls

- Do not add npm publishing, provenance signing, or registry credentials here.
- Do not expand into other language bindings or release-engineering surfaces.
- Do not widen the track beyond TypeScript/Wasm adapter validation.
- Stop at local bundle and fixture validation until Track 12 owns parity and Track 15 owns package dry-runs, with Track 42 owning publication.

## Focused local validation

- Keep the checked-in validation to `npm` scripts, `cargo check`, and the Track 07-13 hardening check until a runtime runner is added in a later track.
- The TypeScript scheduler facade now exposes `cancel(eventId)` and preserves cancelled events in snapshots and event-log rows.
- The TypeScript event-log payload now includes Track 04 schema version, field metadata, 12-byte little-endian handle hex, and 16-byte little-endian tick hex.
- Performance-sensitive scheduler cancellation changes can be compared locally without adding timing-sensitive CI assertions. Run `node --experimental-strip-types bindings/typescript/bench/scheduler-cancel.mjs --base <baseline-commit> --events 20000 --cancellations 10000 --repetitions 3` on a supported Node.js 22–24 runtime; the benchmark records exact raw timings, medians, source commits, and post-state checks while timing only the cancellation loop. Fetch the intended base ref first; the command compares its TypeScript source to the checked-out candidate.
- The scheduler cancellation benchmark is intentionally manual rather than part of hosted CI: machine contention makes timing assertions unsuitable for stable PR gates. The 2026-09-30 Node 22.23.3 run and raw JSON are retained at `bindings/typescript/bench/results-node22-20k.json`.

## CI command

```bash
npm ci && npm run typecheck && npm test && npm run test:browser && npm run build && npm pack --dry-run
```

```bash
cargo check --manifest-path crates/kairo-ecs-wasm/Cargo.toml
cargo +stable-x86_64-pc-windows-gnu test --manifest-path crates/kairo-ecs-wasm/Cargo.toml
```
## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, phase-closeout ledger evidence, cleaned commit/push evidence, and blocker recording. At actual closeout, run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.
