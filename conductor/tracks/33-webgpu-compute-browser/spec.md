# Track 33: WebGPU Compute for Browser

## Purpose

Run GPU-accelerated simulation directly in the browser via WebGPU compute shaders, paired with the Track 09 TypeScript/Wasm binding layer. This delivers a unique state-of-the-art differentiator: high-performance ABM and DES simulation running on the user's own GPU within a browser tab, without server round-trips or native installation.

## Why this track exists

Browser-based simulation is a fast path to adoption — users can try KairoECS without installing anything. Track 09 produces a Wasm module with CPU-only computation. WebGPU unlocks orders of magnitude more parallelism by dispatching compute shaders from the browser to the underlying GPU (Metal, Vulkan, DX12 via the WebGPU API). Sharing kernel designs with Track 32 (native GPU compute) maximizes leverage and reduces duplicated effort.

## Primary subagent

`webgpu-agent`

## Parallelization model

This track depends on Track 09 (TypeScript/Wasm binding) for the Wasm module scaffold and Track 32 (GPU compute) for the shared kernel intermediate representation. It does not modify any native crates — all WebGPU-specific code lives in `crates/kairo-ecs-webgpu/` and `website/webgpu-demo/`.

## Inputs

- `crates/kairo-ecs-wasm/` — Wasm module and TypeScript binding layer from Track 09.
- `docs/gpu-compute/kernel-ir.md` — shared kernel IR from Track 32.
- `crates/kairo-ecs-gpu/src/shaders/` — WGSL shader designs from Track 32 (adapt to WebGPU WGSL subset).
- Track 05 visualization snapshot contract for live in-browser rendering.

## Outputs

- `crates/kairo-ecs-webgpu/` — Rust crate that currently provides dependency-free WebGPU contracts and should later produce a WebGPU compute Wasm module.
- `website/webgpu-demo/` — browser demo page that currently runs CPU fallback animation and labels WebGPU dispatch as backend-not-configured until real device setup lands.
- Performance comparison: WebGPU Wasm vs CPU Wasm for ABM particle update, only after a real browser WebGPU backend and reference hardware run exist.
- Cross-browser smoke test results (Chrome, Edge, Firefox Nightly), only after browser automation and WebGPU flags/runners are available.

## Owned paths

- `crates/kairo-ecs-webgpu/`
- `website/webgpu-demo/`
- `conductor/tracks/33-webgpu-compute-browser/`

## Blocked paths

- `crates/kairo-ecs-wasm/` — owned by Track 09.
- `crates/kairo-ecs-gpu/` — owned by Track 32 (read-only reference for kernel IR).
- `crates/kairo-ecs-core/` — owned by Track 01.

## Acceptance criteria

1. Browser demo runs 100K-agent ABM simulation at >=30 frames per second on a consumer GPU (RTX 3060-class or equivalent).
2. WebGPU compute path produces simulation output identical to the CPU Wasm path for the same random seed.
3. Cross-browser smoke test passes: simulation launches and runs for 30 seconds without crash on Chrome (stable), Edge (stable), and Firefox Nightly.
4. Performance comparison table published in `website/webgpu-demo/` showing WebGPU speedup vs CPU Wasm.
5. The `kairo-ecs-webgpu` crate compiles to Wasm without native-only dependencies (`wasm-pack build` succeeds).

## Release implications

- Non-blocking for headless release. WebGPU is a browser-only feature.
- Gated behind browser WebGPU availability — the demo page detects WebGPU support and falls back gracefully to CPU Wasm.
- WebGPU is still evolving in browser implementations. The track targets the W3C WebGPU 1.0 specification but must handle API churn in pre-stable browsers.

## Non-goals

- Supporting WebGL as a fallback (covered by CPU Wasm path).
- Running WebGPU compute outside the browser (covered by Track 32 native GPU).
- Full feature parity with native GPU path — browser GPU resources are more constrained.
- Mobile browser support for WebGPU in the initial scope.
- GPU-accelerated rendering — the viz layer (Track 05) handles rendering; this track provides compute.

## Quality gates

Use the gates in `conductor/quality-gates.md`. Track-specific gates:
- `webgpu-crate-compiles` — `kairo-ecs-webgpu` compiles to Wasm via `wasm-pack`.
- `webgpu-demo-loads` — the demo page loads in a browser and detects WebGPU availability.
- `webgpu-cpu-parity` — WebGPU compute output matches CPU Wasm output for same seed.
- `webgpu-framerate` — demo maintains >=30fps for 100K-agent ABM on reference hardware after real WebGPU dispatch is configured.
- `webgpu-cross-browser` — smoke test passes on Chrome, Edge, Firefox Nightly.
