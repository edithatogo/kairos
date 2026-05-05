# Benchmark Results

GPU benchmark results are not yet available.

The current accepted validation is limited to:

- GPU crate compiles without default GPU dependencies.
- CPU fallback parity harness produces deterministic ABM and DES output.
- WGSL shader source exists for native GPU backend implementation.

Before marking `gpu-speedup-threshold` complete, record:

- Hardware model and driver version.
- Backend and feature flags.
- Agent count.
- CPU baseline timing.
- GPU timing.
- Speedup ratio.
- Peak GPU memory.
