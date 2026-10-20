# Track 69 Plan: Stochastic Competing Risks & Non-Markovian Hazard Engine

## Phase 0 - Competing Risk Sampling Primitives

- [ ] Task 0.1: Implement `HazardDistribution` (Weibull, Gamma, Log-logistic, Gompertz) and cumulative hazard integration in `crates/kairo-ecs-rng`.
- [ ] Task 0.2: Implement `CompetingHazards` multi-cause sampler selecting cause $k$ and transition time $\tau$ deterministically.

## Phase 1 - Non-Markovian Survival Clock & Event Scheduling

- [ ] Task 1.1: Implement `SurvivalClock` in `crates/kairo-ecs-des` supporting age-dependent, time-varying hazard rates.
- [ ] Task 1.2: Add preemption/censoring handlers that preserve residual hazard mass upon interrupted states.

## Phase 2 - Replay Invariance & Stream Isolation

- [ ] Task 2.1: Verify independent RNG substream draws preventing hazard sampling from perturbing arrival or resource streams.
- [ ] Task 2.2: Add reference survival curve statistical verification fixtures against analytical hazard integrals.

## Phase 3 - Closeout

- [ ] Task 3.1: Run track-specific test matrix, workspace tests, and Conductor validators.
- [ ] Task 3.2: Run `$conductor-review`, auto-apply accepted fixes, and update `handoff.md`.
- [ ] Task 3.3: Commit and push the cleaned slice and verify GitHub Actions.
