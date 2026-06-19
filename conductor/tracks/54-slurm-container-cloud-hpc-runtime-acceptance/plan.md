# Track 54 Plan: Slurm, Container & Cloud HPC Runtime Acceptance

## Phase 0 - TDD and dry-run baseline

- Task 0.1: Add failing validators for Docker, Kubernetes, Slurm, and provider
  evidence manifests.
- Task 0.2: Add dry-run checks that reject missing scenario output checksums.
- Task 0.3: Add explicit quota/blocker evidence format tests.

## Phase 1 - Container and Kubernetes

- Task 1.1: Build and run a KairoECS container scenario locally.
- Task 1.2: Add Kubernetes job manifests and completion validation.
- Task 1.3: Record digests, logs, outputs, and checksums.

## Phase 2 - Slurm

- Task 2.1: Add single-node Slurm job script.
- Task 2.2: Add MPI Slurm job script consuming Track 49 launch contract.
- Task 2.3: Add GPU Slurm job script consuming Track 52 runtime contract.

## Phase 3 - Provider canaries

- Task 3.1: Run AWS Batch canary or record quota/account blocker.
- Task 3.2: Run GCP Batch canary or record quota/account blocker.
- Task 3.3: Run Azure Batch canary or record quota/account blocker.

## Phase 4 - Publication handoff

- Task 4.1: Handoff runtime evidence to Track 43 publication gate.
- Task 4.2: Handoff scheduler and provider metrics to Track 55.
- Task 4.3: Document protected environment approvals and release-manager
  signoff requirements.

## Phase 5 - Closeout

- Task 5.1: Run cloud/HPC validators, live canaries, and Conductor gates.
- Task 5.2: Run `$conductor-review` and apply accepted fixes.
- Task 5.3: Push and verify GitHub Actions.

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
