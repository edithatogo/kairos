# Agent contract

## Owner

`des-api-agent + abm-api-agent + vv-uq-agent`. Coordinates with `api-governance-agent` under Track 114 ecosystem boundary rules.

## Write scope

This track directory (`conductor/tracks/116-tobac-cohort-microsimulation-contracts/`) and explicitly claimed implementation slices in `crates/kairo-ecs-des/`, `crates/kairo-ecs-abm/`, and `crates/kairo-ecs-arrow/` during future execution phases. Read `AGENTS.md`, Track 114 contracts, and the selected phase before claiming.

## Required evidence

- **Domain separation**: Kairos crates and schemas contain zero tobacco, nicotine, vaping, or New Zealand policy literals; all domain semantics live in `edithatogo/tobac`.
- **CRN trajectory coupling**: Paired policy counterfactuals using entity-indexed RNG substreams maintain identical pre-intervention event traces and deterministic post-intervention differences.
- **Competing-risk accuracy**: Large-sample microsimulation incidence and dwell-time distributions agree with an independent analytical semi-Markov cohort reference within declared Monte Carlo tolerance.
- **Schema and unit hygiene**: Positive and negative Arrow/Polars batch fixtures reject negative dwell times, non-monotonic timestamps, duplicate active states, and uncoupled draw indices.

## Handoff and review

Return exact commit, changed paths, actual commands/exits, remaining risks, and unsupported claims. A different reviewer checks semantics and negative cases. Planning registration does not certify runtime execution or advance any consumer pin.
