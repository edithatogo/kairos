# Verification, Validation, and Uncertainty

This page defines how KairoECS treats model credibility. It is written for
track owners, reviewers, and users who need to know what has been checked,
what still carries uncertainty, and which artifacts are enough to support a
claim.

## Terms

- Verification: evidence that the implementation behaves as specified.
- Validation: evidence that the model is fit for the intended use against
  reference data, domain expectations, or scenario outcomes.
- Uncertainty: the remaining spread in outputs caused by stochasticity,
  parameter ranges, calibration choices, and scenario assumptions.

## Evidence boundary

A verification, validation, or uncertainty claim is supported only by
committed, replayable artifacts. For Track 21, the boundary is:

- scenario manifests
- seed manifests
- event traces
- entity snapshot summaries
- Arrow or Parquet telemetry
- summary statistics
- replay commands
- validation reports when reference data exists
- uncertainty reports for repeated runs

Narrative text alone is not enough. Screenshots, email threads, and ad hoc
notes may explain a result, but they are not accepted evidence unless they
resolve to the artifacts above.

## Accepted artifacts

Track 21 accepts the following artifacts as the source of truth for credibility
claims:

- a committed scenario fixture describing the model, inputs, and environment
- a committed seed fixture or seed manifest
- a deterministic replay command that reproduces the same trace or summary
- a trace fixture that can be compared across versions
- a validation report tied to reference data or an explicit validation basis
- an uncertainty report for replications, sensitivity runs, or scenario sweeps
- a short interpretation note that states what was tested and what remains
  uncertain

## Fixture ties

Track 21 is tied to replayable scenario fixtures rather than prose-only
assertions. A useful credibility path is:

1. Define the scenario in a manifest.
2. Record the seed allocation.
3. Emit the event trace and summary statistics.
4. Replay the run from the committed fixtures.
5. Compare the replay against the stored trace or expected summary.
6. Publish the validation or uncertainty report that explains the result.

This is the same pattern used by the deterministic replay and scenario fixtures
elsewhere in the repo. Track 21 makes the interpretation layer explicit:
what the fixture proves, what it does not prove, and where uncertainty remains.

## Reading a claim

When a release note, docs page, or example says a result is verified,
validated, or uncertain, read it as follows:

- Verified means the implementation matched the expected replay or trace
  behavior.
- Validated means the result was compared with reference data or an accepted
  domain basis.
- Uncertain means the output still depends on stochastic runs, scenario
  choices, or parameter ranges that are not fixed by the current evidence.

If the page does not name the scenario, seed, trace, and comparison basis, the
claim is incomplete.
