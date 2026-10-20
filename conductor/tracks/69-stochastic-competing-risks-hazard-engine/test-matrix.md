# Test matrix: composition refinement

| Phase | Required oracle |
| --- | --- |
| 0 | Freeze hazard/cumulative-hazard units, age versus calendar time, competing cause semantics, state transitions, censoring, tie rules and named random-stream ownership with 21/01 review. Write analytic exponential/piecewise controls, degenerate zero-hazard and bounded numerical error cases before adding a consumer-justified non-Markovian family; do not begin with an unqualified catalogue of distributions. |
| 1 | Implement the smallest reviewed sampler/clock on existing RNG and scheduler contracts; preserve progression during queueing, service and transit without clinical policies in Kairos. Define retained threshold/residual hazard versus restart per process contract; update covariates with stale-event invalidation, deterministic cancellation and no redraw of unrelated work. |
| 2 | Test cause-specific cumulative incidence, survival, right censoring, time-varying hazards and chosen numerical tolerances against independent analytic/reference controls; retain failure-rate and sample-size justification. Verify checkpoint and 1/2/4 worker replay, unrelated-module noninterference, observation-only decision access and non-identifiability controls; no clinical/causal validity claim follows. |
| 3 | Owner-native tests, compatibility, exact-source phase/DAG and hosted evidence; no promotion from a planned command. |

Resolve real executable targets at dispatch; future commands are not test results.
Unavailable dependencies remain explicit and do not pass through mocks.

## Conductor closeout checks

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`
- `pwsh -NoProfile -File scripts/validate_conductor_dag.ps1`
- `pwsh -NoProfile -File scripts/validate_conductor_artifacts.ps1`
- `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`

The strict clean-tree closeout gate applies to accepted implementation phases.
Planning validation does not establish a passed runtime or publication gate.
