# 44 Code and Repository Health >9.5 - plan.md

## Phase 0 - Track startup

- [x] Add code health scorecard.
- [x] Add code health validator.
- [x] Add CI workflow.

## Phase 1 - Score categories

- [x] CI and tests.
- [x] Security and supply chain.
- [x] Docs and learning coverage.
- [x] Release and registry readiness.
- [x] API compatibility and conformance.
- [x] Repo hygiene and maintainability.

## Phase 2 - Release integration

- [x] Add `code-health-floor` gate.
- [x] Make Track 42 depend on Track 44.
- [x] Make Track 43 depend on Track 44.

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
