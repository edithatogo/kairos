# WebGPU Comparison

WebGPU and native GPU compute share WGSL kernel design but have different operational constraints. Current Track 33 claims are CPU-fallback-only at runtime until WebGPU bindings and browser dispatch are introduced.

| Dimension | Native GPU | Browser WebGPU |
|---|---|---|
| Device access | Native process owns device context | Browser mediates adapter/device |
| Shader language | WGSL for wgpu, CUDA C/PTX for CUDA | WGSL only |
| Workgroup target | Backend-specific | 256 threads by default |
| Readback | Backend staging buffers | `mapAsync` staging buffers |
| Performance claims | Hardware runner required | Browser, GPU, and version required |

The WebGPU path should prove adoption value and browser parity before claiming native-level throughput.
The adoption/value claim in this slice is therefore limited to cross-browser static validation and explicit fallback-to-cpu boundaries.
