# Agent Contract: webgpu-agent

## Track

Track 33: WebGPU Compute for Browser

## Owned paths

- `conductor/tracks/33-webgpu-compute-browser/`
- `crates/kairo-ecs-webgpu/`
- `website/webgpu-demo/`
- Track-specific artifacts named in `plan.md`

## Required handoff

- Summary of WebGPU kernel implementations and their relationship to Track 32 native kernels.
- Parity test results: WebGPU compute output vs CPU Wasm output.
- Performance comparison table: WebGPU vs CPU Wasm for 100K-agent ABM.
- Cross-browser smoke test results (Chrome, Edge, Firefox Nightly).
- Demo page URL and instructions for local reproduction.
- Known browser-specific WGSL limitations and workarounds.
- Follow-up items for Wasm (Track 09), viz (Track 05), and native GPU (Track 32) subagents.

## Prohibited changes without ADR

- Modifying `crates/kairo-ecs-wasm/` (owned by Track 09).
- Modifying `crates/kairo-ecs-gpu/` or any native GPU shader (owned by Track 32).
- Modifying `crates/kairo-ecs-core/`, `crates/kairo-ecs-state/`, or `crates/kairo-ecs-types/` (owned by Track 01).
- Introducing dependencies on native-only APIs (e.g., CUDA, Vulkan FFI, platform-specific system calls) into the WebGPU crate.
- Changing the Wasm linear memory layout without coordinating with Track 09.
- Making the demo page require any server-side component — it must work as a static site.
- Publishing WebGPU performance numbers without noting the browser version and GPU hardware.

## Gate contract

### webgpu-crate-compiles
- **Input**: `crates/kairo-ecs-webgpu/` source, `Cargo.toml`.
- **Output**: Pass if `wasm-pack build` succeeds targeting `web`. Fail with compilation errors.
- **Blocking**: Yes for PRs that touch `crates/kairo-ecs-webgpu/`. Informational for other PRs.

### webgpu-demo-loads
- **Input**: Built demo artifacts in `website/webgpu-demo/`, headless Chrome with `--enable-unsafe-webgpu` flag.
- **Output**: Pass if the demo page loads, detects WebGPU availability, and initializes the compute pipeline without JavaScript errors. Fail with console error trace.
- **Blocking**: Yes for PRs that touch `website/webgpu-demo/`. Informational for other PRs.

### webgpu-cpu-parity
- **Input**: Fixed random seed, ABM scenario definition, CPU Wasm path, WebGPU compute path.
- **Output**: Pass if both paths produce identical simulation state after N steps. Fail with first differing agent/component/value.
- **Blocking**: Yes for PRs that touch WebGPU kernel code. Requires headless Chrome with WebGPU.

### webgpu-framerate
- **Input**: 100K-agent ABM scenario, reference GPU (RTX 3060-class), demo page instrumentation.
- **Output**: Pass if average FPS >= 30 over a 30-second run. Fail with actual FPS and breakdown.
- **Blocking**: No for PR merge (informational). Becomes blocking at RC when reference hardware is provisioned.

### webgpu-cross-browser
- **Input**: Built demo, Chrome stable, Edge stable, Firefox Nightly.
- **Output**: Pass if the demo launches and runs for 30 seconds without crash on all three browsers. Fail per browser with error log.
- **Blocking**: No for PR merge (informational). Becomes blocking at RC.
