# Handoff: Track 33 WebGPU Compute for Browser

Last updated: 2026-05-11

Update tag: `host-only-next-harvest-slice`

## Status

Initial scaffold implemented and tightened. Browser-native WebGPU device wiring is still blocked, and the crate now reports that explicitly through `BrowserBindingsNotConfigured` / `BrowserBackendNotConfigured` contracts. The crate facade, adapter/bridge/reference-dispatch scaffolds, fallback/parity capability metadata, WebGPU WGSL shader, static demo, smoke test, GPU-free WGSL subset validator, and comparison/subset docs exist.

## Summary

Track 33 is building toward GPU-accelerated simulation in the browser via WebGPU compute shaders, paired with the Track 09 Wasm binding layer. The current `kairo-ecs-webgpu` crate is dependency-free and exposes adapter, bridge, reference dispatch, fallback/parity metadata, and explicit backend-unavailable contracts. The browser demo at `website/webgpu-demo/` runs a CPU fallback animation, detects the WebGPU API, and labels real WebGPU dispatch as `backend-not-configured` until Wasm bindings and device setup land. No browser GPU dispatch or 30fps/100K-agent performance claim is made by the current artifacts.

This slice adds an explicit "host-only next-harvest" control layer:

- The track plan now names host-only maintenance tasks.
- The track validator (`validate-track33.ps1`) now enforces contract strings that prevent accidental runtime performance claims in offline artifacts.
- The demo and track matrix now record fallback-boundary status as complete and keep runtime tests and cross-browser checks blocked until host requirements and browser hardware are available.

## Files created in this track

- `conductor/tracks/33-webgpu-compute-browser/spec.md`
- `conductor/tracks/33-webgpu-compute-browser/plan.md`
- `conductor/tracks/33-webgpu-compute-browser/agent-contract.md`
- `conductor/tracks/33-webgpu-compute-browser/risk-register.md`
- `conductor/tracks/33-webgpu-compute-browser/test-matrix.md`
- `conductor/tracks/33-webgpu-compute-browser/handoff.md`
- `crates/kairo-ecs-webgpu/Cargo.toml`
- `crates/kairo-ecs-webgpu/src/lib.rs`
- `crates/kairo-ecs-webgpu/src/adapter.rs`
- `crates/kairo-ecs-webgpu/src/bridge.rs`
- `crates/kairo-ecs-webgpu/src/capability.rs`
- `crates/kairo-ecs-webgpu/src/dispatch.rs`
- `crates/kairo-ecs-webgpu/src/shaders/abm_webgpu.wgsl`
- `crates/kairo-ecs-webgpu/tests/parity_webgpu.rs`
- `website/webgpu-demo/index.html`
- `website/webgpu-demo/styles.css`
- `website/webgpu-demo/src/main.js`
- `website/webgpu-demo/scripts/smoke.mjs`
- `website/webgpu-demo/scripts/validate-wgsl-subset.mjs`
- `website/webgpu-demo/package.json`
- `website/webgpu-demo/README.md`
- `docs/gpu-compute/webgpu-wgsl-subset.md`
- `docs/gpu-compute/webgpu-comparison.md`

## Contracts consumed

- Track 09 — `crates/kairo-ecs-wasm/` (read-only, consumed for Wasm module scaffold and JavaScript API).
- Track 32 — `docs/gpu-compute/kernel-ir.md`, `crates/kairo-ecs-gpu/src/shaders/` (read-only, consumed for shared kernel design).
- Track 05 — visualization snapshot contract (read-only, consumed for in-browser rendering integration).
- Track 01 — core scheduler semantics (read-only, consumed via Wasm module API from Track 09).

## Contracts produced

- `crates/kairo-ecs-webgpu/` — dependency-free default WebGPU facade with adapter, bridge, fallback/parity metadata, reference dispatch, and backend-not-configured contracts.
- `website/webgpu-demo/` — static browser demo with WebGPU API detection, CPU fallback animation, backend-not-configured dispatch label, controls, metric panels, and GPU-free static validation.
- `docs/gpu-compute/webgpu-wgsl-subset.md` — WebGPU WGSL feature restrictions and local validator contract.
- `docs/gpu-compute/webgpu-comparison.md` — WebGPU vs native GPU comparison.

## Validation

- Passed: `cargo check --manifest-path crates/kairo-ecs-webgpu/Cargo.toml --no-default-features`
- Passed: `cargo check --manifest-path crates/kairo-ecs-webgpu/Cargo.toml --features webgpu --tests`
- Passed after formatting: `cargo fmt --manifest-path crates/kairo-ecs-webgpu/Cargo.toml`
- Passed: `npm test --prefix website/webgpu-demo`
- Passed: `npm run validate:wgsl --prefix website/webgpu-demo`
- Optional runtime gate: `cargo test --manifest-path crates/kairo-ecs-webgpu/Cargo.toml` remains host-dependent until a working browser/WebGPU runtime and linker path are available; the current slice keeps compile-only validation as the default.

## Release gates affected

- **browser-webgpu-smoke** — current central scaffold gate for the static demo and GPU-free WGSL subset. Blocking for PRs touching the demo.
- **wasm-gpu-parity** — current central compile-time parity boundary for the WebGPU crate's CPU fallback and not-configured dispatch behavior. Blocking for WebGPU kernel PRs.
- **webgpu-crate-compiles** / **webgpu-demo-loads** / **webgpu-cpu-parity** — staged runtime targets preserved in the track spec for future browser-backed evidence. Not yet available in this workspace.
- **webgpu-framerate** — demo maintains >=30fps for 100K agents after real browser WebGPU dispatch is configured. Informational only; becomes blocking at RC.
- **webgpu-cross-browser** — smoke test passes on Chrome, Edge, Firefox Nightly. Informational only; becomes blocking at RC.

All WebGPU gates require headless Chrome with `--enable-unsafe-webgpu` flag. Gates are informational when no GPU is present in CI.

## Risks and unresolved questions

- WebGPU browser availability is the primary risk. Chrome stable has WebGPU; Firefox and Safari gate it behind flags. The demo must gracefully degrade to CPU Wasm and communicate browser requirements clearly.
- The WebGPU WGSL subset differs from native WGSL. Features like subgroup operations, 64-bit atomics, and push constants may be unavailable. The shared kernel IR from Track 32 must account for this lowest common denominator.
- Wasm-to-WebGPU buffer sharing is still being optimized in browser engines. Zero-copy import of Wasm memory as `GPUBuffer` may not be available in all browsers, forcing a copy path that adds latency.
- Headless CI testing for WebGPU requires Chrome flags that may break between Chrome versions. The CI setup needs ongoing maintenance.
- Firefox and Safari WebGPU timelines are uncertain. The track ships as Chrome-primary, with Edge and Firefox Nightly as secondary targets. Full cross-browser support may not be achievable by 1.0.
- Browser GPU resource constraints are more severe than native — shared VRAM with the compositor, tab-level memory limits, and thermal throttling. The demo must be conservative with agent counts and buffer sizes.
- Low-cost smoke routes are documented in `docs/gpu-compute/free-testing-routes.md`: use GitHub-hosted macOS runners or the M1 MacBook Pro for Metal-adjacent browser/device smoke, and Colab TPU only for future TPU-specific prototypes. These routes do not satisfy browser WebGPU parity or frame-rate evidence.

### Host-only evidence gates added in this slice

- Contract-boundary checks in `validate-track33.ps1` now require explicit fallback language in demo/readme and forbid unverified performance wording in the WebGPU comparison doc.
- `test-matrix.md` now includes host-only contract artifact checks for this track-specific boundary.
- `plan.md` now contains a dedicated host-only next-harvest stage so process-owned artifacts are aligned with current execution constraints.

## Files changed

No additional file list was recorded by this Conductor hygiene update. Use the track plan, spec, and git history for implementation-specific file evidence.


## Contracts changed

No contract changes were recorded by this Conductor hygiene update.


## Tests added

No tests were added by this Conductor hygiene update.


## Known risks

No new risks were introduced by this Conductor hygiene update.


## Follow-up issues

No additional follow-up issues were recorded by this Conductor hygiene update.


## Integration notes

No additional integration notes were recorded by this Conductor hygiene update.
## Phase closeout evidence

Track 33 is not cleanly closable yet. The offline validator passed on 2026-05-10 (`pwsh -NoProfile -File conductor/tracks/33-webgpu-compute-browser/validate-track33.ps1`), and the current test matrix shows the crate, demo, WGSL, and no-performance-claim boundaries are complete. The remaining blockers are still live-browser items:

- `wasm-pack build --target web` is blocked until the Track 09 Wasm binding package exists and the `wasm-bindgen` / `web-sys` wiring can be introduced and fetched.
- Browser WebGPU device initialization is blocked until the browser binding dependency and headless Chrome WebGPU validation are available.
- The `100K-agent >=30 FPS` claim is blocked until a real WebGPU backend and reference hardware/browser results exist.
- Cross-browser smoke coverage is blocked until a browser test harness and the installed target browsers are available.

Keep this track `In Review` until those blockers are resolved. Record `$conductor-review` findings, accepted fixes, deferred or blocked fixes, validation commands, cleanup state, commit SHA or explicit push blocker, pushed ref, strict `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` result, and next-phase decision here when the track becomes eligible for closeout.
