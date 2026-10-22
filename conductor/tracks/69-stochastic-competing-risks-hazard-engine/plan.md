# Track 69 Plan: Stochastic Competing Risks & Non-Markovian Hazard Engine

Status: pending revised plan; historical registration is not implementation acceptance.

## Phase 0 - Competing-event mathematical contract

- [x] Task 0.1: Freeze hazard/cumulative-hazard units, age versus calendar time, competing cause semantics, state transitions, censoring, tie rules and named random-stream ownership with 21/01 review.
- [x] Task 0.2: Write analytic exponential/piecewise controls, degenerate zero-hazard and bounded numerical error cases before adding a consumer-justified non-Markovian family; do not begin with an unqualified catalogue of distributions.
- [x] Task 0.3: Review phase fixtures, owner handoffs and executed evidence before proceeding.

## Phase 1 - Progression and rescheduling

- [x] Task 1.1: Implement the smallest reviewed sampler/clock on existing RNG and scheduler contracts; preserve progression during queueing, service and transit without clinical policies in Kairos.
- [x] Task 1.2: Define retained threshold/residual hazard versus restart per process contract; update covariates with stale-event invalidation, deterministic cancellation and no redraw of unrelated work.
- [x] Task 1.3: Review phase fixtures, owner handoffs and executed evidence before proceeding.

## Phase 2 - Statistical and composition qualification

- [x] Task 2.1: Test cause-specific cumulative incidence, survival, right censoring, time-varying hazards and chosen numerical tolerances against independent analytic/reference controls; retain failure-rate and sample-size justification.
- [x] Task 2.2: Verify checkpoint and 1/2/4 worker replay, unrelated-module noninterference, observation-only decision access and non-identifiability controls; no clinical/causal validity claim follows.
- [x] Task 2.3: Review phase fixtures, owner handoffs and executed evidence before proceeding.

## Phase 3 - Closeout

- [x] Task 3.1: Run owner tests, conformance/compatibility checks and Conductor phase/DAG validators at the exact integrated revision.
- [x] Task 3.2: Run Conductor review; resolve in-scope findings and record deferred cross-owner work and evidence limits in handoff.md.
- [x] Task 3.3: Update registry/closeout/status through the existing coordinator only after acceptance; record source commit, hosted evidence or blockers and rollback.

[Neutral worklist and integration order](../../design/careops-composition-plan-20261020.md).
Use the mandatory workflow phase-closeout protocol and single-maintainer harness.
Plans and registration do not authorise runtime/API changes without a reviewed packet.

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
