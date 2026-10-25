# Plan: Scheduler Preview API Compatibility

Candidate base is fixed at `5692e6a9ee866131262d60103a84e2544dc51e4a`.
Implementation and parent pin work remain separate; no pin update occurs before
review and consumer acceptance.

## Phase 0 — Track registration

- [x] Task 0.1: Register Track 106, exact base, bounded ownership and review gates.
- [x] Task 0.2: Record the approved specification, risks, tests and ADR decision.
- [x] Task 0.3: Conductor review and validate phase registration; record the phase decision.

## Phase 1 — Test-first compatibility implementation

- [x] Task 1.1: Add the shared preview fixture and a focused Rust conformance test; baseline failed at `5692e6a` with unresolved `ScheduledEventPreview` and missing `peek_next`.
- [x] Task 1.2: Restore the copied `ScheduledEventPreview` projection and `peek_next`, preserving existing queue ordering and lifecycle behavior.
- [x] Task 1.3: Add API documentation and compatibility notes; record any public API owner objections.
- [x] Task 1.4: Run focused core tests, formatting, rustdoc, fixture validation and required Conductor validators; retain exact command receipts.
- [ ] Task 1.5: Conductor review, apply accepted in-scope fixes, validate phase, record commit and next-phase decision.

## Phase 2 — Owner review and CareOps integration gate

- [ ] Task 2.1: Obtain core scheduler, conformance and API Governance review against the exact candidate commit.
- [ ] Task 2.2: In an isolated CareOps checkout, run `cargo check --locked -p careops-ed-cli` and R10.3 reference tests against the candidate; retain exact evidence.
- [ ] Task 2.3: Record consumer-owner acceptance and any required remediation. Stop without changing the parent pin if an owner review or check is unavailable or fails.
- [ ] Task 2.4: Conductor review and closeout; only after acceptance may a separate pin-update task be considered.

## Phase closeout gate

Before any phase advances: run `$conductor-review`; apply accepted fixes within
owned paths. Auto-apply accepted review fixes; record blocked/cross-owner
findings in `handoff.md`. Synchronize `conductor/tracks.yaml`,
`conductor/tracks.md`, `conductor/phase-closeout.yaml`, and
`conductor/status.md` (plus readiness/map when changed). Run
`pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`,
`pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_dag.ps1`,
and this track's gates; commit the cleaned slice and run
`pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`.
Commit and push the cleaned slice as required by the repository workflow; this
approved local scope does not authorize remote push or parent pin advance, so
record a pending push as a blocker unless separately authorized.
