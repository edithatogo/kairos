# Factory Bottleneck Scenario Run and Replay Smoke

This note defines the first Track 22 scenario that can be used as a local
smoke target by other tracks.

## Scenario identity

| Field | Value |
|---|---|
| Scenario ID | `factory_bottleneck_v1` |
| Scenario manifest | `examples/experiments/factory_bottleneck_v1.scenario.toml` |
| Seed manifest | `examples/experiments/factory_bottleneck_v1.seeds.toml` |
| Scenario index | `scenarios/manifest-index.json` |
| Replay fixture | `vvuq_scenario_replay_v1` |
| Execution fixture | `scheduler_ordering_v1` |
| Comparison basis | `expected_kind_order` |
| Expected event kind order | `1,2,4,3` |
| Expected summary hash | `1d53b73b244a84de` |

## Local smoke validator

Run the read-only validator before making release or replay claims:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/scenarios/validate-track22-smoke.ps1
```

The validator checks that:

- `scenarios/manifest-index.json` names the real scenario and seed manifests.
- The scenario and seed manifests agree on `scenario_id`, `base_seed`, and
  `fixture_id`.
- `conformance/fixtures/manifest.json` contains ready entries for
  `vvuq_scenario_replay_v1` and `scheduler_ordering_v1`.
- `conformance/fixtures/vvuq_scenario_replay.json` points back to the same
  scenario and seed manifests.
- `conformance/fixtures/deterministic_ordering.json` sorts to event kind order
  `1,2,4,3` using time, priority, and sequence.
- The required replay output names are present in the index and replay fixture.

## CLI run and replay notes

When the local Rust toolchain can link `kairo-ecs-cli`, the concrete smoke
commands are:

```powershell
cargo run -p kairo-ecs-cli -- validate-scenario --scenario examples/experiments/factory_bottleneck_v1.scenario.toml --seed-manifest examples/experiments/factory_bottleneck_v1.seeds.toml
cargo run -p kairo-ecs-cli -- replay --scenario examples/experiments/factory_bottleneck_v1.scenario.toml --seed-manifest examples/experiments/factory_bottleneck_v1.seeds.toml --output target/kairo-ecs-smoke/factory_bottleneck_v1
cargo run -p kairo-ecs-cli -- resume-plan --scenario examples/experiments/factory_bottleneck_v1.scenario.toml --output target/kairo-ecs-smoke/factory_bottleneck_v1
```

Expected replay artifacts:

- `manifest.json`
- `summary.json`
- `replay-comparison.json`
- `resumability-plan.json`

## Replay acceptance criteria

- The scenario manifest schema is `kairoecs.scenario.v1`.
- The seed manifest schema is `kairoecs.seed.v1`.
- The seed manifest and scenario manifest identify the same scenario,
  base seed, and execution fixture.
- The replay fixture remains tied to `scheduler_ordering_v1`.
- The deterministic ordering fixture emits event kind order `1,2,4,3`.
- The output directory includes the four expected replay artifact names.

## Red-team prompts

- Can a hidden input, environment variable, or working-directory difference
  change the selected fixture path?
- Can repeated runs leak state through an existing output directory?
- Does replay compare semantic output, or only prove that files exist?
- Is the summary hash tied to stable scenario inputs rather than wall-clock
  or machine-specific metadata?

## Devil's advocate objections

- This smoke scenario proves scheduler ordering only; it is not a domain
  validation of a factory model.
- The current smoke target uses a small deterministic fixture, so it cannot
  prove parameter-sweep resumability under failure.
- Cross-machine trust still requires the CLI replay command to run under CI
  with a pinned toolchain and clean output directory.

## Claim boundary

This scenario supports a Track 22 verification smoke claim only. It does not
support a real-world validation, uncertainty quantification, performance, or
production reproducibility claim until broader runner outputs and CI replay
evidence exist.
