# Track 116 specification: Cohort and spatial tobacco/vaping microsimulation contracts for tobac

## Overview and approved outcome

Provide neutral, deterministic DES/ABM cohort-trajectory, competing-risk hazard, and spatial retail-exposure Arrow/Polars contracts so `edithatogo/tobac` (`conductor/tracks/dependency_ecosystem_codesign_20261110/`) can run individual-level microsimulation cross-checks against aggregate cohort compartmental models while keeping tobacco/vaping epidemiology, regulatory policy, and New Zealand demographic calibration strictly inside `tobac`.

User authority: 10 November 2026 request to register corresponding Conductor tracks across maintained ecosystem dependencies for `tobac` beyond-frontier co-design.

## Inputs

- Kairos Tracks 01 (`kairo-ecs-core`, `kairo-ecs-state`, `kairo-ecs-rng`), 03 (`kairo-ecs-des`, `kairo-ecs-abm`), 04 (`kairo-ecs-arrow`), 69 (competing risks and non-Markovian hazard engine), 70 (Polars-native data interchange), and 114 (maintained dependencies and independent consumer contracts).
- Downstream consumer track in `edithatogo/tobac`: `conductor/tracks/dependency_ecosystem_codesign_20261110/`.

## Outputs

- Versioned neutral Arrow/Polars schema contracts for individual cohort state-transition records (initiation, dual-use, cessation, relapse, competing mortality) and spatial retail-outlet proximity exposure events.
- Deterministic common-random-numbers (CRN) stream partitioning contract across policy arms so paired policy counterfactuals preserve individual trajectory coupling without leaking domain labels into `kairo-ecs-rng`.
- Positive and negative contract fixtures verifying schema conformance, monotonic event clocks, competing-risk hazard conservation, and explicit rejection of mismatched units or unseeded RNG streams.
- Plans and specifications do not assert runtime implementation or downstream pin acceptance.

## Owned paths and interfaces

This track owns `conductor/tracks/116-tobac-cohort-microsimulation-contracts/`. Any future runtime source slices in `crates/kairo-ecs-des/`, `crates/kairo-ecs-abm/`, or `crates/kairo-ecs-arrow/` must be claimed with the original capability owners (`des-api-agent`, `abm-api-agent`, `arrow-agent`, `vv-uq-agent`) before implementation.

## Dependencies and parallel-safe tracks

Depends on Tracks 01, 03, 04, 69, 70, and 114. Coordinates with `innovate` (`semi_markov_competing_risks_20261110`), `dapane` (`tobac_regulatory_fiscal_valuation_20261110`), and `voiage` (`tobac_credal_endogenous_voi_20261110`) without coupling their internal crate graphs.

## Blocked paths

Tobacco/vaping transition matrices, NZHS/ASH Year 10 survey weights, excise tax formulas, and Māori/non-Māori equity weights belong to `tobac` and must never enter Kairos crates or public schemas.

## Acceptance criteria and quality gates

- **Domain neutrality**: Kairos schemas and Rust APIs use generic state/transition/exposure identifiers; zero tobacco-specific terminology appears in Kairos crates.
- **CRN counterfactual pairing**: Paired policy evaluations with identical seed and substream index reproduce identical pre-intervention trajectories and deterministic variance reduction on post-intervention diffs.
- **Competing-risk conservation**: Individual-level semi-Markov / competing-risk event tallies converge to analytical cohort survival/incidence oracles under homogeneous hazard fixtures.
- **Arrow/Polars zero-copy handoff**: Exported trajectory and cohort-summary batches validate against explicit schema versions and round-trip into `tobac`, `dapane`, and `voiage` adapters without silent coercion.

Required common gates: `phase-closeout-check`, `frontier-evidence-check`, and independent review.

## Risks and release implications

Individual-level microsimulation can introduce Monte Carlo variance that obscures small policy deltas if CRN substream synchronization is broken by state-dependent event counts. Substream-per-entity-and-transition addressing is mandatory before downstream consumer qualification.
