# Handoff: Track 33 WebGPU Compute for Browser

## Status

Initial scaffold implemented and tightened. Browser-native WebGPU device wiring is still blocked, and the crate now reports that explicitly through `BrowserBindingsNotConfigured` / `BrowserBackendNotConfigured` contracts. The crate facade, adapter/bridge/reference-dispatch scaffolds, WebGPU WGSL shader, static demo, smoke test, and comparison/subset docs exist.

## Summary

Track 33 is building toward GPU-accelerated simulation in the browser via WebGPU compute shaders, paired with the Track 09 Wasm binding layer. The current `kairo-ecs-webgpu` crate is dependency-free and exposes adapter, bridge, reference dispatch, and explicit backend-unavailable contracts. The browser demo at `website/webgpu-demo/` runs a CPU fallback animation, detects the WebGPU API, and labels real WebGPU dispatch as `backend not configured` until Wasm bindings and device setup land. No browser GPU dispatch or 30fps/100K-agent performance claim is made by the current artifacts.

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
- `crates/kairo-ecs-webgpu/src/dispatch.rs`
- `crates/kairo-ecs-webgpu/src/shaders/abm_webgpu.wgsl`
- `crates/kairo-ecs-webgpu/tests/parity_webgpu.rs`
- `website/webgpu-demo/index.html`
- `website/webgpu-demo/styles.css`
- `website/webgpu-demo/src/main.js`
- `website/webgpu-demo/scripts/smoke.mjs`
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

- `crates/kairo-ecs-webgpu/` — dependency-free default WebGPU facade with adapter, bridge, reference dispatch, and backend-not-configured contracts.
- `website/webgpu-demo/` — static browser demo with WebGPU API detection, CPU fallback animation, backend-not-configured dispatch label, controls, and metric panels.
- `docs/gpu-compute/webgpu-wgsl-subset.md` — WebGPU WGSL feature restrictions.
- `docs/gpu-compute/webgpu-comparison.md` — WebGPU vs native GPU comparison.

## Validation

- Passed: `cargo check --manifest-path crates/kairo-ecs-webgpu/Cargo.toml --no-default-features`
- Passed: `cargo check --manifest-path crates/kairo-ecs-webgpu/Cargo.toml --features webgpu --tests`
- Passed after formatting: `cargo fmt --manifest-path crates/kairo-ecs-webgpu/Cargo.toml`
- Passed: `npm test --prefix website/webgpu-demo`
- Blocked: `cargo test --manifest-path crates/kairo-ecs-webgpu/Cargo.toml` because this shell resolves `link.exe` to Git's `usr\bin\link.exe`, which exits with `couldn't create signal pipe, Win32 error 5`; `rust-lld` also lacks Windows SDK import libraries in this environment.

## Release gates affected

- **webgpu-crate-compiles** — WebGPU crate compiles to Wasm. Blocking for PRs touching the WebGPU crate.
- **webgpu-demo-loads** — Demo page loads and detects WebGPU. Blocking for PRs touching the demo.
- **webgpu-cpu-parity** — WebGPU output matches CPU Wasm for same seed. Blocking for WebGPU kernel PRs.
- **webgpu-framerate** — Demo maintains >=30fps for 100K agents after real browser WebGPU dispatch is configured. Informational only; becomes blocking at RC.
- **webgpu-cross-browser** — Smoke test passes on Chrome, Edge, Firefox Nightly. Informational only; becomes blocking at RC.

All WebGPU gates require headless Chrome with `--enable-unsafe-webgpu` flag. Gates are informational when no GPU is present in CI.

## Risks and unresolved questions

- WebGPU browser availability is the primary risk. Chrome stable has WebGPU; Firefox and Safari gate it behind flags. The demo must gracefully degrade to CPU Wasm and communicate browser requirements clearly.
- The WebGPU WGSL subset differs from native WGSL. Features like subgroup operations, 64-bit atomics, and push constants may be unavailable. The shared kernel IR from Track 32 must account for this lowest common denominator.
- Wasm-to-WebGPU buffer sharing is still being optimized in browser engines. Zero-copy import of Wasm memory as `GPUBuffer` may not be available in all browsers, forcing a copy path that adds latency.
- Headless CI testing for WebGPU requires Chrome flags that may break between Chrome versions. The CI setup needs ongoing maintenance.
- Firefox and Safari WebGPU timelines are uncertain. The track ships as Chrome-primary, with Edge and Firefox Nightly as secondary targets. Full cross-browser support may not be achievable by 1.0.
- Browser GPU resource constraints are more severe than native — shared VRAM with the compositor, tab-level memory limits, and thermal throttling. The demo must be conservative with agent counts and buffer sizes.
