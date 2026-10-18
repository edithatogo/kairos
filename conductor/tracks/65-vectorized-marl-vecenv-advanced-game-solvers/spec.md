# Track 65: Vectorized Multi-World RL (`VecEnv`), PettingZoo Bridge & Advanced Game Theory Solvers

## Objective

Match OpenSpiel, PettingZoo, PufferLib, and EnvPool by adding Mixed-Strategy Nash (support enumeration & Lemke-Howson), Correlated Equilibrium, Counterfactual Regret Minimization (`CFR`/`CFR+`), and Evolutionary Replicator Dynamics to `kairo-ecs-game-theory`, alongside a Rayon-parallelized `VecEnv` batch runner in `kairo-ecs-ml` that steps 1,000+ independent DES/ABM/Game worlds into contiguous NumPy/PyTorch tensors with zero GIL overhead.

## Inputs

- Existing contracts and scaffolds from Tracks 37, 60, 61, 63, 64.
- Upstream ecosystem crate policies in `deny.toml` and `conductor/tech-stack.md`.
- Protected surface inventory in `docs/design/protected-surface-inventory.json`.

## Outputs

- `crates/kairo-ecs-game-theory/`
- `crates/kairo-ecs-ml/`
- `bindings/python/`
- `docs/game-theory/`
- `docs/ml/`
- `conductor/tracks/65-vectorized-marl-vecenv-advanced-game-solvers/`

## Owned paths

- `crates/kairo-ecs-game-theory/`
- `crates/kairo-ecs-ml/`
- `bindings/python/`
- `docs/game-theory/`
- `docs/ml/`
- `conductor/tracks/65-vectorized-marl-vecenv-advanced-game-solvers/`

## Blocked paths

- Do not modify unrelated track directories or bypass `deny.toml` license and advisory checks when adding upstream dependencies.
- Do not claim production HPC cluster scaling from single-host or free-tier smoke runs.
- Do not break existing deterministic conformance fixtures (`scheduler_ordering_v1`, `scheduler_cancellation_v1`, `rng_reproducibility_v1`, `vvuq_scenario_replay_v1`).

## Dependencies

- Depends on Tracks: 37, 60, 61, 63, 64

## Parallel-safe tracks

- Can execute in parallel with tracks outside `crates/kairo-ecs-game-theory/, crates/kairo-ecs-ml/` provided shared contracts in `conductor/contracts/` remain unchanged or are updated via ADR.

## Acceptance criteria

- Gate `phase-closeout-check` passes with recorded command receipts.
- Gate `mixed-nash-lemke-howson-parity` passes with recorded command receipts.
- Gate `cfr-plus-extensive-form-convergence` passes with recorded command receipts.
- Gate `rayon-vecenv-1024-worlds-throughput` passes with recorded command receipts.
- Gate `pettingzoo-gymnasium-api-conformance` passes with recorded command receipts.
- All unit, integration, and conformance tests pass under `just ci` and Conductor validators.

## Quality gates

- `phase-closeout-check`
- `mixed-nash-lemke-howson-parity`
- `cfr-plus-extensive-form-convergence`
- `rayon-vecenv-1024-worlds-throughput`
- `pettingzoo-gymnasium-api-conformance`

## Release implications

- Promotes KairoECS from dependency-free R2 contract scaffolds to R3/R4 ecosystem-integrated runtime execution for Vectorized Multi-World RL (`VecEnv`), PettingZoo Bridge & Advanced Game Theory Solvers.
- Any new external Cargo or language dependencies introduced in this track must pass `cargo deny`, `dependency-review.yml`, and SBOM generation before RC/1.0 release signoff.
