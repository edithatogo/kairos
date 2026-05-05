# Handoff: Track 22 Experiment Runner & Scenario Management

## Summary

Captured the scenario-management story so other tracks can assume named runs, replay inputs, documented output shape, and a concrete replay comparison flow.

## Files changed

`conductor/experiment-runner.md`, `docs/trustworthy-simulation/replay-and-seeds.md`, `conductor/tracks/22-experiment-runner-scenario-management/experiment-runner-plan.md`, `conductor/tracks/22-experiment-runner-scenario-management/test-matrix.md`, `conductor/tracks/22-experiment-runner-scenario-management/handoff.md`

## Contracts consumed

`conformance/fixtures/manifest.json`, `benches/benchmark-plan.md`, `conductor/workflow.md`, `website/`

## Release gates affected

Scenario replay and determinism checks now feed the reproducibility and release gate surface.

## Concrete runner note

- Scenario manifest shape: `schema_version`, `scenario_id`, `model.id`, `parameters`, `runs.replications`, `seed_policy.base_seed`, `replay.fixture_id`, `outputs.format`, `outputs.artifact_root`.
- Replay command: `kairoecs-experiment replay --scenario scenarios/factory_bottleneck_v1.yaml --seed-manifest scenarios/seeds.yaml --fixture scheduler_ordering_v1 --output runs/factory_bottleneck_v1`.
- Output shape: `manifest.json`, `replications.parquet`, `summary.json`, `replay-comparison.json`.
- Comparison flow: load manifest and seeds, replay the fixture, compare emitted trace and summary metrics, emit a drift report.
- Real fixture reference: `scheduler_ordering_v1` from `conformance/fixtures/manifest.json`.

## Risks and unresolved questions

The main risk is that a runner can look deterministic in one environment and drift in another if the scenario inputs are not pinned.
