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
