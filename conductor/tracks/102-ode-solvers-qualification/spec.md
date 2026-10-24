# ode_solvers: conditional

Planning only; adoption pending. [Value policy](../../dependency-value-policy.md), [sequencing](../../dependency-sequencing.md).

## Inputs

Accepted local dependencies [78]. Exact live consumer inventory and independent baseline.

## Outputs

Accepted improve/adopt/defer/reject decision and bounded implementation receipts if justified. Existing-tooling optimisation is not a new library installation success criterion.

## Owned paths

Lead vv-uq-agent: Generic numerical integration consumer in Kairos only; CareOps keeps clinical equations/calibration. Exact source owner handoff required. Planning owns this folder; runtime/manifest/lock/CI changes require explicit current-owner handoff and claims.

## Blocked paths

Unrelated owners, cross-repository source, clinical policy, DES/resource semantics and protected settings.

## Dependencies

[78]. Conditional branches do not gate the core migration. No network/ODE work solely to satisfy a library roadmap.

## Contract

Only if an accepted continuous-dynamics consumer needs an ODE solver, compare ode_solvers with retaining a correctly named/qualified existing method. step_continuous_rk4 currently performs one Euler-style update plus projection; that correctness mismatch is separately recorded in Track 65 and must not wait for this adoption. Do not claim a solver library replaces game equilibrium/CFR methods, DES scheduling, hazard policy or event detection. Specify fixed/adaptive timestep, tolerance, positivity/conservation and event-boundary restart contracts; version intentional trajectory differences.

## Acceptance criteria

Analytical reference solutions and expected convergence order; tolerance refinement and independent reference solver; conservation/positivity without hiding error by clamping; discontinuity/event-boundary fixtures, determinism on supported targets and bounded runtime. Qualify maintenance, platform support and needed solver class before selecting a release.

## Quality gates

`dependency-102-behaviour`, `dependency-value-proof`, `phase-closeout-check`. Independent failure controls, full costs, exact version/maintenance/security/licence/MSRV/target qualification before adoption.

## CareOps handoff

Accepted immutable upstream receipts before a parent pin/API change. Parent owns its consumer and test packets. No schema or scientific policy invented upstream.

## Release implications

Public protocol/numerical changes require compatibility/version review; no hosted or operational acceptance from planning.

## Reference to requalify

https://docs.rs/ode_solvers/latest/ode_solvers/

## CareOps-Sim impact and opportunity review

Closeout includes the [lightweight downstream review](../../dependency-value-policy.md#careops-sim-impact-and-opportunity-review). Kairos supplies its accepted immutable receipt; the CareOps-Sim coordinator and consumer owner record implications, worthwhile opportunities and one of the four outcomes in `conductor/dependency-handoffs.json`, keyed by `owner_track: 102-ode-solvers-qualification`. Reuse existing consumer tracks; no automatic direct dependency or duplicate implementation track. For already accepted phases this is downstream follow-through, not reopening or changing historical acceptance. Review remains pending until evidenced separately from implementation.
