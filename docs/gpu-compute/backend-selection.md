# Backend Selection

KairoECS uses two planned native GPU backend families.

## wgpu

wgpu is the primary cross-platform backend. It maps to Vulkan, Metal, and DX12 and keeps the native path close to the WebGPU browser path.

Use wgpu for:

- Windows, macOS, and Linux support.
- Shared WGSL shader development with Track 33.
- CI smoke tests that do not require CUDA.

## CUDA

CUDA is the planned high-throughput backend for NVIDIA HPC systems.

Use CUDA for:

- Reference performance runs.
- Large ABM workloads where NVIDIA hardware is available.
- Kernel experiments that need CUDA profiling tools.

## Current implementation state

The current crate exposes feature-gated backend modules with explicit unavailable contracts:

- `wgpu-backend` reports `wgpu-backend-not-configured`.
- `cuda-backend` reports `cuda-backend-not-configured`.

Those modules do not silently execute CPU fallback work. The deterministic `CpuFallbackCompute` path is separate and exists for dependency-free validation only. Real device initialization remains blocked until backend dependencies are introduced and GPU runners are available.
