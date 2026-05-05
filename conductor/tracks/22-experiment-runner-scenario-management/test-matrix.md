# Test Matrix: Track 22 Experiment Runner & Scenario Management

| Check | Required by alpha | Required by beta | Required by 1.0 |
|---|---:|---:|---:|
| Scenario manifest exists | yes | yes | yes |
| Replay or seed control is documented | yes | yes | yes |
| Markdown lint/link check | yes | yes | yes |
| Artifact existence check | yes | yes | yes |
| Docs build smoke test passes | yes | yes | yes |
| Release gate integration | no | yes | yes |
| Scenario output shape is documented | yes | yes | yes |
| Red-team objections about replay drift are answered | yes | yes | yes |
| Real fixture reference included (`scheduler_ordering_v1`) | yes | yes | yes |
| CLI scenario validation compiles | yes | yes | yes |
| Replay output shape smoke exists | yes | yes | yes |
| Resumability plan smoke exists | yes | yes | yes |

## Local validation commands

```bash
cargo check -p kairo-ecs-cli
cargo run -p kairo-ecs-cli -- validate-scenario --scenario examples/experiments/factory_bottleneck_v1.scenario.toml --seed-manifest examples/experiments/factory_bottleneck_v1.seeds.toml
cargo run -p kairo-ecs-cli -- replay --scenario examples/experiments/factory_bottleneck_v1.scenario.toml --seed-manifest examples/experiments/factory_bottleneck_v1.seeds.toml --output target/kairo-ecs-smoke/factory_bottleneck_v1
cargo run -p kairo-ecs-cli -- resume-plan --scenario examples/experiments/factory_bottleneck_v1.scenario.toml --output target/kairo-ecs-smoke/factory_bottleneck_v1
```
