# Handoff: Track 21 Verification, Validation & Uncertainty

## Summary

Track 21 now has a concrete local definition for verification, validation, and
uncertainty, plus a page that anchors those terms to committed replay and
scenario fixtures.

## Files changed

`docs/trustworthy-simulation/verification-validation-uncertainty.md`
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

## Risks and unresolved questions

The main risk is overstating confidence when the evidence only supports a
narrower claim, especially if a scenario is reproducible but not yet validated
against reference data.
