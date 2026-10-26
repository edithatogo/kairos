# Plan: Scheduler Preview API Compatibility

The implementation branch is rebased on current `main` at `80063b7413a26013e265107ea462828d94a49849`; exact proposed head is tracked by PR #333 (`29abb88718fb1e76b7dedf81766f7d49058dc5d2` at last verification). The original CareOps dependency failure was observed against `5692e6a9ee866131262d60103a84e2544dc51e4a`.
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
- [x] Task 1.5: Conductor source review completed; two accepted in-scope fixes applied, current-head checks passed locally, and PR #333 opened ready for review. Reviewer-agent assessments and hosted CI remain gates before phase acceptance.

## Phase 2 — Reviewer-agent assessment and CareOps integration gate

- [ ] Task 2.1: Obtain reviewer-agent/subagent assessments from core scheduler, conformance and API compatibility perspectives against the exact candidate commit; retain separate review receipts and coordinator dispositions.
- [x] Task 2.2: In an isolated CareOps checkout, verify the candidate using a candidate-specific lock: `cargo check --locked --offline -p careops-ed-cli`, R10.3 references and rustfmt passed. The original lock fails before compile because it requires updating; original lock and parent pin were restored unchanged.
- [ ] Task 2.3: Record exact consumer-check results and assess the original-lock dependency delta with reviewer-agent evidence. The single maintainer integrates and accepts the outcome; no separate consumer-owner reviewer is required. Keep the parent pin and lock unchanged until upstream and consumer checks pass.
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
The approved candidate has already been pushed as PR #333. Parent pin advancement remains a separate gated action.
Commit and push the cleaned slice only after phase review; the implementation slice is pushed, while this documentation reconciliation is being validated before commit/push.
