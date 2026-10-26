# Plan: Scheduler Preview API Compatibility

The implementation branch is rebased on current `main` at `80063b7413a26013e265107ea462828d94a49849`; PR #333 currently proposes `e3c7dc2f9f66e3c980b9d3cfb631be31842368c5`. The original CareOps dependency failure was observed against `5692e6a9ee866131262d60103a84e2544dc51e4a`.
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

- [x] Task 2.1: Reviewer-agent assessments from core scheduler, conformance and API compatibility perspectives passed on source commit `b9250f5e013a6f27d43f1e58e20c4be1ff0421fb`; findings and dispositions are recorded in the test matrix and handoff. The docs-only descendant `e3c7dc2` does not change reviewed source.
- [x] Task 2.2: In an isolated CareOps checkout, verify the candidate using a candidate-specific lock: `cargo check --locked --offline -p careops-ed-cli`, R10.3 references and rustfmt passed. The original lock fails before compile because it requires updating; original lock and parent pin were restored unchanged.
- [x] Task 2.3: Reviewer-agent assessment found the candidate lock delta is explainable but the candidate pin also changes Kairos RNG behavior from SplitMix64 to ChaCha8, outside Track 106 and without old-pin replay parity evidence. Do not adopt that broad pin/lock update. At retained CareOps pin `44261eb5cf8275f132f63a3396903d116d701dbe` with original lock hash `9ad9d38b8fe43a10a90a02989c1c1adf83607a90d620a64242386398e1a671f3`, `cargo check --locked --offline -p careops-ed-cli`, both R10.3 reference tests, and focused Rust 1.99.1 rustfmt pass. The pin and lock remain unchanged; broader RNG migration/replay qualification stays with the separate CareOps dependency track.
- [x] Task 2.4: Conductor review completed with no in-scope code findings; all accepted review findings were fixed. Track 106 is archived in this PR branch. The separate CareOps RNG/pin migration remains outside this track.

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
PR #333 carries the reviewed API implementation and archive closeout. No CareOps parent pin advancement is included.
Commit and push the cleaned slice only after phase review; the implementation slice is pushed, while this documentation reconciliation is being validated before commit/push.
