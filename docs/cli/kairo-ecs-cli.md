# kairo-ecs-cli

`kairo-ecs-cli` is the local experiment runner for Track 22. The checked-in R2
slice supports scenario validation, deterministic replay, and resumability plan
generation for the committed `factory_bottleneck_v1` smoke scenario.

## Commands

- `validate-scenario --scenario <path> --seed-manifest <path>`
- `replay --scenario <path> --seed-manifest <path> --output <dir>`
- `resume-plan --scenario <path> --output <dir>`

The following commands are reserved for the fuller Track 22 runner surface:

- `run`
- `collect`
- `analyze`

## Quickstart

```powershell
cargo run -p kairo-ecs-cli -- --help
cargo run -p kairo-ecs-cli -- validate-scenario --scenario examples/experiments/factory_bottleneck_v1.scenario.toml --seed-manifest examples/experiments/factory_bottleneck_v1.seeds.toml
```

When the Windows linker is configured correctly, the replay smoke is:

```powershell
cargo run -p kairo-ecs-cli -- replay --scenario examples/experiments/factory_bottleneck_v1.scenario.toml --seed-manifest examples/experiments/factory_bottleneck_v1.seeds.toml --output target/kairo-ecs-smoke/factory_bottleneck_v1
cargo run -p kairo-ecs-cli -- resume-plan --scenario examples/experiments/factory_bottleneck_v1.scenario.toml --output target/kairo-ecs-smoke/factory_bottleneck_v1
```

## Output Shape

The replay command writes the local smoke evidence set:

- `manifest.json`
- `summary.json`
- `replay-comparison.json`
- `resumability-plan.json`

## Local Validation

- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/scenarios/validate-track22-smoke.ps1`
- `cargo check -p kairo-ecs-cli`
- `node tests/conformance/conformance-check.mjs`

## Related Files

- `docs/scenarios/factory-bottleneck-run-replay.md`
- `docs/trustworthy-simulation/scenario-evidence.md`
- `scenarios/manifest-index.json`
- `conductor/tracks/22-experiment-runner-scenario-management/handoff.md`
