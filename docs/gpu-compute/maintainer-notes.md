# Maintainer Notes

## Adding a kernel

1. Add the CPU reference behavior to the parity harness.
2. Add WGSL under `crates/kairo-ecs-gpu/src/shaders/`.
3. Add a WebGPU-safe variant under `crates/kairo-ecs-webgpu/src/shaders/` if browser execution is planned.
4. Add tests that compare the backend output to the CPU reference with a fixed seed.

## Updating the IR

Update `kernel-ir.md` first, then update native and WebGPU shader docs. Do not mark a matrix item complete until the artifact and validation command are recorded.
