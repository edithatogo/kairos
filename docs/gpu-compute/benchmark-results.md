# Benchmark Results

GPU benchmark results are not yet available.

The current accepted validation is limited to:

- GPU crate compiles without default GPU dependencies.
- CPU fallback parity harness produces deterministic ABM and DES output.
- WGSL shader source exists for native GPU backend implementation.
- Feature-gated native backend modules report explicit `*-backend-not-configured` errors until real backend dependencies are introduced.
- Track 52 local CPU parity and persistent-memory contract evidence is recorded
  as scaffold evidence in
  `conductor/hpc-evidence/manifests/track52-local-cpu-parity-scaffold.json`.

Before marking `gpu-speedup-threshold` complete, record:

- Hardware model and driver version.
- Backend and feature flags.
- Agent count.
- CPU baseline timing.
- GPU timing.
- Speedup ratio.
- Peak GPU memory.

Hardware-backed wgpu or CUDA evidence must use a completed copy of
`conductor/hpc-evidence/manifests/track52-live-gpu-hardware-template.json`.
Until that copy is promoted to `evidence_class: live-hpc` with raw artifacts,
checksums, device metadata, and `waiver.status: none`, benchmark statements
remain no-results scaffold language.
