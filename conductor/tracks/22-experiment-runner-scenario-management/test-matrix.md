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
| CLI docs page exists and is linked from the site | yes | yes | yes |
| Red-team objections about replay drift are answered | yes | yes | yes |
| Real fixture reference included (`scheduler_ordering_v1`) | yes | yes | yes |
| CLI scenario validation compiles | yes | yes | yes |
| Replay output shape smoke exists | yes | yes | yes |
| Resumability plan smoke exists | yes | yes | yes |
| Scenario index references real manifests and fixtures | yes | yes | yes |
| Read-only local scenario smoke validator passes | yes | yes | yes |
| Scenario claim boundary is machine-checked as local smoke only | yes | yes | yes |

## Local validation commands

```bash
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/scenarios/validate-track22-smoke.ps1
node scripts/validation/validate-track21-27-evidence-boundaries.mjs
cargo check -p kairo-ecs-cli
cargo run -p kairo-ecs-cli -- validate-scenario --scenario examples/experiments/factory_bottleneck_v1.scenario.toml --seed-manifest examples/experiments/factory_bottleneck_v1.seeds.toml
cargo run -p kairo-ecs-cli -- replay --scenario examples/experiments/factory_bottleneck_v1.scenario.toml --seed-manifest examples/experiments/factory_bottleneck_v1.seeds.toml --output target/kairo-ecs-smoke/factory_bottleneck_v1
cargo run -p kairo-ecs-cli -- resume-plan --scenario examples/experiments/factory_bottleneck_v1.scenario.toml --output target/kairo-ecs-smoke/factory_bottleneck_v1
node tests/conformance/conformance-check.mjs
node scripts/validation/validate-tracks21-27.mjs
```

## Latest focused validation evidence

| Command | Result | Evidence |
|---|---|---|
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/scenarios/validate-track22-smoke.ps1` | PASS | Returned `status: ok` for `factory_bottleneck_v1`, replay fixture `vvuq_scenario_replay_v1`, execution fixture `scheduler_ordering_v1`, kind order `1,2,4,3`, hash `1d53b73b244a84de`. |
| `cargo check -p kairo-ecs-cli` | PASS | Finished dev-profile check successfully. |
| `node tests/conformance/conformance-check.mjs` | PASS | Validated four ready fixtures: `scheduler_ordering_v1`, `scheduler_cancellation_v1`, `rng_reproducibility_v1`, `vvuq_scenario_replay_v1`. |
| `cargo run -p kairo-ecs-cli -- validate-scenario --scenario examples/experiments/factory_bottleneck_v1.scenario.toml --seed-manifest examples/experiments/factory_bottleneck_v1.seeds.toml` | FAIL/BLOCKED | Link step used Git for Windows `link.exe` at `C:\Users\60217257\scoop\apps\git\current\usr\bin\link.exe` and failed with `fatal error - couldn't create signal pipe, Win32 error 5`. |
## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, cleaned commit/push, and blocker recording.