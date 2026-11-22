# Track 116 implementation plan

Provide neutral DES/ABM cohort-trajectory, competing-risk, and spatial exposure contracts for `tobac` microsimulation cross-checks. The provider contract and standalone fixture are qualified. The receiving applications own their domain inputs and installed adapter round-trips.

## Phase 0: Contract and fixture boundary

- [x] 116.0.1: Specify neutral typed Arrow/Polars cohort transition and exposure schemas with full-width identity and time.
- [x] 116.0.2: Define paired draw addresses by entity, source state, visit and slot.
- [x] 116.0.3: Exercise negative controls for clocks, hazard rates, repeated visits, units and RNG provenance.
- [x] 116.0.review: Independently review the contracts and execute the named oracles.

## Phase 1: Competing-risk and spatial exposure adapters

- [x] 116.1.1: Feed real Track 69 competing-risk draws into the cohort trajectory exporter in the standalone fixture.
- [x] 116.1.2: Feed real spatial-index proximity queries into the exposure exporter.
- [x] 116.1.3: Compare imported trajectory outcomes from 100,000 entities with an independent piecewise survival and incidence reference.
- [x] 116.1.review: Review independently, apply the accepted fixes and rerun the oracles.

## Phase 2: Downstream `tobac` qualification handoff

- [x] 116.2.1: Deliver the locked, fictional standalone consumer and prove execution from clean source.
- [x] 116.2.2: Record the provider handoff for `edithatogo/tobac` in the existing handoff.
- [x] 116.2.review: Independently review the standalone fixture and its acceptance evidence.
- [ ] Qualify installed receiving adapters against the versioned schemas, including downstream cohort summaries.

## Phase closeout gate

Before completing a phase, run `$conductor-review` with an independent reviewer and auto-apply accepted review fixes within the owned slice. Synchronize `conductor/tracks.yaml`, `conductor/tracks.md`, `conductor/phase-closeout.yaml` and `conductor/status.md`. Run `scripts/validate_conductor_phase_gates.ps1` and the specific test matrix. Commit and push the cleaned slice; then run `scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`. Record exact source, command, input/toolchain/artefact identity, exit status and the next-phase decision. No phase advances from planning or metadata checks alone.
