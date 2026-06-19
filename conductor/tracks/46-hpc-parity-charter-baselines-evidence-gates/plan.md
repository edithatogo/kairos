# Track 46 Plan: HPC Parity Charter, Baselines & Evidence Gates

## Phase 0 - Track creation

- Task 0.1: Create the track artifact set and registry entries.
- Task 0.2: Record the HPC parity wave in `conductor/hpc-parity-wave.md`.
- Task 0.3: Add central gate definitions to `conductor/quality-gates.md`.

## Phase 1 - Baseline catalogue

- Task 1.1: Define comparison surfaces for ROSS, SST, Repast HPC, OpenFPM,
  Parallel HDF5, ADIOS2, hwloc, CUDA/WebGPU, Slurm, and FMI.
- Task 1.2: Map each baseline to a KairoECS claim boundary and required
  evidence field.
- Task 1.3: Add SOTA scorecard rows for PDES, distributed sync, NUMA, I/O,
  GPU, FMI, scheduler runtime, and scaling certification.

## Phase 2 - Evidence manifest

- [x] Task 2.1: Create a machine-readable evidence manifest schema.
- [x] Task 2.2: Add sample manifests for local scaffold proof and live-HPC proof.
- [x] Task 2.3: Add validation that prevents live-proof fields from being omitted.

## Phase 3 - Release claim controls

- [x] Task 3.1: Add a no-overclaim scan for HPC docs, README, packaging, and
  release-note surfaces.
- [x] Task 3.2: Require every production HPC claim to name a closed evidence
  manifest and commit SHA.
- [x] Task 3.3: Add waiver language for unavailable external hardware or accounts.

## Phase 4 - Cross-track handoff

- Task 4.1: Hand the evidence manifest to Tracks 47-55.
- Task 4.2: Require each downstream test matrix to include live proof and
  fallback proof as separate gates.
- Task 4.3: Record blocked external evidence in downstream handoffs.

## Phase 5 - Closeout

- Task 5.1: Run Conductor validators and claim-boundary checks.
- Task 5.2: Run `$conductor-review` and apply accepted in-scope fixes.
- Task 5.3: Commit, push, and record the commit SHA and pushed ref.

## Phase closeout gate

Before any task or phase in this track is marked complete, and before the next
phase begins:

1. Run `$conductor-review` against this track and the current diff.
2. Auto-apply accepted review fixes inside this track's owned paths.
3. Record rejected, cross-track, or blocked-path fixes in `handoff.md`.
4. Update `conductor/tracks.yaml`, `conductor/tracks.md`,
   `conductor/phase-closeout.yaml`, `conductor/status.md`,
   `conductor/implementation-readiness.md`, and `conductor/track-map.md` when
   readiness, ownership, dependency, gate, or wave data changes.
5. Run `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`
   plus the gates listed in `test-matrix.md`.
6. Commit and push the cleaned slice, then record the commit SHA or blocker in
   `handoff.md`.
7. Run `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`.
8. Advance only after there is no in-scope unstaged or untracked work except
   documented draft satellites.
