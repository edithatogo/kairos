# Risk register

| Risk | Impact | Mitigation / evidence owner |
|---|---|---|
| State-dependent event counts desynchronize common random numbers across policy arms | High Monte Carlo variance on incremental policy net benefit and false sign reversals in downstream VOI | `vv-uq-agent + des-api-agent`; enforce `(entity_id, transition_slot, epoch)` substream addressing with deterministic replay tests. |
| Domain leakage into Kairos kernel | Pollutes neutral ECS/DES/ABM abstractions with tobacco-specific states or New Zealand demographic strata | `api-governance-agent`; enforce generic integer/symbol state IDs and externalized stratum metadata in Arrow schemas. |
| Discrepancy between aggregate semi-Markov cohort equations and individual microsimulation hazards | Conflicting policy rankings between `tobac` macro and micro simulation modes | `vv-uq-agent`; require large-N convergence oracle against `innovate` / `tobac` analytical semi-Markov reference before consumer pin adoption. |
| Silent unit or time-step mismatch across Arrow/Polars handoff | Mis-scaled person-years or discount factors in downstream `dapane` and `voiage` evaluation | `arrow-agent`; require explicit time-unit and calendar-epoch metadata in batch schema headers with negative fixture tests. |
