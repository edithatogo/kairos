# Conductor Phase Gate Policy

## Automatic phase closeout gate

Every non-terminal track must close each phase through the same review-fix-cleanup loop before the next phase starts.

1. Run `$conductor-review` against the track and current diff.
2. Auto-apply accepted review fixes inside the track's owned paths.
3. Record rejected, cross-track, or blocked-path fixes in `handoff.md`.
4. Update `conductor/phase-closeout.yaml` with review outcome, accepted fixes, validation commands, cleanup state, commit SHA or blocker, pushed ref, and next-phase decision.
5. Run `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` plus the track gates listed in `test-matrix.md`.
6. Commit and push the cleaned slice, then record the commit SHA or blocker in `handoff.md`.
7. Run `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` to verify recorded commits, pushed refs, and cleanup state.
8. Advance the next phase only after there is no in-scope unstaged or untracked work except documented draft satellites.

## Auto-apply boundary

Accepted review fixes are applied automatically only when they stay inside the track's owned paths and do not conflict with another track's active ownership. Cross-track fixes, blocked paths, rejected findings, skipped validators, and external blockers must remain visible in `handoff.md` until resolved.

## Blocking rule

If review, cleanup, validation, commit, or push is blocked, the phase remains unadvanced. The blocker must name the affected control, the command or review finding, the owner, and the next action.
