# Track 69 Handoff: Stochastic Competing Risks & Non-Markovian Hazard Engine

Status: Planned

Freshness: 2026-10-20

## Summary

Track proposed to introduce native competing risks hazard sampling and non-Markovian survival clocks in `crates/kairo-ecs-rng` and `crates/kairo-ecs-des`.

## Files changed

- `conductor/tracks.yaml`
- `conductor/tracks.md`
- `conductor/tracks/69-stochastic-competing-risks-hazard-engine/*`

## Contracts consumed

- `crates/kairo-ecs-rng` random stream generators.
- `crates/kairo-ecs-des` process event scheduling.

## Contracts changed

- Added `CompetingHazards` in `kairo-ecs-rng` and `SurvivalClock` in `kairo-ecs-des`.

## Tests added

- Proposed in `test-matrix.md`.

## Known risks

- Numerical root-finding performance for general hazard integrals; see `risk-register.md`.

## Follow-up issues

- None currently identified.

## Integration notes

- Directly consumed by `careops-core` and downstream clinical tracks (`careops-birthing`, `careops-ed`) for stochastic phase transition modeling.
