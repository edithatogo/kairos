# Plan: Python 3.15.1 migration and qualification

Prior candidate work is preserved in [the evidence snapshot](prior-qualification.md). Unchecked tasks require reconciliation/acceptance; they are not instructions to repeat proven work blindly.

## Phase 0 — Baseline and retained evidence

- [ ] Task 0.1: Inventory every active Python entry point, subprocess, package, CI/container/helper path and live environment; classify historical or synthetic exceptions explicitly.
- [ ] Task 0.2: Reconcile the retained candidate evidence and source hashes with current main and current owners; credit completed qualification without treating it as main acceptance.
- [ ] Task 0.3: Freeze exact path ownership, interpreter/dependency targets, required platform checks and safe terminal handover criteria.
- [ ] Task 0.4: Conductor — review and verify phase (workflow.md); record evidence, unresolved targets and next-phase decision.

## Phase 1 — Complete and qualify the candidate

- [ ] Task 1.1: Add or reuse meaningful failure tests for runtime mismatch, out-of-line discovery, environment preservation and native/package execution before changing affected paths.
- [ ] Task 1.2: Complete supported Python 3.15.1 runtime, constraints, lock, CI, container, helper and current-instruction alignment in isolated environments; remove older active runners.
- [ ] Task 1.3: Run the affected helper/package/binding/security and representative consumer checks with the explicit repo-local interpreter; retain raw outputs, hashes and concrete blockers.
- [ ] Task 1.4: Conductor — review and verify phase (workflow.md); require actual runtime evidence and resolve in-scope findings.

## Phase 2 — Reviewed integration and current-main proof

- [ ] Task 2.1: Reconcile fresh main and ongoing writers, open the bounded PR, address review findings and pass every required hosted check without waivers.
- [ ] Task 2.2: Merge through normal protection, verify the exact main tree and inspect post-merge runtime/check evidence; preserve unrelated implementation and pins.
- [ ] Task 2.3: Conductor — review and verify phase (workflow.md); distinguish local qualification, hosted proof and accepted integration.

## Phase 3 — Safe environment handover and closeout

- [ ] Task 3.1: Confirm old consumers have reached terminal states, hand over the qualified repo-local interpreter safely and verify its actual version in consuming commands.
- [ ] Task 3.2: Re-audit all active entry points and older defaults, reconcile remaining external/platform blockers, retain rollback/history and update evidence at the accepted main revision.
- [ ] Task 3.3: Conductor — review and verify phase (workflow.md); complete only when every requirement and handover has evidence.

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


Registry changes are coordinated with their current writer. This planning revision
does not close an implementation phase or authorise publication.
