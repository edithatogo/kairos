# Track 46: HPC Parity Charter, Baselines & Evidence Gates

## Purpose

Define the production HPC parity bar for KairoECS and create the shared
evidence contract consumed by Tracks 47-55. This track prevents scaffold or
emulator success from being reported as parity with ROSS, SST, Repast HPC,
OpenFPM, Parallel HDF5, ADIOS2, hwloc, CUDA/WebGPU, Slurm, or FMI runtimes.

## Maturity

Spec Approved planning track. No runtime implementation or live HPC proof is
claimed by this artifact.

## Inputs

- Existing Conductor status for Tracks 32-35, 38, 39, 43, and 44.
- `conductor/sota-scorecard.md`, `conductor/quality-gates.md`, and
  `conductor/hpc-parity-wave.md`.
- Existing benchmark and reproducibility policy from Tracks 18 and 31.

## Outputs

- Shared HPC parity charter and evidence manifest requirements.
- Central quality-gate definitions for Tracks 47-55.
- Release-claim language boundaries for all HPC-facing docs and registry text.
- Baseline catalogue for external library and runtime comparisons.

## Owned paths

- `conductor/hpc-parity-wave.md`
- `conductor/sota-scorecard.md`
- `conductor/quality-gates.md`
- `conductor/tracks/46-hpc-parity-charter-baselines-evidence-gates/`

## Blocked paths

- Runtime crates owned by Tracks 47-53.
- Cloud, Slurm, and provider assets owned by Track 54.
- Benchmark result artifacts owned by Track 55.

## Dependencies

Tracks 18, 26, 28, 29, 31, and 44.

## Parallel-safe tracks

Tracks 47-55 may draft tests and implementation plans in parallel after this
track's evidence fields and release-claim vocabulary are accepted.

## Acceptance criteria

- Every new HPC parity track names live evidence required for `Done`.
- Central gates distinguish scaffold/fallback tests from live runtime proof.
- Public claim language cannot say production-ready HPC parity until Track 55
  has closed with raw weak/strong scaling evidence.
- Evidence manifest fields are sufficient to reproduce hardware, scheduler,
  toolchain, runtime, and artifact provenance.

## Quality gates

- `hpc-parity-charter`
- `hpc-evidence-manifest`
- `hpc-claim-boundary`
- `phase-closeout-check`

## Release implications

This track is release-gating for any production HPC parity claim. It does not
ship runtime capability by itself.
