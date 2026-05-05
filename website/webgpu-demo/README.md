# Kairos WebGPU Demo

This static demo scaffolds the Track 33 browser experience. It detects WebGPU availability, exposes agent-count and backend controls, and runs a CPU fallback animation until the Wasm/WebGPU module is integrated.

Run the local smoke test:

```powershell
npm test --prefix website/webgpu-demo
```

Browser validation for the real WebGPU path is blocked until Track 09 provides the Wasm binding package and this track wires browser device initialization.
