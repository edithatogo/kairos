# Track 67 Plan: Linked Entity Hierarchies, Dynamic Spawning & Cascaded Lifecycles

Status: Complete

## Phase 0 - Relationship and spawning contract

- [x] Task 0.1: Inventory Track 59/01 relationship APIs and review neutral placement without forcing game-theory solver dependencies. Freeze one-to-many cardinality, typed edges, generational handles and stable external identity mappings.
- [x] Task 0.2: Write failing fixtures for deterministic spawn during event handling, multiple children, stale handles, duplicate creation and explicit independent/coupled lifecycle policy. Reserve only the reviewed existing owner paths.
- [x] Task 0.3: Review phase fixtures, owner handoffs and executed evidence before proceeding.

## Phase 1 - Bounded relationship implementation

- [x] Task 1.1: Implement the smallest accepted neutral relationship/spawn slice with safe iteration and explicit capacity/budget failures; do not require unbounded allocation or assume every dynamic spawn is allocation-free.
- [x] Task 1.2: Implement explicitly selected detach/reparent/cascade semantics. Parent termination must not implicitly delete independent children; preserve deterministic cleanup and accounting.
- [x] Task 1.3: Review phase fixtures, owner handoffs and executed evidence before proceeding.

## Phase 2 - Telemetry and recovery integration

- [x] Task 2.1: Coordinate schema and Arrow implementation with 04/62 rather than writing unowned IO paths. Support one-to-many links, distinct lifecycle events and versioned external identity mappings.
- [x] Task 2.2: Qualify repeated create/terminate/recover, parent/child spatial separation, checkpoint round trip and unchanged downstream ED traces; clinical birth/episode semantics remain downstream.
- [x] Task 2.3: Review phase fixtures, owner handoffs and executed evidence before proceeding.

## Phase 3 - Closeout

- [x] Task 3.1: Run owner tests, conformance/compatibility checks and Conductor phase/DAG validators at the exact integrated revision.
- [x] Task 3.2: Run Conductor review; resolve in-scope findings and record deferred cross-owner work and evidence limits in handoff.md.
- [x] Task 3.3: Update registry/closeout/status through the existing coordinator only after acceptance; record source commit, hosted evidence or blockers and rollback.

## Phase: Review Fixes
- [x] Task: Apply review suggestions 13c2406

[Neutral worklist and integration order](../../design/careops-composition-plan-20261020.md).
Use the mandatory workflow phase-closeout protocol and single-maintainer harness.

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
