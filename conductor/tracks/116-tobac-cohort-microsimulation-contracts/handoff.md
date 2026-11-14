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

Record the following evidence when an implementation phase reaches closeout;
these pending fields do not certify the registration or advance a phase.

| Evidence | Current state |
|---|---|
| `$conductor-review` | Pending independent review of the implemented phase and its acceptance oracles. |
| accepted fixes | Pending the phase review; record applied fixes and unresolved findings when they exist. |
| commit SHA | Pending the reviewed implementation commit; the registration commit is not runtime evidence. |
| pushed ref | Pending publication of that reviewed implementation commit. |
| `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Required after the reviewed commit is pushed; no implementation closeout result is claimed. |
| next-phase decision | Remain Spec Approved. All Phase 0–2 tasks are pending; no implementation phase may advance from structural checks alone. |

## Registration repair — 14 November 2026

At GitHub main `aeb52c937f1ffdc01e3dc5064cceae5134bf2790`, the existing
phase-gate validator reported six missing handoff evidence fields and three
missing test-matrix requirements. The same nine errors were reproduced locally
with PowerShell 7.7.0. This repair adds the pending evidence fields above and
names the existing phase/clean-source gates in `test-matrix.md`.

Only those two documentation files change. The validator, scientific tasks,
registry status, Rust-only execution policy, ownership boundaries and consumer
pins are unchanged. Candidate validation and exact-head hosted results belong
to the repair PR; they do not close an implementation phase.
