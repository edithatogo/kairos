# Test Matrix — 03 The Flow: DES Trajectory API & ABM Behavior API

## Required tests

- `cargo test -p kairo-ecs-core` to cover the scheduler and shared event model that DES and ABM both depend on.
- `cargo test -p kairo-ecs-state` to keep the state transition layer deterministic while the trajectory and behavior APIs are being defined.
- `cargo fmt --all --check` before any handoff that touches Rust code.
- `pwsh -NoProfile -File scripts\validate_conductor_setup.ps1 -SkipCargo` to keep the conductor setup consistent.
- `pwsh -NoProfile -File scripts\validate_track_coverage.ps1 -SkipCargo` to prove the track is still accounted for in the wave policy and registry.
- `cargo test --workspace` once the track starts adding concrete DES and ABM code paths.
- `cargo test -p kairo-ecs-des -p kairo-ecs-abm` for the Track 03 deterministic trajectory and behavior-update smoke fixtures.

## Current CI commands

```bash
cargo fmt --all --check
cargo test -p kairo-ecs-core
cargo test -p kairo-ecs-state
cargo test -p kairo-ecs-des -p kairo-ecs-abm
pwsh -NoProfile -File scripts\validate_conductor_setup.ps1 -SkipCargo
pwsh -NoProfile -File scripts\validate_track_coverage.ps1 -SkipCargo
```
