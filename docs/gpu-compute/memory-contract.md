# GPU Memory and Dispatch Contract

Track 32 can validate memory and dispatch shape without opening a GPU device.
This keeps CPU-only CI useful while real wgpu/CUDA dependencies and hardware
runners are still pending.

## State footprint

`GpuState::footprint()` reports the bytes required for the flat buffers owned by
the GPU facade:

- `particles_bytes` covers `AgentParticle` buffers.
- `entity_values_bytes` covers deterministic DES entity value buffers.
- `total_bytes()` is the upload/download payload size before backend-specific
  alignment or driver allocation overhead.

The Track 32 release target remains a 1 GB device budget for a 10M-entity
scenario. The crate exposes `TRACK32_TARGET_MEMORY_BUDGET` so validators can
smoke-check candidate state layouts before GPU hardware is available.

## Dispatch shape

`DispatchShape::for_items()` maps a flat item count to the shared kernel launch
shape:

- default workgroup size: 256 threads
- workgroup count: ceiling division of item count by 256
- invocation count: checked `u32` item count

The 256-thread default matches the WGSL shader scaffolds and remains compatible
with WebGPU-oriented limits. Backends may choose lower device-specific limits
only if they still report that choice through `GpuBackendCapabilities`.

## Backend capabilities

Every `GpuCompute` implementation must report:

- backend name
- availability status
- maximum workgroup size
- zero-copy borrow support
- unified memory support

Feature-gated native backends currently report `BackendNotConfigured` rather
than falling back to CPU work. The only backend that may execute in CPU-only CI
is `CpuFallbackCompute`.
