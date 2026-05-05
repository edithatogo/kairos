# GPU Compute Architecture

The GPU path is an accelerator behind a facade rather than a replacement scheduler.

## Flow

1. CPU code builds a flat `GpuState` buffer.
2. The backend uploads buffers through a `TransferPlan`.
3. The backend dispatches ABM or DES kernels.
4. The backend downloads changed buffers.
5. CPU code remains the source of truth for orchestration and validation.

## Ownership

- Host owns source buffers and result buffers.
- GPU backends borrow uploaded buffers during dispatch.
- Backends must not mutate core ECS structures directly.
- All native GPU code lives under `crates/kairo-ecs-gpu/`.

## Feature isolation

The default crate build contains no GPU dependencies. Native GPU backend code is enabled through feature flags and must remain optional.

## Hardware-independent checks

The GPU crate exposes contract helpers that do not need a device:

- `GpuState::footprint()` computes flat upload/download bytes.
- `TRACK32_TARGET_MEMORY_BUDGET` records the current 1 GB device and 2 GB staging-budget smoke target.
- `DispatchShape::for_items()` computes the shared 256-thread workgroup launch shape.
- `GpuBackendCapabilities` makes fallback, wgpu, and CUDA availability explicit.

These checks are not performance evidence. They exist to catch feature leakage,
memory-shape drift, and dispatch-contract drift on CPU-only developer machines
before GPU runner validation is available.
