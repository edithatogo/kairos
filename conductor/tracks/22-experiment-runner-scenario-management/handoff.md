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

## R2 local implementation slice

Added the first concrete Track 22 runner surface in `crates/kairo-ecs-cli/`:

- `validate-scenario --scenario <path> --seed-manifest <path>`
- `replay --scenario <path> --seed-manifest <path> --output <dir>`
- `resume-plan --scenario <path> --output <dir>`

The R2 slice intentionally supports only the committed
`factory_bottleneck_v1` smoke scenario and `scheduler_ordering_v1` replay
fixture. It writes the expected local output shape:

- `manifest.json`
- `summary.json`
- `replay-comparison.json`
- `resumability-plan.json`

Validation evidence:

```bash
cargo check -p kairo-ecs-cli
node tests/conformance/conformance-check.mjs
```

On this Windows session, `cargo run` reached linking but failed because PATH
resolved `link.exe` to Git for Windows instead of the MSVC linker. A fallback
`rust-lld` attempt also failed because the Windows SDK import libraries were
not discoverable. The runner commands above should execute once the MSVC linker
and Windows SDK libraries are available on PATH/LIB.

## Risks and unresolved questions

The main risk is that a runner can look deterministic in one environment and drift in another if the scenario inputs are not pinned.
