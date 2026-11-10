# Handoff

Updated 10 November 2026 (Australia/Brisbane). Status: Spec Approved; no implementation phase is accepted by this registration.

## Summary

Registered Track 116 (`116-tobac-cohort-microsimulation-contracts`) to coordinate neutral DES/ABM cohort-trajectory, semi-Markov competing-risk, CRN substream coupling, and spatial retail-exposure Arrow/Polars contracts for `edithatogo/tobac` (`conductor/tracks/dependency_ecosystem_codesign_20261110/`). All tobacco/vaping epidemiology, regulatory policy levers, and New Zealand demographic inputs remain strictly owned by `tobac`.

## Files changed

This track's specification, plan, metadata, agent contract, risk register, test matrix, and index; plus registry entries in `conductor/tracks.yaml`, `conductor/tracks.md`, `conductor/ecosystem-consumers.md`, and `conductor/tracks/114-dependency-and-consumer-contracts/index.md`.

## Contracts consumed

`AGENTS.md`, `docs/adr/0011-ecosystem-boundaries.md`, `conductor/ecosystem-consumers.md`, and Tracks 01, 03, 04, 69, 70, and 114.

## Contracts changed

Planning registration only. Defines the target neutral cohort-transition and CRN substream handoff boundary for `tobac`; no Rust crate API, ABI, or dependency pin is changed by this registration.

## Tests added

`test-matrix.md` defines the required positive/negative schema, CRN coupling, and analytical semi-Markov convergence oracles for Phases 0–2.

## Known risks

Uncoupled RNG substreams under variable event counts can inflate Monte Carlo variance between policy arms; domain terminology must not leak into neutral Kairos crates. Both risks are bound to explicit negative and convergence gates in `risk-register.md` and `test-matrix.md`.

## Follow-up issues

Execute Phases 0–2 in `plan.md` before `tobac` adopts an installed Kairos microsimulation adapter pin.

## Integration notes

Keep Kairos neutral and domain-free. Coordinate downstream consumer qualification in `edithatogo/tobac` under `dependency_ecosystem_codesign_20261110`.

## Phase closeout evidence

No phase is closed. Planning registration is not execution or scientific validation evidence.
