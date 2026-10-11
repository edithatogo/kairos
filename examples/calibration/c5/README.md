# C5 synthetic calibration example

This directory contains the bounded, normalized synthetic study fixture used by
KairoECS C5. The runner evaluates integer tick parameters against the supplied
synthetic records; it does not ingest site data, fit a clinical model, or establish
operational validity. The exact study schema, split rules, and limits are in the
[accepted C5 implementation contract](../../../conductor/design/calibration/ADR-0022-c5-study-runner.md).

## Build and inspect

Run these commands from the Kairos repository root with the pinned Rust 1.99 toolchain:

```sh
cargo +1.99.0 build -p kairo-ecs-cli
./target/debug/kairo-ecs-cli verify trace --study examples/calibration/c5/study.json
./target/debug/kairo-ecs-cli replay --study examples/calibration/c5/study.json --mode shadow --output .artifacts/c5/shadow
./target/debug/kairo-ecs-cli replay --study examples/calibration/c5/study.json --mode free --output .artifacts/c5/free
```

The shadow mode uses ledger anchors and independent probes. Free mode runs probes
through a shared FCFS native resource and does not use observed completions to drive
native time or state. Both modes report the `synthetic_flow_probe.v1` adapter.
Replay output directories must be new.

## Calibrate, stop, and resume

```sh
./target/debug/kairo-ecs-cli calibrate --study examples/calibration/c5/study.json --output .artifacts/c5/run-w1 --workers 1
./target/debug/kairo-ecs-cli calibrate --study examples/calibration/c5/study.json --output .artifacts/c5/run-w4 --workers 4
./target/debug/kairo-ecs-cli calibrate --study examples/calibration/c5/study.json --output .artifacts/c5/resume --workers 1 --stop-after 3
./target/debug/kairo-ecs-cli calibrate --study examples/calibration/c5/study.json --output .artifacts/c5/resume --resume .artifacts/c5/resume --workers 4
./target/debug/kairo-ecs-cli compare-runs --left .artifacts/c5/run-w1 --right .artifacts/c5/run-w4 --output .artifacts/c5/compare.json
```

Each new run path must not exist. On resume, `--resume` must name the same existing
path as `--output`. Recovery is at validated completed task-shard boundaries in separate
`initial-shards/` and `final-shards/` directories. It does not restore a running
world or partial task. Resume verifies provenance: use the same executable bytes,
study bytes, source/build and lock metadata, and optional scenario/seed manifest
bytes as the original invocation. `compare-runs` requires compatible provenance
and complete, integrity-checked bundles.

The study splits records into non-overlapping, half-open training, validation, and
final-test intervals. A record crossing a partition cutoff is clipped there and
reported as censored; it is not treated as a later observed completion. Validation
selects and freezes the winner. The final-test tasks then evaluate only that winner
on the held-out partition, using the native shared FCFS resource; observed held-out
completion times do not control the simulation trajectory. Missing required transit
measurements count as uncovered and receive the configured failure penalty.

For this fixture, the validation winner is `c000004` with
`route_ticks=2` and `work_ticks=3`, with initial ranking score `0`. The held-out
summary has mean completion error `0` from two completed attempts, with no censored
or failed attempts. The independent fixed-confirmation sample is intentionally too
small to establish the requested precision, so the summary reports
`mc_indeterminate: true`. Do not interpret the exact fixture recovery as an
empirical, clinical, cross-site, or performance result.

For the complete CLI reference and the retained legacy scenario commands, see
[`docs/cli/kairo-ecs-cli.md`](../../../docs/cli/kairo-ecs-cli.md).

### Platform recovery boundary

Completed files are flushed before no-overwrite publication. Unix also syncs the
containing directory. Windows support is limited to process-termination recovery;
there is no Windows directory-entry flush or power-loss durability guarantee.
A completed shard is validated before reuse; recovery does not restore a live
simulation world. See the C5 acceptance evidence for platforms actually exercised.
