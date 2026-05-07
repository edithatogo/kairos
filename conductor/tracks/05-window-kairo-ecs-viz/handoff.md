# Handoff — 05 The Window: kairo-ecs-viz Visualization

Last verified: 2026-05-07

## Summary

Track 05 now has a review-ready visualization slice. `kairo-ecs-viz` is a dependency-light crate that defines the frame contract, validates headless frames, emits formal fixture JSON, and returns deterministic frame summaries without opening a window or linking GUI dependencies. Optional renderer feature names are explicit (`wgpu-renderer`, `bevy-renderer`) and currently report not-configured status until real renderer dependencies are deliberately introduced.

This pass completed the Track 05 headless release contract: `kairo-ecs-viz` converts deterministic `WorldSnapshot` data into a headless `RenderFrame`, can attach scheduler `DispatchedEvent` markers, emits deterministic text and SVG output, and exports the formal `kairo_ecs.visualization.frame.v1` JSON fixture envelope for docs and CI environments without graphics hardware.

## Files changed

- `crates/kairo-ecs-viz/Cargo.toml`
- `crates/kairo-ecs-viz/src/lib.rs`
- `crates/kairo-ecs-viz/tests/feature_matrix.rs`
- `examples/viz/headless-snapshot/Cargo.toml`
- `examples/viz/headless-snapshot/src/main.rs`
- `conductor/tracks.yaml`
- `conductor/tracks/05-window-kairo-ecs-viz/spec.md`
- `conductor/tracks/05-window-kairo-ecs-viz/risk-register.md`
- `conductor/tracks/05-window-kairo-ecs-viz/validate-state-snapshot.ps1`
- `conductor/tracks/05-window-kairo-ecs-viz/test-matrix.md`
- `conductor/tracks/05-window-kairo-ecs-viz/handoff.md`
- `website/docs/visualization/README.md`

## Contracts consumed

`conductor/workflow.md`, `conductor/contracts/core-contract.md`, and `conductor/contracts/conformance-contract.md`. The code consumes `kairo-ecs-state::WorldSnapshot` plus `kairo-ecs-types` identity, time, dispatched-event, and event-kind types.

## Contracts changed

No shared core contracts were changed.

## Tests added

- Unit tests in `crates/kairo-ecs-viz/src/lib.rs` for headless summary rendering, deterministic text rendering, deterministic SVG rendering, formal fixture JSON, validation, `WorldSnapshot` conversion, and dispatched-event marker conversion.
- Integration tests in `crates/kairo-ecs-viz/tests/feature_matrix.rs` for explicit renderer feature state and empty-frame smoke coverage.
- Headless example package in `examples/viz/headless-snapshot`.
- Track-local validator `conductor/tracks/05-window-kairo-ecs-viz/validate-state-snapshot.ps1`, updated to accept the current deterministic `sort_unstable_by_key` Track 01 state implementation.

Smoke gates:

```bash
cargo check --manifest-path crates/kairo-ecs-viz/Cargo.toml --no-default-features
cargo check --manifest-path crates/kairo-ecs-viz/Cargo.toml --no-default-features --tests
cargo check --manifest-path crates/kairo-ecs-viz/Cargo.toml --all-features --tests
cargo check --manifest-path examples/viz/headless-snapshot/Cargo.toml
cargo check -p kairo-ecs-core --no-default-features
cargo tree -p kairo-ecs-core --no-default-features
cargo test -p kairo-ecs-viz
cargo test -p kairo-ecs-core
cargo test -p kairo-ecs-state
npm --prefix website run build
pwsh -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\05-window-kairo-ecs-viz\validate-state-snapshot.ps1
```

## Known risks

The visualization contract still needs to stay aligned with the core state and scheduler surfaces so docs and examples do not drift from implementation. Real WGPU/Bevy rendering remains intentionally absent in this slice and should only be added behind explicit opt-in features with CI gates that do not require a display server.

Workspace-wide `cargo fmt --all --check` remains blocked by unrelated formatting drift outside Track 05-owned paths. The focused Track 05 formatting gate passes.

## Integration notes

The core workspace remains headless-safe: no core crate depends on `kairo-ecs-viz`, and the viz crate's default features are empty. The fixture-ready ECS snapshot conversion path is implemented through `WorldSnapshot`; richer payloads can be added later when Track 01 exposes component payload snapshots.

## Follow-up issues

- Add real renderer backends only after renderer dependencies and display-free CI gates are explicit.
- Align the current `kairo_ecs.visualization.frame.v1` fixture with Track 12 if a wider conformance fixture registry later changes field naming.
- Extend frame entities beyond stable labels and milli-unit positions when Track 01 exposes component payload snapshots.
