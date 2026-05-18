# Kairos WebGPU Demo

This static demo scaffolds the Track 33 browser experience for hosts without browser GPU bindings.
It detects WebGPU API availability, exposes agent-count and backend controls, and runs a dependency-free CPU fallback animation until the Wasm/WebGPU module is integrated.

Current contract:

- Selecting WebGPU reports `backend not configured` when WebGPU API is available but dispatch path is not yet wired.
- Selecting CPU Wasm runs the dependency-free simulation path for now.
- No browser dispatch or framerate claims are made by this demo slice.

Run the local smoke test:

```powershell
npm test --prefix website/webgpu-demo
```

The test is intentionally browser-GPU-free. It checks the static demo contract and validates the local WGSL shader against the current Track 33 subset notes.

Browser validation for the real WebGPU path is blocked until Track 09 provides the Wasm binding package and this track wires browser device initialization.

### Host-only checks

- `npm run smoke --prefix website/webgpu-demo`
  - checks for demo controls, fallback contract symbols, and static backend labeling.
- `npm run validate:wgsl --prefix website/webgpu-demo`
  - validates `crates/kairo-ecs-webgpu/src/shaders/abm_webgpu.wgsl` against the WebGPU subset doc.
- `pwsh -NoProfile -File conductor/tracks/33-webgpu-compute-browser/validate-track33.ps1`
  - performs the Track 33-wide offline gate, including no-regression contract wording checks.
