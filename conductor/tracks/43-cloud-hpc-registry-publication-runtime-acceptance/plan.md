# 43 Cloud/HPC Registry Publication & Runtime Acceptance - plan.md

## Phase 0 - Track startup

- [x] Define cloud/HPC registry ownership.
- [x] Add HPC registry manifest and validation gate.
- [x] Add guarded HPC registry workflow.

## Phase 1 - Publication lanes

- [x] OCI image lane.
- [x] Kubernetes bundle lane.
- [x] Slurm template lane.
- [x] AWS Batch template/canary lane.
- [x] GCP Batch template/canary lane.
- [x] Azure Batch template/canary lane.

## Phase 2 - Runtime evidence

- [x] Require command, runner/cluster/account/region/partition, job ID, final status, and artifact references.
- [x] Keep claims bounded to scaffold/offline validation until live evidence exists.
- [ ] Run Docker canary.
- [ ] Run Kubernetes canary.
- [ ] Run Slurm canary.
- [ ] Run AWS/GCP/Azure Batch canaries.
  - [x] Azure CPU Batch substrate canary completed on 2026-05-20.
  - [ ] Azure `kairo-ecs-cli` container/scenario canary with output/checksum evidence.
  - [ ] AWS Batch canary.
  - [ ] GCP Batch canary.

## Phase closeout gate

Before any task or phase in this track is marked complete, and before the next phase begins:

1. Run `$conductor-review` against this track and the current diff.
2. Auto-apply accepted review fixes inside this track's owned paths.
3. Record rejected, cross-track, or blocked-path fixes in `handoff.md`.
4. Update the track registry/status surfaces: `conductor/tracks.yaml` (authoritative machine-readable registry), `conductor/tracks.md` (human index), `conductor/phase-closeout.yaml` (review ledger), `conductor/status.md` (narrative status), and `conductor/implementation-readiness.md` or `conductor/track-map.md` when readiness, ownership, dependency, gate, or wave data changes.
5. Run `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` plus the gates listed in `test-matrix.md`.
6. Commit and push the cleaned slice, then record the commit SHA or blocker in `handoff.md`.
7. Run `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` to verify recorded commits, pushed refs, and cleanup state.
8. Advance the next phase only after there is no in-scope unstaged or untracked work except documented draft satellites.

## Archive boundary

- [x] Review and archive the repo-side guarded cloud/HPC publication and runtime-evidence gate as Done.
- [x] Verify the guarded registry manifest and runtime evidence validator pass while keeping production_claim_status blocked.
- [x] Verify protected publish mode fails until live publication evidence and all required runtime scopes are passed.
- [x] Record that Docker, Kubernetes, Slurm, AWS Batch, GCP Batch, Azure KairoECS scenario canaries, protected publication, and release-manager approval remain release-gated and are not claimed by this archive.
