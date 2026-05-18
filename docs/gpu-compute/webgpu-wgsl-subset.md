# WebGPU WGSL Subset

Track 33 uses the same kernel IR as Track 32 but restricts shaders to browser-compatible WGSL.

Validation for this slice is host-only and non-runtime:

- `npm run validate:wgsl --prefix website/webgpu-demo`
- static token checks in `website/webgpu-demo/scripts/validate-wgsl-subset.mjs`
- no local binary WebGPU compile in this host slice

Avoid:

- 64-bit atomics.
- Subgroup operations.
- Push constants.
- Storage buffer layouts that depend on native-only alignment behavior.
- Workgroups larger than 256 threads.

Required:

- Explicit uniform or storage-buffer parameters.
- Bounds checks using the logical item count.
- Deterministic integer RNG compatible with the native WGSL kernel.

Current Track 33 ABM subset:

- `@workgroup_size(256)` is the maximum accepted workgroup size for browser smoke validation.
- Agent storage layout is `vec2<f32>` position plus `vec2<f32>` velocity.
- Dispatch parameters are supplied through a uniform block containing `dt`, `seed`, `count`, and padding.
- The local validator is static and GPU-free: `npm run validate:wgsl --prefix website/webgpu-demo`.
