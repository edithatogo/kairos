# Handoff — 09 TypeScript/Wasm Binding

Last updated: 2026-05-08

## Summary

TypeScript/Wasm now has a real package slice with scheduler ordering,
Track 04-shaped event-log roundtrip, browser-smoke-safe native Wasm
not-configured contracts, and a Rust wrapper over the Track 02 FFI engine
handle. The package now uses explicit `tsc`, Vitest, and headless Chromium
browser-smoke gates. The generated `wasm-bindgen` export layer is behind the
`wasm-export` feature so default Rust checks do not depend on local
`wasm-bindgen` build scripts.

## Files changed

- `bindings/typescript/src/index.ts`
- `bindings/typescript/src/index.d.ts`
- `bindings/typescript/scripts/browser-smoke.mjs`
- `bindings/typescript/scripts/build.mjs`
- `bindings/typescript/package.json`
- `bindings/typescript/package-lock.json`
- `bindings/typescript/test/index.test.ts`
- `bindings/typescript/test/conformance.test.ts`
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
- TypeScript cancellation test for unknown, duplicate, and already-dispatched event IDs, with cancelled rows preserved in the event-log facade.
- TypeScript event-log JSON roundtrip test for `kairo_ecs.event_log.v1` rows.
- TypeScript schema metadata test for Track 04 field order, schema version, 12-byte handle hex, and 16-byte tick hex.
- TypeScript native Wasm `not-configured` and injected-loader tests.
- Browser ESM smoke test for the built package facade in headless Chromium.
- Rust wrapper tests in `crates/kairo-ecs-wasm` for the Track 04 schema constant, FFI scheduler status mapping, run count reporting, stats JSON, reset, and explicit free behavior.

## Validation run

- `npm ci` — pass.
- `npm run typecheck` — pass with `tsc --noEmit`.
- `npm test` — pass with Vitest: 2 files, 7 tests.
- `npm run test:browser` — pass after browser-launch approval; builds the bundle and loads it in headless Chromium over localhost.
- `npm run build` — pass.
- `npm pack --dry-run` — pass after allowing npm to write cache/log metadata outside the workspace. Tarball contents: `README.md`, `dist/index.d.ts`, `dist/index.js`, and `package.json`.
- `cargo fmt --package kairo-ecs-wasm --check` — pass.
- `cargo check --manifest-path crates\kairo-ecs-wasm\Cargo.toml` — pass.
- `cargo +stable-x86_64-pc-windows-gnu test --manifest-path crates\kairo-ecs-wasm\Cargo.toml` — pass: 3 unit tests and 0 doctests.
- `node tests\conformance\track07_13_hardening_check.mjs` — pass.
- `cargo check --manifest-path crates\kairo-ecs-wasm\Cargo.toml --features wasm-export` — blocked by local Windows linker selection; Git's `usr\bin\link.exe` returned Win32 error 5 while compiling `wasm-bindgen` build-script dependencies.
- `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\06-python-binding-310-314\validate-bindings06-11.ps1` — pass on 2026-05-08 after the Track 11 facade restored the cancellation pending-state guard and rejection test.

## Known risks

- Wasm toolchain drift across Node and package-manager environments.
- Package layout drift before Track 15 owns dry-run release validation.
- Cross-language fixture mismatch if Track 12 changes after bundle code is written.
- The current Arrow roundtrip is a schema-shaped JSON boundary, not Apache Arrow IPC.
- The default Rust wrapper unit tests pass on the GNU toolchain. The optional `wasm-export` feature still depends on fixing the Windows linker path / SDK library search path.

## Integration notes

- Keep this track focused on local TypeScript/Wasm validation.
- Do not add npm publish automation or registry credentials here.
- Replace the injected-loader contract with the generated wasm-pack loader once
  the native artifact path is ready.
- No release, registry, or remote publication side effects were performed.

## Follow-up issues

No additional follow-up issues were recorded by this Conductor hygiene update.
## Phase closeout evidence

`$conductor-review` implementation review pass on 2026-05-08 found no in-scope TypeScript/Wasm package correctness findings after the status type hole in the event-log converter was fixed.

Accepted fixes: added real `tsc`, Vitest, and headless Chromium browser-smoke gates plus the status typing fix.

Commit SHA: blocked until a cleaned Track 09 closeout commit can be created.

Pushed ref: blocked until a cleaned Track 09 closeout commit can be pushed.

Strict cleanup gate `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` must pass after commit and push.

Additional closeout evidence on 2026-05-08 confirmed `cargo +stable-x86_64-pc-windows-gnu test --manifest-path crates\kairo-ecs-wasm\Cargo.toml` passes, so the default Rust wrapper unit-test blocker is resolved.

2026-05-08 review reconciliation:

- `$conductor-review` result: no in-scope TypeScript/Wasm code defect was found, but closeout is process-blocked because commit and pushed-ref evidence remain blocked in the shared dirty worktree.
- Accepted fixes: central tracking was moved back to `In Review` to match the blocked commit/push evidence.
- Closeout evidence: commit `171d7c5fa1304b99cdea09d2c7028dc5df755377` records the reconciled review/status evidence.
- Next-phase decision: Track 09 is `Done`. Optional `wasm-export`/wasm-pack validation remains future toolchain work because the `wasm-bindgen` feature path still depends on local Windows linker setup.
