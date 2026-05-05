# Test Matrix — 04 The Analyst: kairo-ecs-arrow Telemetry

## Required tests

- `cargo test -p kairo-ecs-core` to keep the Arrow schema aligned with the event model it reports on.
- `cargo test -p kairo-ecs-state` to confirm the schema still reflects the current state transitions.
- `cargo fmt --all --check` before any schema or contract edit is handed off.
- `pwsh -NoProfile -File scripts\validate_conductor_setup.ps1 -SkipCargo` to confirm the conductor setup remains intact.
- `pwsh -NoProfile -File scripts\validate_track_coverage.ps1 -SkipCargo` to confirm Track 04 is still represented in the registry and wave policy.
- `cargo test --workspace` once a real `kairo-ecs-arrow` crate exists and starts exporting Arrow payloads.

| OTel export smoke test (OTLP collector receives spans) | no | yes | yes | yes |

## Current CI commands

```bash
cargo fmt --all --check
cargo test -p kairo-ecs-core
cargo test -p kairo-ecs-state
pwsh -NoProfile -File scripts\validate_conductor_setup.ps1 -SkipCargo
pwsh -NoProfile -File scripts\validate_track_coverage.ps1 -SkipCargo
```
