# Factory Bottleneck v1 VVUQ Note

## Claim

`factory_bottleneck_v1` has artifact-backed verification smoke coverage for
deterministic scheduler ordering. The current evidence does not establish
real-world validation or quantified uncertainty.

## Evidence Artifacts

| Role | Artifact |
|---|---|
| VVUQ fixture | `conformance/fixtures/vvuq_scenario_replay.json` |
| Scenario manifest | `examples/experiments/factory_bottleneck_v1.scenario.toml` |
| Seed manifest | `examples/experiments/factory_bottleneck_v1.seeds.toml` |
| Replay fixture | `conformance/fixtures/deterministic_ordering.json` |
| Fixture ID | `scheduler_ordering_v1` |
| Comparison basis | `expected_kind_order` |

## Verified Behavior

The scenario and seed manifests bind `factory_bottleneck_v1` to
`scheduler_ordering_v1`. The conformance fixture defines four events and the
expected event-kind order `[1, 2, 4, 3]` after sorting by time, priority, and
sequence. The VVUQ fixture records the same comparison basis and required replay
outputs:

- `manifest.json`
- `summary.json`
- `replay-comparison.json`
- `resumability-plan.json`

The local check for this note validates that those references still agree. This
is verification of fixture wiring and deterministic ordering semantics, not a
broader model-validity result.

## Validation Status

No reference data or accepted domain benchmark is attached to this scenario.
The scenario must therefore be described as unvalidated for real-world queueing,
throughput, or bottleneck claims.

Validation can be claimed only after a future report names the reference data,
the comparison statistic, the acceptance threshold, and the release decision
that depends on the comparison.

## Uncertainty Status

The scenario declares `replications = 2`, but the current VVUQ fixture does not
contain repeated-run output distributions, confidence intervals, or sensitivity
sweeps. The uncertainty claim is therefore limited to identifying what remains
unknown:

- stochastic spread across seeds
- sensitivity to resource and arrival-process parameters
- sensitivity to the chosen max-event cutoff
- cross-platform replay stability beyond this fixture

## Local Check

```bash
node scripts/validation/validate-vvuq-note.mjs
node tests/conformance/conformance-check.mjs
```

Passing these commands means the note is still anchored to committed fixtures
and the conformance fixture still passes its local replay assertion.
