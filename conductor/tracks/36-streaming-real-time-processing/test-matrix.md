# Test Matrix — 36 Streaming & Real-Time Processing

## Required tests

- `cargo build --workspace` to verify core workspace compiles with no streaming features enabled.
- `cargo test -p kairo-ecs-streaming --no-default-features` to confirm unit tests pass without any adapter.
- `cargo test -p kairo-ecs-streaming --features kafka` for the Kafka feature-gate scaffold test. Current adapter is an in-memory test double.
- `cargo test -p kairo-ecs-streaming --features nats` for the NATS feature-gate scaffold test. Current adapter is an in-memory test double.
- `cargo test -p kairo-ecs-streaming --features websocket` for the WebSocket feature-gate scaffold test. Current adapter is an in-memory test double.
- `cargo test -p kairo-ecs-streaming --features arrow-flight` for the Arrow Flight feature-gate scaffold test. Current endpoint is an in-memory test double.
- `cargo test -p kairo-ecs-streaming --all-features` for the full feature-gate scaffold suite. This is not a broker integration test until real clients land.
- `cargo fmt --all --check` before any crate or contract edit is handed off.
- `cargo clippy -p kairo-ecs-streaming --all-targets -- -D warnings`.
- `pwsh -NoProfile -File scripts\validate_conductor_setup.ps1 -SkipCargo` to confirm conductor setup remains intact.
- `pwsh -NoProfile -File scripts\validate_track_coverage.ps1 -SkipCargo` to confirm Track 36 is registered.

## Current CI commands

```bash
cargo build --workspace
cargo fmt --all --check
cargo clippy -p kairo-ecs-streaming --all-targets -- -D warnings
cargo test -p kairo-ecs-streaming --no-default-features
cargo test -p kairo-ecs-streaming --all-features
pwsh -NoProfile -File scripts\validate_conductor_setup.ps1 -SkipCargo
pwsh -NoProfile -File scripts\validate_track_coverage.ps1 -SkipCargo
```

## Local broker-free checks

These checks are expected to remain runnable without external services:

```bash
cargo check -p kairo-ecs-streaming --no-default-features
cargo check -p kairo-ecs-streaming --all-features --tests
rustfmt --check crates/kairo-ecs-streaming/src/lib.rs crates/kairo-ecs-streaming/tests/feature_matrix.rs
powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\36-streaming-real-time-processing\validate-track36-40.ps1 -SkipCargoTests
```
