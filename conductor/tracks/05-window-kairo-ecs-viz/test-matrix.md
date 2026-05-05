# Test Matrix — 05 The Window: kairo-ecs-viz Visualization

## Required tests

- `cargo check --manifest-path crates/kairo-ecs-viz/Cargo.toml --no-default-features` to prove the visualization crate compiles without renderer features or GUI dependencies.
- `cargo check --manifest-path crates/kairo-ecs-viz/Cargo.toml --no-default-features --tests` to prove the headless frame contract tests compile without linking GUI dependencies.
- `cargo check --manifest-path crates/kairo-ecs-viz/Cargo.toml --all-features --tests` to prove optional renderer feature names compile and report explicit not-configured status.
- `cargo check --manifest-path examples/viz/headless-snapshot/Cargo.toml` for a no-window example compile smoke test.
- `cargo check -p kairo-ecs-core --no-default-features` to prove the headless core remains independent of visualization.
- `cargo tree -p kairo-ecs-core --no-default-features` to verify `kairo-ecs-viz` is absent from the core dependency tree.
- `cargo test -p kairo-ecs-core` to keep the visualization inputs aligned with the shared event model.
- `cargo test -p kairo-ecs-state` to make sure the visualized state snapshots stay deterministic.
- `npm --prefix website run build` to validate the website build path that carries the visualization docs.
- `cargo fmt --all --check` before any Rust-facing handoff.
- `pwsh -NoProfile -File scripts\validate_conductor_setup.ps1 -SkipCargo` to keep the conductor setup synchronized.
- `pwsh -NoProfile -File scripts\validate_track_coverage.ps1 -SkipCargo` to confirm Track 05 is still covered by the wave policy and registry.

## Current CI commands

```bash
cargo check --manifest-path crates/kairo-ecs-viz/Cargo.toml --no-default-features
cargo check --manifest-path crates/kairo-ecs-viz/Cargo.toml --no-default-features --tests
cargo check --manifest-path crates/kairo-ecs-viz/Cargo.toml --all-features --tests
cargo check --manifest-path examples/viz/headless-snapshot/Cargo.toml
cargo check -p kairo-ecs-core --no-default-features
cargo tree -p kairo-ecs-core --no-default-features
cargo fmt --all --check
npm --prefix website run build
cargo test -p kairo-ecs-core
cargo test -p kairo-ecs-state
npm --prefix website run build
pwsh -NoProfile -File scripts\validate_conductor_setup.ps1 -SkipCargo
pwsh -NoProfile -File scripts\validate_track_coverage.ps1 -SkipCargo
```
