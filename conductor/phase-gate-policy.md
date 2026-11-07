# Conductor Phase Gate Policy

## Automatic phase closeout gate

Apply this loop at a coherent integration or phase/status milestone, not at each
leaf task or test iteration. This proportional interpretation supersedes older
repeated per-leaf ceremony. Routine authorized changes need a diff and focused
check result; contract/numerical/replay/security/release changes need the relevant
specialist review. No additional human approval is implied. See the
[workflow](workflow.md) and [bounded workpack](execution/README.md).

1. Run `$conductor-review` against the track and current diff.
2. Auto-apply accepted review fixes inside the track's owned paths.
3. Record rejected, cross-track, or blocked-path fixes in `handoff.md`.
4. When status or phase changes, update the relevant registry/status surfaces in lockstep: `conductor/tracks.yaml` is the authoritative machine-readable registry, `conductor/tracks.md` is the human-readable index, `conductor/phase-closeout.yaml` is the review ledger, and `conductor/status.md` is the narrative status. Also update `conductor/implementation-readiness.md` and `conductor/track-map.md` when readiness, ownership, dependency, gate, or wave data changes.
5. Run `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`, `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_dag.ps1`, and the track gates listed in `test-matrix.md`.
6. Commit and push the cleaned slice, then record the commit SHA or blocker in `handoff.md`.
7. Run `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` to verify recorded commits, pushed refs, and cleanup state.
8. Advance the next phase only after there is no in-scope unstaged or untracked work except documented draft satellites.

The review step is part of implementation closeout, not a separate optional maintenance task. A track cannot move from implementation to `In Review`, or from review to `Done`, unless the review loop has run and the registry/status surfaces above reflect the same decision.

## Auto-apply boundary

Accepted review fixes are applied automatically only when they stay inside the track's owned paths and do not conflict with another track's active ownership. Cross-track fixes, blocked paths, rejected findings, skipped validators, and external blockers must remain visible in `handoff.md` until resolved.

## Blocking rule

If review, cleanup, validation, commit, or push is blocked, the phase remains unadvanced. The blocker must name the affected control, the command or review finding, the owner, and the next action.
