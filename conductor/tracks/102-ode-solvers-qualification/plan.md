# Plan: ode_solvers conditional

All implementation pending. Conditional/evaluation tracks may complete only a no-go report without dependency installation; do not mark adoption tasks done for that outcome.

## Phase 0 — Evidence and value gate

- [ ] Task 0.1: Verify local/external prerequisites, current ownership, actual consumer and source target; reconcile already replaced paths before starting.
- [ ] Task 0.2: Qualify exact release/features, maintenance/security/licence/MSRV/targets; capture baseline code, failures, costs and independent behavioural fixtures.
- [ ] Task 0.3: Record adopt/defer/reject against value policy, freeze numerical/performance budgets and exact test commands; add failing independent tests before any replacement. Stop on the specified no-go condition.
- [ ] Task 0.4: Conductor — review and verify phase (workflow.md); record accepted fixes, no-go/remaining blockers and explicit next-phase decision.

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

Only if an accepted continuous-dynamics consumer needs an ODE solver, compare ode_solvers with retaining a correctly named/qualified existing method. step_continuous_rk4 currently performs one Euler-style update plus projection; that correctness mismatch is separately recorded in Track 65 and must not wait for this adoption. Do not claim a solver library replaces game equilibrium/CFR methods, DES scheduling, hazard policy or event detection. Specify fixed/adaptive timestep, tolerance, positivity/conservation and event-boundary restart contracts; version intentional trajectory differences.

Independent oracle: Analytical reference solutions and expected convergence order; tolerance refinement and independent reference solver; conservation/positivity without hiding error by clamping; discontinuity/event-boundary fixtures, determinism on supported targets and bounded runtime. Qualify maintenance, platform support and needed solver class before selecting a release.
