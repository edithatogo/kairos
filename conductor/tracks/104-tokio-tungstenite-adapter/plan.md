# Plan: tokio-tungstenite conditional

Phase 0 closed with adoption deferred. See the [decision](handoff.md#phase-0-closeout--25-november-2026). The adoption phases reopen with the specified consumer and evidence.

## Phase 0 — Evidence and value gate

- [x] Task 0.1: Verify local/external prerequisites, current ownership, actual consumer and source target; reconcile already replaced paths before starting.
- [ ] Task 0.2 (conditional on an accepted WebSocket consumer): Qualify exact release/features, maintenance/security/licence/MSRV/targets; capture baseline code, failures, costs and independent behavioural fixtures.
- [x] Task 0.3 (no-go path): Record the deferral against the current consumer and source evidence. Apply the remaining qualification gates before any adoption.
- [x] Task 0.4: Review the exact current decision with the [shared reproduction and closeout evidence](../../dependency-sequencing.md#conditional-evaluation-closeout--25-november-2026). Close Phase 0.

## Phase 1 — Bounded replacement if justified

- [ ] Task 1.1: Only after accepted adopt decision, implement the specification behind existing authoritative domain boundaries; do not keep duplicate generic production paths.
- [ ] Task 1.2: Migrate actual consumers, version intentional contracts, delete superseded code and retain required validation/oracles; prove failure/recovery and measure full costs.
- [ ] Task 1.3: Conductor — review and verify phase (workflow.md); record accepted fixes and explicit next-phase decision.

## Phase 2 — Acceptance and handoff

- [ ] Task 2.1: Run every test-matrix oracle and applicable native/consumer/compatibility lanes; audit net retained complexity and costs against the frozen baseline.
- [ ] Task 2.2: Record accepted immutable commit, raw receipts and downstream consumer/pin requirements; update documentation and rollback without release claims.
- [ ] Task 2.3: Conductor — review and verify phase (workflow.md); close only with actual accepted evidence and precise adoption disposition.

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


## Specific boundary

Separate async integration only if accepted Tokio architecture and actual concurrent socket workload justify it. Do not add Tokio solely to use this wrapper. Track 82 currently follows Prost as programme scheduling, not a technical dependency of WebSockets; reassess that scheduling through its owner if a real earlier consumer arises. Keep one production transport implementation, with generic semantics shared; no permanent sync/async backend framework.

Independent oracle: Cancellation-safe connection lifecycle, bounded queues/backpressure, shutdown/reconnect, slow consumers, no lost/duplicate command effects and same protocol conformance as Track 103. Measure actual benefit over accepted simpler transport and count transitive/build cost.
