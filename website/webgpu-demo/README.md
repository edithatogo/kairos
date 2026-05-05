# Kairos WebGPU Demo

This static demo scaffolds the Track 33 browser experience. It detects WebGPU API availability, exposes agent-count and backend controls, and runs a dependency-free CPU fallback animation until the Wasm/WebGPU module is integrated.

Current contract: selecting WebGPU reports `backend not configured` when the browser API is present. The page does not claim browser GPU dispatch or performance until Track 09 Wasm bindings and real WebGPU device setup are wired.

Run the local smoke test:

```powershell
npm test --prefix website/webgpu-demo
```

The test is intentionally browser-GPU-free. It checks the static demo contract and validates the local WGSL shader against the current Track 33 subset notes.

Browser validation for the real WebGPU path is blocked until Track 09 provides the Wasm binding package and this track wires browser device initialization.
