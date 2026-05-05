# Handoff — 05 The Window: kairo-ecs-viz Visualization

## Summary

Track 05 now has a minimal R2 visualization slice. `kairo-ecs-viz` is a dependency-light crate that defines the frame contract, validates headless frames, and returns deterministic frame summaries without opening a window or linking GUI dependencies. Optional renderer feature names are explicit (`wgpu-renderer`, `bevy-renderer`) and currently report not-configured status until real renderer dependencies are deliberately introduced.

## Files changed

- `Cargo.toml`
- `crates/kairo-ecs-viz/Cargo.toml`
- `crates/kairo-ecs-viz/src/lib.rs`
- `crates/kairo-ecs-viz/tests/feature_matrix.rs`
- `examples/viz/headless-snapshot/Cargo.toml`
- `examples/viz/headless-snapshot/src/main.rs`
- `website/docs/visualization/README.md`
- `conductor/tracks/05-window-kairo-ecs-viz/test-matrix.md`
- `conductor/tracks/05-window-kairo-ecs-viz/handoff.md`

## Contracts consumed

`conductor/workflow.md`, `conductor/contracts/core-contract.md`, and `conductor/contracts/conformance-contract.md`. The code consumes only `kairo-ecs-types` identity, time, and event-kind types.

## Contracts changed

No shared core contracts were changed.

## Tests added

- Unit tests in `crates/kairo-ecs-viz/src/lib.rs` for headless summary rendering and validation.
- Integration tests in `crates/kairo-ecs-viz/tests/feature_matrix.rs` for explicit renderer feature state and empty-frame smoke coverage.
- Headless example package in `examples/viz/headless-snapshot`.

Smoke gates:

```bash
cargo check --manifest-path crates/kairo-ecs-viz/Cargo.toml --no-default-features
cargo check --manifest-path crates/kairo-ecs-viz/Cargo.toml --no-default-features --tests
cargo check --manifest-path crates/kairo-ecs-viz/Cargo.toml --all-features --tests
cargo check --manifest-path examples/viz/headless-snapshot/Cargo.toml
cargo check -p kairo-ecs-core --no-default-features
cargo tree -p kairo-ecs-core --no-default-features
```

## Known risks

The visualization contract still needs to stay aligned with the core state and scheduler surfaces so docs and examples do not drift from implementation. Real WGPU/Bevy rendering remains intentionally absent in this slice and should only be added behind explicit opt-in features with CI gates that do not require a display server.

On this Windows host, commands that link Rust test or example executables still fail because `link.exe` resolves to `C:\Users\60217257\scoop\apps\git\current\usr\bin\link.exe` and exits with `couldn't create signal pipe, Win32 error 5`. Track 05 therefore uses `cargo check ... --tests` and example `cargo check` as the required headless smoke gates until the linker path is corrected.

## Integration notes

The core workspace remains headless-safe: no core crate depends on `kairo-ecs-viz`, and the viz crate's default features are empty. Next step is to add fixture-backed conversion from accepted ECS snapshots into `RenderFrame` without changing core contracts.
