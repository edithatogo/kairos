# Track 33 Plan: WebGPU Compute for Browser

## Phase 0 — Contract alignment with Track 09 and Track 32

### Task 0.1 — Inventory Wasm binding surface
- Document the Track 09 Wasm module's JavaScript API: world creation, step execution, state query.
- Identify the hook point where a WebGPU compute path replaces the CPU Wasm step.
- Confirm that Wasm linear memory layout is compatible with WebGPU buffer binding.

### Task 0.2 — Shared kernel IR review
- Review `docs/gpu-compute/kernel-ir.md` from Track 32.
- Identify WGSL features used in native shaders that are unavailable or restricted in WebGPU WGSL (e.g., subgroup operations, 64-bit atomics, push constants).
- Document the WebGPU WGSL subset and any required shader adaptations in `docs/gpu-compute/webgpu-wgsl-subset.md`.

### Task 0.3 — Lock the owned surface
- All new code lives in `crates/kairo-ecs-webgpu/` and `website/webgpu-demo/`.
- Do not modify `crates/kairo-ecs-wasm/`, `crates/kairo-ecs-gpu/`, or any core/state crates.
- WebGPU features gated behind browser detection — no server-side dependency on GPU hardware.

## Phase 1 — Scaffold kairo-ecs-webgpu crate

### Task 1.1 — Crate skeleton
- `crates/kairo-ecs-webgpu/Cargo.toml` with `wasm-bindgen`, `web-sys` (WebGPU bindings), and `wgpu` (for shader module compilation at build time).
- `src/lib.rs` with Wasm-bindgen exports for `init_webgpu`, `run_webgpu_step`, `get_result_buffer`.
- Feature flag `webgpu` that gates browser-specific bindings.

### Task 1.2 — WebGPU adapter initialization
- `src/adapter.rs` — detect WebGPU availability via `navigator.gpu`, request adapter and device, configure buffer bindings.
- Graceful fallback: if WebGPU is unavailable, export an `is_webgpu_available()` function that returns `false`.

### Task 1.3 — Buffer bridge with Wasm
- `src/bridge.rs` — map Wasm linear memory into WebGPU buffer bindings.
- Handle alignment constraints (WGSL `std430` vs Rust struct layout).
- Implement typed buffer descriptors matching the native GPU buffer layer from Track 32.

## Phase 2 — Implement WebGPU compute kernels

### Task 2.1 — Adapt ABM particle kernel for WebGPU WGSL
- Port the ABM kernel from `crates/kairo-ecs-gpu/src/shaders/` to WebGPU-compatible WGSL.
- Adjust workgroup sizes for browser GPU limits (typically 256 threads, not 1024).
- Implement PCG-family RNG in WebGPU WGSL.
- Unit test via `wasm-bindgen-test` in headless browser (using `wasm-pack test --headless`).

### Task 2.2 — Dispatch orchestration
- `src/dispatch.rs` — split work into workgroups, dispatch compute pass, read back results.
- Handle staging buffer for readback (MapAsync pattern in WebGPU).
- Implement double-buffering for overlapping compute and readback.

### Task 2.3 — Parity test against CPU Wasm
- `tests/parity_webgpu.rs` — run fixed-seed ABM step via both CPU Wasm and WebGPU compute, assert identical output.
- Run in headless Chrome via `wasm-pack test --chrome --headless`.

## Phase 3 — Build browser demo page

### Task 3.1 — Demo scaffolding
- `website/webgpu-demo/index.html` — single-page demo with canvas for viz, controls for agent count (10K, 50K, 100K, 500K), and backend toggle (WebGPU / CPU Wasm).
- `website/webgpu-demo/` directory with build script (`package.json`, `webpack` or `vite` config).

### Task 3.2 — Integration with visualization
- Wire the WebGPU compute output into the Track 05 visualization contract (snapshot format for in-browser rendering).
- Render agent positions using Canvas 2D or WebGL as a lightweight viz layer — no dependency on the full viz crate.

### Task 3.3 — Performance instrumentation
- FPS counter, agent count, and backend indicator displayed in demo UI.
- Timestamp-based profiling: breakdown of upload, compute, readback, and render phases.
- Performance comparison table generated from browser dev tools and published to the demo page.

## Phase 4 — Cross-track integration

### Task 4.1 — CI gate setup
- Add `webgpu-crate-compiles` gate to `conductor/quality-gates.md`.
- Add `webgpu-demo-loads` gate (uses headless Chrome with WebGPU flag).
- Add `webgpu-cpu-parity` gate (runs parity test in headless Chrome).
- Add `webgpu-cross-browser` gate (runs smoke test across browsers via Playwright or Puppeteer).

### Task 4.2 — Documentation
- `website/webgpu-demo/README.md` — how to run, browser requirements, interpretation of results.
- `docs/gpu-compute/webgpu-comparison.md` — WebGPU vs native GPU tradeoffs, feature gaps.
- Update Track 32 benchmark results with WebGPU column.

### Task 4.3 — Cross-track communication
- Hand off to Track 09 (Wasm) for WebGPU-aware Wasm module loading.
- Hand off to Track 05 (viz) for browser rendering contract alignment.
- Notify Track 32 (native GPU) of any kernel IR changes needed for WebGPU compatibility.
- Notify Track 15 (packaging) that WebGPU demo is website-only, not part of headless release.

## Phase 5 — Handoff and closeout

### Task 5.1 — Prepare maintainer notes
- How to test WebGPU in headless Chrome.
- How to update WGSL shaders for new WebGPU spec revisions.
- How to regenerate the demo build after changing Wasm bindings.
- Browser compatibility checklist — which API features work on which browser version.

### Task 5.2 — Update the risk register
- Mark resolved risks as mitigated.
- Escalate any browser that fails the cross-browser smoke test.
- Document WGSL features pending browser implementation.

### Task 5.3 — Finalize handoff
- Publish demo URL (GitHub Pages or similar static host).
- Record reference hardware and browser versions used for performance claims.
