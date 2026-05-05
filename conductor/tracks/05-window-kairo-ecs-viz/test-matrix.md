# Test Matrix — 05 The Window: kairo-ecs-viz Visualization

## Required tests

- `cargo test -p kairo-ecs-core` to keep the visualization inputs aligned with the shared event model.
- `cargo test -p kairo-ecs-state` to make sure the visualized state snapshots stay deterministic.
- `npm --prefix website run build` to validate the website build path that carries the visualization docs.
- `cargo fmt --all --check` before any Rust-facing handoff.
- `pwsh -NoProfile -File scripts\validate_conductor_setup.ps1 -SkipCargo` to keep the conductor setup synchronized.
- `pwsh -NoProfile -File scripts\validate_track_coverage.ps1 -SkipCargo` to confirm Track 05 is still covered by the wave policy and registry.

## Current CI commands

```bash
cargo fmt --all --check
npm --prefix website run build
cargo test -p kairo-ecs-core
cargo test -p kairo-ecs-state
npm --prefix website run build
pwsh -NoProfile -File scripts\validate_conductor_setup.ps1 -SkipCargo
pwsh -NoProfile -File scripts\validate_track_coverage.ps1 -SkipCargo
```
