# Visualization

Maturity: experimental.

Track 05 provides the optional `kairo-ecs-viz` crate. The current R2 slice is
headless-safe: it defines the visualization frame contract and smoke-testable
summary rendering without linking GUI, windowing, WGPU, or Bevy dependencies.

## Headless contract

- `RenderFrame` carries a simulation time, frame entities, and event markers.
- `FrameEntity` stores stable entity identity, label, and integer milli-unit
  positions so docs and tests can compare deterministic values.
- `render_headless` validates the frame and returns a `FrameSummary` with entity
  count, event count, and bounds.
- Default features are empty. `wgpu-renderer` and `bevy-renderer` are explicit
  opt-in feature names and currently report not-configured status instead of
  opening a window or requiring graphics hardware.

## Smoke gates

```bash
cargo check --manifest-path crates/kairo-ecs-viz/Cargo.toml --no-default-features
cargo check --manifest-path crates/kairo-ecs-viz/Cargo.toml --no-default-features --tests
cargo check --manifest-path crates/kairo-ecs-viz/Cargo.toml --all-features --tests
cargo check --manifest-path examples/viz/headless-snapshot/Cargo.toml
cargo check -p kairo-ecs-core --no-default-features
cargo tree -p kairo-ecs-core --no-default-features
```

These gates are designed for CI hosts without a display server or GPU.
