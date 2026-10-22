# Track 68 Plan: Inter-Unit Boundary Channels & Asynchronous Handover Sinks

Status: Complete

## Phase 0 - Transfer state and ownership contract

- [x] Task 0.1: Freeze neutral operation IDs/envelopes and requested/offered/reserved/departing/in-transit/arrived/acknowledged/rejected/cancelled/expired states, including failure and compensation transitions.
- [x] Task 0.2: Write failing two-depot fixtures for bounded capacity, sender holds, destination reservations and authority/resource-release points; an accepted message is not completed transfer.
- [x] Task 0.3: Review phase fixtures, owner handoffs and executed evidence before proceeding.

## Phase 1 - Single-process transfer and backpressure

- [x] Task 1.1: Implement deterministic bounded transfers on the existing scheduler first; telemetry streaming and distributed transport are optional adapters, not the state authority.
- [x] Task 1.2: Implement retry idempotency, expiry, delayed/duplicate acknowledgement, cancellation before departure and fallback after departure; distinguish reserved capacity from physical occupancy.
- [x] Task 1.3: Review phase fixtures, owner handoffs and executed evidence before proceeding.

## Phase 2 - Conservation, recovery and adapter join

- [x] Task 2.1: Verify exactly-once logical effects under repeated delivery, no lost/duplicated entities, horizon censoring and no premature origin/destination/escort release.
- [x] Task 2.2: Qualify checkpoint/resume in every transit state and consumer rejection paths. Coordinate Arrow schema work with 04/62; real process/transport faults require separate 34/35 qualification.
- [x] Task 2.3: Review phase fixtures, owner handoffs and executed evidence before proceeding.

## Phase 3 - Closeout

- [x] Task 3.1: Run owner tests, conformance/compatibility checks and Conductor phase/DAG validators at the exact integrated revision.
- [x] Task 3.2: Run Conductor review; resolve in-scope findings and record deferred cross-owner work and evidence limits in handoff.md.
- [x] Task 3.3: Update registry/closeout/status through the existing coordinator only after acceptance; record source commit, hosted evidence or blockers and rollback.

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
