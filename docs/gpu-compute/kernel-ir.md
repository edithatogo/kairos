# Kernel IR

The GPU kernel IR is a small contract shared by native GPU and WebGPU code. It is not a serialized file format yet; it is the set of operations that kernel authors may rely on when translating CPU simulation semantics to compute shaders.

## Dispatch

- Kernels process flat buffers with one logical item per global invocation.
- Default workgroup size is 256 threads to stay compatible with WebGPU browser limits and most native backends.
- Dispatch count is `ceil(item_count / workgroup_size)`.

## Buffers

- Simulation state is passed as structure-of-arrays or packed fixed-layout arrays.
- Host allocates and owns buffers.
- GPU backends borrow buffers during compute and return updated buffers through explicit readback.
- Zero-copy is an optimization, not a semantic requirement.

## Determinism

- Kernels must accept explicit seeds.
- Random values use a PCG-family integer hash that can be implemented in Rust, WGSL, and CUDA C.
- DES kernels must document whether events are applied in total timestamp order or in commutative batches.

## Supported operations

- Element-wise particle updates.
- Bounded reductions with deterministic ordering.
- Atomic integer accumulation for commutative DES event effects.
- Buffer copy, staging upload, and staging readback.

## Excluded from the initial IR

- 64-bit atomics.
- Subgroup operations.
- Device-specific push constants.
- Dynamic allocation inside a shader.
