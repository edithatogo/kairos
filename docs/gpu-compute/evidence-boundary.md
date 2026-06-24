# GPU Evidence Boundary

Track 52 separates local CPU parity from live GPU proof.

## Local CPU Parity

The local scaffold manifest is
`conductor/hpc-evidence/manifests/track52-local-cpu-parity-scaffold.json`.
It may be used to show that the backend-independent persistent-memory contract,
typed unavailable-device reports, and deterministic CPU fallback parity gates
are wired. It does not prove wgpu dispatch, CUDA dispatch, speedup, GPU memory
residency on hardware, or production HPC parity.

## Hardware GPU Proof

The hardware evidence template is
`conductor/hpc-evidence/manifests/track52-live-gpu-hardware-template.json`.
Before any GPU acceleration claim is evidence-backed, copy that template into a
completed `live-hpc` manifest and attach:

- exact commit SHA and pushed ref under test
- wgpu or CUDA backend feature flags
- device model, driver/runtime, operating system, compiler, and scheduler
- parity scenario, seed or fixture, and raw command capture
- raw timing output, resident-buffer counters, copy-boundary counters, and
  checksum for the immutable artifact bundle
- reviewer, evidence date, and `waiver.status: none`

CPU-only CI, unavailable-device tests, and local fallback parity remain
scaffold evidence. They are useful gates, but they cannot close Track 52's
hardware proof or feed Track 55 scaling certification.
