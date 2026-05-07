# Scenario Evidence

Track 21 credibility claims and Track 22 experiment-runner claims now share one
local smoke path:

- Scenario manifest:
  `examples/experiments/factory_bottleneck_v1.scenario.toml`
- Seed manifest:
  `examples/experiments/factory_bottleneck_v1.seeds.toml`
- Replay fixture:
  `conformance/fixtures/deterministic_ordering.json`
- Fixture ID:
  `scheduler_ordering_v1`
- Comparison basis:
  `expected_kind_order`

The scenario is a verification fixture, not a validation study. It proves that
the runner can load a committed scenario, bind it to a committed seed manifest,
execute the deterministic scheduler ordering smoke, compare the observed event
order to the fixture expectation, and write replay evidence.

The artifact-backed interpretation note for this smoke path is
[`docs/validation/factory-bottleneck-v1-vvuq-note.md`](../validation/factory-bottleneck-v1-vvuq-note.md),
and the local checker is `node scripts/validation/validate-vvuq-note.mjs`.

It does not prove real-world model validity, queueing-theory accuracy, or
platform-scale resumability. Those claims need reference data, larger
replications, and interruption/restart evidence.

## Local Smoke Commands

```bash
cargo run -p kairo-ecs-cli -- validate-scenario --scenario examples/experiments/factory_bottleneck_v1.scenario.toml --seed-manifest examples/experiments/factory_bottleneck_v1.seeds.toml
cargo run -p kairo-ecs-cli -- replay --scenario examples/experiments/factory_bottleneck_v1.scenario.toml --seed-manifest examples/experiments/factory_bottleneck_v1.seeds.toml --output target/kairo-ecs-smoke/factory_bottleneck_v1
cargo run -p kairo-ecs-cli -- resume-plan --scenario examples/experiments/factory_bottleneck_v1.scenario.toml --output target/kairo-ecs-smoke/factory_bottleneck_v1
```

Expected replay artifacts:

- `manifest.json`
- `summary.json`
- `replay-comparison.json`
- `resumability-plan.json`
