# Handoff: Track 21 Verification, Validation & Uncertainty

## Summary

Track 21 now has a concrete local definition for verification, validation, and
uncertainty, plus a page that anchors those terms to committed replay and
scenario fixtures.

## Files changed

`docs/trustworthy-simulation/verification-validation-uncertainty.md`
`docs/trustworthy-simulation/scenario-evidence.md`
`docs/validation/factory-bottleneck-v1-vvuq-note.md`
`website/docs-link-manifest.json`
`website/src/index.md`
`conductor/tracks/21-verification-validation-uncertainty/handoff.md`
`conductor/tracks/21-verification-validation-uncertainty/test-matrix.md`
`conductor/tracks/21-verification-validation-uncertainty/risk-register.md`

## Contracts consumed

`conductor/tracks/21-verification-validation-uncertainty/spec.md`
`conductor/tracks/21-verification-validation-uncertainty/plan.md`
`conductor/workflow.md`

## Evidence boundary

The track treats committed scenario fixtures, seed manifests, replay commands,
event traces, summary statistics, validation reports, and uncertainty reports
as the accepted evidence set. Prose by itself is not enough.

## Accepted artifacts

- deterministic replay command
- committed scenario fixture
- committed seed manifest
- trace fixture or trace summary
- validation report tied to reference data
- uncertainty report tied to repeated runs
- interpretation note that states limits and assumptions

## Integration notes

The docs page should stay aligned with replay/scenario fixtures so a reader can
move from a claim to a reproducible run without guessing at hidden inputs.
The public docs surface now also links the artifact-backed VVUQ note from the
site navigation and the scenario-evidence page.

## R2 local evidence slice

Added a committed VVUQ scenario replay fixture:

- `conformance/fixtures/vvuq_scenario_replay.json`
- `docs/trustworthy-simulation/scenario-evidence.md`
- `examples/experiments/factory_bottleneck_v1.scenario.toml`
- `examples/experiments/factory_bottleneck_v1.seeds.toml`

The fixture is deliberately narrow. It verifies deterministic replay for
`scheduler_ordering_v1` and records the claim boundary as verification smoke
only. It does not claim real-world validation or quantified uncertainty.

Validation evidence:

```bash
node tests/conformance/conformance-check.mjs
cargo check -p kairo-ecs-cli
```

## Current Worker 4 slice - 2026-05-06

Added an artifact-backed validation and uncertainty note:

- `docs/validation/factory-bottleneck-v1-vvuq-note.md`
- `scripts/validation/validate-vvuq-note.mjs`

The note is anchored to `conformance/fixtures/vvuq_scenario_replay.json`,
`examples/experiments/factory_bottleneck_v1.scenario.toml`,
`examples/experiments/factory_bottleneck_v1.seeds.toml`, and
`conformance/fixtures/deterministic_ordering.json`. It records the narrow
verified behavior, states that real-world validation is not yet supported, and
limits uncertainty claims to explicit unknowns until repeated-run artifacts
exist.

Validation evidence:

```bash
node scripts/validation/validate-vvuq-note.mjs
node tests/conformance/conformance-check.mjs
```

Integration note for the main thread: do not mark the Track 21 delivery
readiness checkbox complete from this slice alone. The safe integration wording
is: "Track 21 has a fixture-backed VVUQ note and local note/fixture checks for
`factory_bottleneck_v1`; release-readiness still needs reference-data
validation evidence and quantitative uncertainty artifacts before broader
model-credibility claims are allowed."

## Risks and unresolved questions

The main risk is overstating confidence when the evidence only supports a
narrower claim, especially if a scenario is reproducible but not yet validated
against reference data.

## Contracts changed

No VVUQ contracts changed in this scoped cleanup. The accepted evidence remains the replay fixture, scenario and seed files, deterministic ordering fixture, and artifact-backed VVUQ note.

## Tests added

No executable tests were added in this scoped cleanup. Existing evidence remains `node scripts/validation/validate-vvuq-note.mjs` plus the conformance checks listed above.

## Known risks

The current evidence supports deterministic replay smoke only; real-world validation and quantified uncertainty remain unsupported until reference-data and repeated-run artifacts exist.

## Follow-up issues

Add reference-data validation evidence and quantitative uncertainty artifacts before broad model-credibility claims are made.
## Phase closeout evidence

2026-05-11 implementation/review pass:

- `$conductor-review` finding: the VVUQ note and scenario-evidence slice are
  stable, the evidence boundary remains explicit, and the cross-track validator
  still rejects broadened claims.
- Accepted fixes: retained the narrow fixture-backed boundary and kept
  quantitative uncertainty claims deferred to repeated-run evidence.
- Validation commands passed: `node scripts/validation/validate-vvuq-note.mjs`,
  `node scripts/validation/validate-track21-27-evidence-boundaries.mjs`,
  `node tests/conformance/conformance-check.mjs`, and
  `node scripts/validation/validate-tracks21-27.mjs`.
- Cleanup state: no generated artifacts were retained.
- commit SHA: `f4fe05f483569d3ae46070f2e80bda28930b0d23`
- pushed ref: `origin/conductor-close-reviewed-tracks-20260510`
- `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`: passed after
  the closeout commit was recorded and pushed.
- next-phase decision: Track 21 is `Done`; keep public VVUQ claims bounded to
  the fixture-backed note and its explicit evidence boundary.
