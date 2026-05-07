# Visualization

Maturity: experimental.

Track 05 provides the optional `kairo-ecs-viz` crate. The current review slice
is headless-safe: it defines the visualization frame contract, formal fixture
output, deterministic SVG previews, and smoke-testable summary rendering
without linking GUI, windowing, WGPU, or Bevy dependencies.

## Headless contract

- `RenderFrame` carries a simulation time, frame entities, and event markers.
- `FrameEntity` stores stable entity identity, label, and integer milli-unit
  positions so docs and tests can compare deterministic values.
- `render_headless` validates the frame and returns a `FrameSummary` with entity
  count, event count, and bounds.
- `render_headless_text` emits a deterministic line-oriented frame dump for
  docs, smoke examples, and CI hosts without graphics hardware.
- `render_fixture_json` emits the formal `kairo_ecs.visualization.frame.v1`
  fixture envelope for conformance and documentation examples.
- `render_headless_svg` emits a deterministic no-GUI SVG preview suitable for
  static docs and CI artifacts.
- `RenderFrame::from_world_snapshot_and_events` joins the deterministic
  `WorldSnapshot` entity list with scheduler `DispatchedEvent` markers without
  taking a dependency on the headless core runtime.
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
