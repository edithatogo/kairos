# Track 66: Parallel Ensemble Sweeps (Sobol/CRN), SBI Calibration & Copy-on-Write Wasm Studio

## Objective

Provide end-to-end scientific calibration and interactive visual debugging by adding quasi-random Sobol/Latin-Hypercube ensemble sweeps with Common Random Numbers (CRN) and Optuna/ABC-SMC calibration hooks to `kairo-ecs-cli`, upgrading `kairo-ecs-debug` to copy-on-write (CoW) page-delta snapshots, and wiring the live Wasm engine into `website/playground` for browser-based time-travel scrubbing and entity inspection.

## Inputs

- Existing contracts and scaffolds from Tracks 21, 22, 24, 40, 63, 64.
- Upstream ecosystem crate policies in `deny.toml` and `conductor/tech-stack.md`.
- Protected surface inventory in `docs/design/protected-surface-inventory.json`.

## Outputs

- `crates/kairo-ecs-cli/`
- `crates/kairo-ecs-debug/`
- `website/playground/`
- `docs/validation/`
- `docs/debugging/`
- `conductor/tracks/66-ensemble-sbi-calibration-cow-wasm-studio/`

## Owned paths

- `crates/kairo-ecs-cli/`
- `crates/kairo-ecs-debug/`
- `website/playground/`
- `docs/validation/`
- `docs/debugging/`
- `conductor/tracks/66-ensemble-sbi-calibration-cow-wasm-studio/`

## Blocked paths

- Do not modify unrelated track directories or bypass `deny.toml` license and advisory checks when adding upstream dependencies.
- Do not claim production HPC cluster scaling from single-host or free-tier smoke runs.
- Do not break existing deterministic conformance fixtures (`scheduler_ordering_v1`, `scheduler_cancellation_v1`, `rng_reproducibility_v1`, `vvuq_scenario_replay_v1`).

## Dependencies

- Depends on Tracks: 21, 22, 24, 40, 63, 64

## Parallel-safe tracks

- Can execute in parallel with tracks outside `crates/kairo-ecs-cli/, crates/kairo-ecs-debug/` provided shared contracts in `conductor/contracts/` remain unchanged or are updated via ADR.

## Acceptance criteria

- Gate `phase-closeout-check` passes with recorded command receipts.
- Gate `sobol-lhs-crn-ensemble-runner` passes with recorded command receipts.
- Gate `optuna-abc-smc-calibration-hook` passes with recorded command receipts.
- Gate `cow-delta-snapshot-memory-budget` passes with recorded command receipts.
- Gate `wasm-time-travel-inspector-smoke` passes with recorded command receipts.
- All unit, integration, and conformance tests pass under `just ci` and Conductor validators.

## Quality gates

- `phase-closeout-check`
- `sobol-lhs-crn-ensemble-runner`
- `optuna-abc-smc-calibration-hook`
- `cow-delta-snapshot-memory-budget`
- `wasm-time-travel-inspector-smoke`

## Release implications

- Promotes KairoECS from dependency-free R2 contract scaffolds to R3/R4 ecosystem-integrated runtime execution for Parallel Ensemble Sweeps (Sobol/CRN), SBI Calibration & Copy-on-Write Wasm Studio.
- Any new external Cargo or language dependencies introduced in this track must pass `cargo deny`, `dependency-review.yml`, and SBOM generation before RC/1.0 release signoff.
