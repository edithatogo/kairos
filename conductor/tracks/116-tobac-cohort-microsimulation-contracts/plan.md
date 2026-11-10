# Track 116 implementation plan

Provide neutral DES/ABM cohort-trajectory, competing-risk, and spatial exposure contracts for `tobac` microsimulation cross-checks while keeping all tobacco/vaping domain assumptions in `tobac`. Every implementation item below remains pending.

## Phase 0: Contract and fixture boundary

- [ ] 116.0.1: Specify the neutral Arrow/Polars cohort transition and exposure record schemas (`entity_id`, `stratum_id`, `from_state`, `to_state`, `dwell_time`, `event_time`, `rng_substream_id`, `spatial_cell_id`) and align them with Track 114 movement/usage boundaries.
- [ ] 116.0.2: Define the deterministic entity-level CRN substream addressing contract so paired policy arms remain coupled even when event counts diverge across interventions.
- [ ] 116.0.3: Define positive and negative contract fixtures (monotonic clock violation, negative hazard rate, duplicate entity state, mismatched time unit, uncoupled RNG stream).
- [ ] 116.0.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision.

## Phase 1: Competing-risk and spatial exposure adapters

- [ ] 116.1.1: Bind Track 69 competing-risk / non-Markovian dwell-time sampling to the neutral cohort transition batch exporter without domain-specific state names.
- [ ] 116.1.2: Bind Track 03/86/87 spatial proximity queries to neutral outlet-density/distance exposure covariates for ABM cohort steps.
- [ ] 116.1.3: Verify large-sample convergence of the microsimulation competing-risk kernel against an independent analytical semi-Markov cohort oracle.
- [ ] 116.1.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision.

## Phase 2: Downstream `tobac` qualification handoff

- [ ] 116.2.1: Publish a minimal public fictional cohort fixture that runs without `tobac` installed and verifies Arrow/Polars batch export and CRN replay identity.
- [ ] 116.2.2: Record the handoff receipt for `edithatogo/tobac` (`dependency_ecosystem_codesign_20261110`); keep `tobac` pin upgrades in `tobac`'s own repository after installed qualification.
- [ ] 116.2.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision.

## Phase closeout gate

Before completing a phase, run `$conductor-review` with an independent reviewer and auto-apply accepted review fixes within the owned slice. Synchronize `conductor/tracks.yaml`, `conductor/tracks.md`, `conductor/phase-closeout.yaml` and `conductor/status.md`. Run `scripts/validate_conductor_phase_gates.ps1` and the specific test matrix. Commit and push the cleaned slice; then run `scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`. Record exact source, command, input/toolchain/artefact identity, exit status and the next-phase decision. No phase advances from planning or metadata checks alone.
