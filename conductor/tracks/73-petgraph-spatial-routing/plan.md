# Plan: petgraph spatial topology and routing adoption

All implementation tasks are pending. Write failing behavioural tests before production replacement. Accept CareOps ECharts R28 and Kairos sampling Track 72 before upstream routing work; accept Kairos Track 73 Phase 2 before CareOps R29 pin adoption.

## Phase 0 — Entry and qualification

- [ ] Task 0.1: Verify the entry receipts and ownership handoffs; reject missing/stale upstream or downstream evidence before any pin/dependency/runtime change.
- [ ] Task 0.2: Inventory actual replace/retain candidates and freeze baseline contracts, fixtures, costs and supported targets; qualify exact pins/features, licences/advisories and compatibility.
- [ ] Task 0.3: Freeze generic topology/identity/cost/tie/no-route API and CareOps mapping contract; add independent small-graph, access and multi-floor tests.
- [ ] Task 0.4: Conductor — review and verify phase (workflow.md); apply accepted fixes, record blockers/receipts and explicit next-phase decision.

## Phase 1 — Replacement and consumers

- [ ] Task 1.1: Adopt petgraph storage/Dijkstra with deterministic route extraction and explicit invalidation; preserve resource/scheduler ownership.
- [ ] Task 1.2: Migrate actual graph consumers and synthetic CareOps adapters after accepted upstream handoff; remove superseded adjacency/algorithm code only where replaced.
- [ ] Task 1.3: Conductor — review and verify phase (workflow.md); apply accepted fixes, record blockers/receipts and explicit next-phase decision.

## Phase 2 — Acceptance and handoff

- [ ] Task 2.1: Run every test-matrix oracle and applicable full native/browser lanes; resolve regressions and record raw evidence.
- [ ] Task 2.2: Audit deleted/retained production code, update compatibility/user documentation and record rollback plus exact accepted commit/pin receipts.
- [ ] Task 2.3: Conductor — review and verify phase (workflow.md); apply accepted fixes, record blockers/receipts and explicit next-phase decision.

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
