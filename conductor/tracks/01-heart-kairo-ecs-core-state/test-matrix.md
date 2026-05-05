# Test Matrix — 01 The Heart: kairo-ecs-core & kairo-ecs-state

## Required tests

- `cargo fmt --all --check`
- `cargo clippy -p kairo-ecs-types -p kairo-ecs-core -p kairo-ecs-state -p kairo-ecs-rng --all-targets -- -D warnings`
- `cargo test -p kairo-ecs-types`
- `cargo test -p kairo-ecs-core`
- `cargo test -p kairo-ecs-state`
- `cargo test -p kairo-ecs-rng`
- `cargo test --workspace`
- Fixture presence check for `conformance/fixtures/deterministic_ordering.json`, `conformance/fixtures/cancellation.json`, and `conformance/fixtures/rng_replay.json`
- `scripts/validate_conductor_setup.ps1` and `scripts/validate_track_coverage.ps1` both succeed

## CI commands

```bash
cargo fmt --all --check
cargo clippy -p kairo-ecs-types -p kairo-ecs-core -p kairo-ecs-state -p kairo-ecs-rng --all-targets -- -D warnings
cargo test -p kairo-ecs-types
cargo test -p kairo-ecs-core
cargo test -p kairo-ecs-state
cargo test -p kairo-ecs-rng
cargo test --workspace
pwsh -NoProfile -File scripts/validate_conductor_setup.ps1 -SkipCargo
pwsh -NoProfile -File scripts/validate_track_coverage.ps1 -SkipCargo
test -f conformance/fixtures/deterministic_ordering.json
test -f conformance/fixtures/cancellation.json
test -f conformance/fixtures/rng_replay.json
```
