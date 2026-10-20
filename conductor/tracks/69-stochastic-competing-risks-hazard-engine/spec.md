# Track 69: Stochastic Competing Risks & Non-Markovian Hazard Engine

## Objective

Provide native discrete-event support for multi-state competing risk hazard sampling (`CompetingHazards`, `HazardDistribution`, `SurvivalClock`) in `crates/kairo-ecs-rng` and `crates/kairo-ecs-des`. Allows complex biological, physiological, and clinical trajectories (such as labor progression, spontaneous remission, or acute decompensation) to be modeled with competing time-to-event hazards under deterministic, stream-isolated random number generation.

## Inputs

- `crates/kairo-ecs-rng` pseudo-random number generator and stream isolation.
- `crates/kairo-ecs-des` event calendar and timer mechanics.
- Tracks 01, 03, 64, 66 contracts.

## Outputs

- `crates/kairo-ecs-rng/src/competing_risks.rs`
- `crates/kairo-ecs-des/src/hazard_clock.rs`
- `docs/design/competing-risks.md`
- `conductor/tracks/69-stochastic-competing-risks-hazard-engine/`

## Owned paths

- `crates/kairo-ecs-rng/`
- `crates/kairo-ecs-des/`
- `docs/design/competing-risks.md`
- `conductor/tracks/69-stochastic-competing-risks-hazard-engine/`

## Blocked paths

- `crates/kairo-ecs-core/` (owned by Track 67).
- `crates/kairo-ecs-gpu/` (owned by Track 32).
- `crates/kairo-ecs-pdes/` (owned by Track 34/46).

## Release implications

Non-breaking addition to RNG and DES scheduling. Enables non-Markovian survival analysis and competing risk simulation without custom ad-hoc sampling in application crates.
