# Replay and Seed Manifests

Deterministic replay is central to KairoECS.

```yaml
schema_version: kairoecs.seed.v1
scenario_id: factory_bottleneck_v1
base_seed: 12345
streams:
  arrival_process: 1001
  service_process: 1002
  agent_42: 42042
fixture_id: scheduler_ordering_v1
fixture_path: conformance/fixtures/deterministic_ordering.json
```

```bash
kairoecs-experiment replay --scenario scenarios/factory_bottleneck_v1.yaml --seed-manifest seeds.yaml --fixture scheduler_ordering_v1 --output runs/factory_bottleneck_v1
```

The replay output should include:

- a run manifest for the selected scenario
- a Parquet or Arrow event log for the replayed replications
- a JSON comparison report against the fixture identified in the seed manifest

Use `scheduler_ordering_v1` for the first end-to-end replay check because it has a real fixture source and stable assertions in `conformance/fixtures/manifest.json`.

The committed Track 22 smoke manifests are:

- `examples/experiments/factory_bottleneck_v1.scenario.toml`
- `examples/experiments/factory_bottleneck_v1.seeds.toml`

They can be checked locally with:

```bash
cargo run -p kairo-ecs-cli -- validate-scenario --scenario examples/experiments/factory_bottleneck_v1.scenario.toml --seed-manifest examples/experiments/factory_bottleneck_v1.seeds.toml
cargo run -p kairo-ecs-cli -- replay --scenario examples/experiments/factory_bottleneck_v1.scenario.toml --seed-manifest examples/experiments/factory_bottleneck_v1.seeds.toml --output target/kairo-ecs-smoke/factory_bottleneck_v1
```
