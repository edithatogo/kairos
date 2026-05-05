# WebGPU Comparison

WebGPU and native GPU compute share WGSL kernel design but have different operational constraints.

| Dimension | Native GPU | Browser WebGPU |
|---|---|---|
| Device access | Native process owns device context | Browser mediates adapter/device |
| Shader language | WGSL for wgpu, CUDA C/PTX for CUDA | WGSL only |
| Workgroup target | Backend-specific | 256 threads by default |
| Readback | Backend staging buffers | `mapAsync` staging buffers |
| Performance claims | Hardware runner required | Browser, GPU, and version required |

The WebGPU path should prove adoption value and browser parity before claiming native-level throughput.
