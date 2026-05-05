# WebGPU WGSL Subset

Track 33 uses the same kernel IR as Track 32 but restricts shaders to browser-compatible WGSL.

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
