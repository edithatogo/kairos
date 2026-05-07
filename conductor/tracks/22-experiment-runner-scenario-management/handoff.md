# Handoff: Track 22 Experiment Runner & Scenario Management

Last updated: 2026-05-07

## Summary

Captured the scenario-management story so other tracks can assume named runs, replay inputs, documented output shape, and a concrete replay comparison flow.

## Files changed

`scenarios/manifest-index.json`, `docs/scenarios/factory-bottleneck-run-replay.md`, `docs/cli/kairo-ecs-cli.md`, `scripts/scenarios/validate-track22-smoke.ps1`, `conductor/tracks/22-experiment-runner-scenario-management/test-matrix.md`, `conductor/tracks/22-experiment-runner-scenario-management/risk-register.md`, `conductor/tracks/22-experiment-runner-scenario-management/handoff.md`

## Contracts consumed

`conformance/fixtures/manifest.json`, `conformance/fixtures/vvuq_scenario_replay.json`, `conformance/fixtures/deterministic_ordering.json`, `examples/experiments/factory_bottleneck_v1.scenario.toml`, `examples/experiments/factory_bottleneck_v1.seeds.toml`, `conductor/workflow.md`

## Release gates affected

Scenario replay and determinism checks now feed the reproducibility and release gate surface.

## Concrete runner note

- Scenario index: `scenarios/manifest-index.json`.
- Scenario manifest: `examples/experiments/factory_bottleneck_v1.scenario.toml`.
- Seed manifest: `examples/experiments/factory_bottleneck_v1.seeds.toml`.
- Replay command: `cargo run -p kairo-ecs-cli -- replay --scenario examples/experiments/factory_bottleneck_v1.scenario.toml --seed-manifest examples/experiments/factory_bottleneck_v1.seeds.toml --output target/kairo-ecs-smoke/factory_bottleneck_v1`.
- Output shape: `manifest.json`, `summary.json`, `replay-comparison.json`, `resumability-plan.json`.
- Comparison flow: load manifest and seeds, replay the fixture, compare emitted trace and summary metrics, emit a drift report.
- Real fixture references: `vvuq_scenario_replay_v1` and `scheduler_ordering_v1` from `conformance/fixtures/manifest.json`.

## Scenario manifest/run/replay note

`docs/scenarios/factory-bottleneck-run-replay.md` now records the first usable
Track 22 scenario smoke target, while `docs/cli/kairo-ecs-cli.md` provides the
matching command reference and quickstart:

- Scenario ID: `factory_bottleneck_v1`.
- Replay fixture: `vvuq_scenario_replay_v1`.
- Execution fixture: `scheduler_ordering_v1`.
- Comparison basis: `expected_kind_order`.
- Expected event kind order: `1,2,4,3`.
- Expected summary hash: `1d53b73b244a84de`.
- Claim boundary: verification smoke only; not a real-world validation or
  uncertainty claim.

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
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/scenarios/validate-track22-smoke.ps1
cargo check -p kairo-ecs-cli
node tests/conformance/conformance-check.mjs
cargo run -p kairo-ecs-cli -- validate-scenario --scenario examples/experiments/factory_bottleneck_v1.scenario.toml --seed-manifest examples/experiments/factory_bottleneck_v1.seeds.toml
```

- PASS: `validate-track22-smoke.ps1` returned `status: ok` for
  `factory_bottleneck_v1`, `vvuq_scenario_replay_v1`, `scheduler_ordering_v1`,
  expected kind order `1,2,4,3`, and expected summary hash
  `1d53b73b244a84de`.
- PASS: `cargo check -p kairo-ecs-cli` completed in the dev profile.
- PASS: `node tests/conformance/conformance-check.mjs` validated four ready
  fixtures, including `vvuq_scenario_replay_v1`.
- FAIL/BLOCKED: `cargo run -p kairo-ecs-cli -- validate-scenario ...` reached
  link and failed because `link.exe` resolved to
  `C:\Users\60217257\scoop\apps\git\current\usr\bin\link.exe`, which returned
  `fatal error - couldn't create signal pipe, Win32 error 5`. The CLI replay
  and resume commands remain blocked until the MSVC linker/Windows SDK path is
  corrected for this shell.

The new CLI docs page stays intentionally narrow: it documents the implemented
smoke commands, reserves `run`/`collect`/`analyze` for the fuller runner
surface, and points readers at the local smoke validator.

## Risks and unresolved questions

The main risk is that a runner can look deterministic in one environment and drift in another if the scenario inputs are not pinned.

## Contracts changed

Track 22 now documents the implemented smoke-command surface for `validate-scenario`, `replay`, and `resume-plan`, with `factory_bottleneck_v1` as the only claimed scenario target.

## Tests added

The current evidence is `scripts/scenarios/validate-track22-smoke.ps1`, `cargo check -p kairo-ecs-cli`, and `node tests/conformance/conformance-check.mjs`. Local `cargo run` validation remains blocked by the Windows linker path issue recorded above.

## Known risks

Scenario replay drift remains the main track risk. The local shell also cannot yet provide executable CLI replay evidence until `link.exe` resolves to the MSVC linker/Windows SDK toolchain.

## Follow-up issues

Fix the local Windows linker path, then rerun the `validate-scenario`, `replay`, and `resume-plan` commands. Broaden the runner beyond the committed smoke scenario only after the parser and resumability tests have real coverage.

## Integration notes

Tracks 21, 28, and release planning may consume only the smoke evidence named here; they should not treat it as quantified uncertainty evidence or production experiment-runner readiness.
