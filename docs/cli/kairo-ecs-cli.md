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

## Experimental C5 finite-study commands

The commands below are an experimental, Rust-native lane for the normalized
synthetic study in [`examples/calibration/c5`](../../examples/calibration/c5/README.md).
They do not replace or change the legacy scenario/seed commands above. C5 currently
accepts versioned synthetic JSON with integer tick values and declared training,
validation, and final-test partitions. It is not a general trace importer or a
clinical/operational calibration tool. See the [C5 implementation contract](../../conductor/design/calibration/ADR-0022-c5-study-runner.md)
for schema and validation boundaries.

From the Kairos repository root, build with the pinned Rust 1.99 toolchain and run
the binary directly:

```sh
cargo +1.99.0 build -p kairo-ecs-cli
./target/debug/kairo-ecs-cli verify trace --study examples/calibration/c5/study.json
./target/debug/kairo-ecs-cli replay --study examples/calibration/c5/study.json --mode shadow --output .artifacts/c5/shadow
./target/debug/kairo-ecs-cli replay --study examples/calibration/c5/study.json --mode free --output .artifacts/c5/free
```

`verify trace` checks the study's normalized records and partition/censor
boundaries. Shadow replay anchors observations from the supplied trace and runs
independent native probes. Free replay uses a shared FCFS native resource; observed
completions do not drive native time or state. Both outputs identify the synthetic
adapter and replay mode. Each output path must be new.

Run a study with a chosen worker count. The output directory must not already exist
unless `--resume` names that same directory:

```sh
./target/debug/kairo-ecs-cli calibrate --study examples/calibration/c5/study.json --output .artifacts/c5/run-w1 --workers 1
```

The run stores provenance, initial and final task shards, the validation-stage
summary, the final summary, and a run receipt. Work is durable at completed task
boundaries; resume does not claim to checkpoint a live simulation world. Resume
requires the same exact study bytes, executable, source/build and lock provenance,
and any optional scenario/seed manifest bytes used for the original run.

To exercise cooperative stop and resume in a fresh process, use the same directory
and binary for both commands:

```sh
./target/debug/kairo-ecs-cli calibrate --study examples/calibration/c5/study.json --output .artifacts/c5/resume --workers 1 --stop-after 3
./target/debug/kairo-ecs-cli calibrate --study examples/calibration/c5/study.json --output .artifacts/c5/resume --resume .artifacts/c5/resume --workers 4
```

The checked-in fixture's validation ranking recovers candidate `c000004`
(`route_ticks=2`, `work_ticks=3`) with score `0`. Its held-out final-test summary
has mean completion error `0` across two completed, uncensored, non-failed attempts.
The fixture deliberately has too few fixed confirmation replications to meet the
declared precision target, so `mc_indeterminate` remains `true`; the exact winner
and held-out result do not turn that confirmation into a precision claim. Missing
required transit measurements contribute the configured failure penalty.

Compare two complete runs whose provenance bindings match (for example, runs made
with different worker counts using the same binary and study):

```sh
./target/debug/kairo-ecs-cli compare-runs --left .artifacts/c5/run-w1 --right .artifacts/c5/run-w4 --output .artifacts/c5/compare.json
```

The comparison checks provenance compatibility and artifact integrity before
reporting summaries. It does not establish cross-version or cross-machine
equivalence, empirical validity, clinical usefulness, or performance.

### Platform recovery boundary

Completed files are flushed before no-overwrite publication. Unix also syncs the
containing directory. Windows support is limited to process-termination recovery;
there is no Windows directory-entry flush or power-loss durability guarantee.
A completed shard is validated before reuse; recovery does not restore a live
simulation world. See the C5 acceptance evidence for platforms actually exercised.
