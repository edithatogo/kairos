# Track 69: Stochastic Competing Risks & Non-Markovian Hazard Engine

## Objective

Provide native discrete-event support for multi-state competing risk hazard sampling (`CompetingHazards`, `HazardDistribution`, `SurvivalClock`) in `crates/kairo-ecs-rng` and `crates/kairo-ecs-des`. Allows independently progressing processes (such as equipment failure, repair or job completion) to be modeled with competing time-to-event hazards under deterministic, stream-isolated random number generation.

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

Proposed RNG/DES extension; compatibility and statistical correctness require independent review. Enables non-Markovian survival analysis and competing risk simulation without custom ad-hoc sampling in application crates.

## Composition contract refinement — 20 October 2026

Separate process progression from resource service. States may evolve while waiting or travelling, with explicit cancellation/revision of scheduled events when covariates change. Preserve existing RNG contracts and stream ownership; a common seed alone is insufficient counterfactual alignment. Censoring is an observation boundary, not necessarily termination of the latent process. Retention versus restart of integrated hazard is an explicit mathematical choice, not a universal preemption rule. Separate observed information from latent future draws and keep clinical distributions outside Kairos.

See [owner worklist and downstream join](../../design/careops-composition-plan-20261020.md). All work remains pending. Prospective source filenames are resolved at dispatch; they are not evidence that APIs exist.

## Acceptance and cross-owner gates

Require the phase test matrix, meaningful negative controls, exact input/source/output hashes, deterministic replay and failure/recovery receipts. Coordinate Arrow modifications through 04/62 and shared scheduler/public contracts through 01/25. Current broad registry paths are prospective: reserve bounded disjoint files and rebind on source drift. Publication, hardware runtime and clinical validation remain separate.
