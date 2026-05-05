# Test Matrix — 36 Streaming & Real-Time Processing

## Required tests

- `cargo build --workspace` to verify core workspace compiles with no streaming features enabled.
- `cargo test -p kairo-ecs-streaming --no-default-features` to confirm unit tests pass without any adapter.
- `cargo test -p kairo-ecs-streaming --features kafka` for Kafka adapter tests.
- `cargo test -p kairo-ecs-streaming --features nats` for NATS adapter tests.
- `cargo test -p kairo-ecs-streaming --features websocket` for WebSocket adapter tests.
- `cargo test -p kairo-ecs-streaming --features arrow-flight` for Arrow Flight adapter tests.
- `cargo test -p kairo-ecs-streaming --all-features` for full integration test suite.
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
