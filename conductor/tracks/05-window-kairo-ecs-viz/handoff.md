# Handoff — 05 The Window: kairo-ecs-viz Visualization

Last verified: 2026-05-08

## Summary

Track 05 is closed as Done. `kairo-ecs-viz` is a dependency-light crate that defines the frame contract, validates headless frames, emits formal fixture JSON, and returns deterministic frame summaries without opening a window or linking GUI dependencies. Optional renderer feature names are explicit (`wgpu-renderer`, `bevy-renderer`) and currently report not-configured status until real renderer dependencies are deliberately introduced.

This pass completed the Track 05 headless release contract: `kairo-ecs-viz` converts deterministic `WorldSnapshot` data into a headless `RenderFrame`, can attach scheduler `DispatchedEvent` markers, emits deterministic text and SVG output, and exports the formal `kairo_ecs.visualization.frame.v1` JSON fixture envelope for docs and CI environments without graphics hardware.

## Files changed

- `conductor/phase-closeout.yaml`
- `conductor/tracks.md`
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
cargo +stable-x86_64-pc-windows-gnu check --manifest-path crates/kairo-ecs-viz/Cargo.toml --no-default-features
cargo +stable-x86_64-pc-windows-gnu check --manifest-path crates/kairo-ecs-viz/Cargo.toml --no-default-features --tests
cargo +stable-x86_64-pc-windows-gnu check --manifest-path crates/kairo-ecs-viz/Cargo.toml --all-features --tests
cargo +stable-x86_64-pc-windows-gnu check --manifest-path examples/viz/headless-snapshot/Cargo.toml
cargo +stable-x86_64-pc-windows-gnu check -p kairo-ecs-core --no-default-features
cargo +stable-x86_64-pc-windows-gnu tree -p kairo-ecs-core --no-default-features
cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-viz
cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-core
cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-state
npm --prefix website run build
pwsh -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\05-window-kairo-ecs-viz\validate-state-snapshot.ps1
```

## Known risks

The visualization contract still needs to stay aligned with the core state and scheduler surfaces so docs and examples do not drift from implementation. Real WGPU/Bevy rendering remains intentionally absent in this slice and should only be added behind explicit opt-in features with CI gates that do not require a display server.

Default MSVC-target `cargo test -p kairo-ecs-viz` is not a reliable local gate on this host because `link.exe` resolves to Git's Unix-link shim and fails before code execution. The same test set passes on `stable-x86_64-pc-windows-gnu`, which is the local Rust gate used for closeout evidence.

## Integration notes

The core workspace remains headless-safe: no core crate depends on `kairo-ecs-viz`, and the viz crate's default features are empty. The fixture-ready ECS snapshot conversion path is implemented through `WorldSnapshot`; richer payloads can be added later when Track 01 exposes component payload snapshots.

## Follow-up issues

- Add real renderer backends only after renderer dependencies and display-free CI gates are explicit.
- Align the current `kairo_ecs.visualization.frame.v1` fixture with Track 12 if a wider conformance fixture registry later changes field naming.
- Extend frame entities beyond stable labels and milli-unit positions when Track 01 exposes component payload snapshots.
## Phase closeout evidence

Track 05 closeout completed on 2026-05-08 after `$conductor-review`-style source review of the track spec, plan, test matrix, handoff, risk register, owned implementation, example, visualization docs, and validators. No accepted code fixes were required in Track 05 owned implementation paths during this closeout pass; documentation/status closeout updates were applied.

Validation commands:

```bash
cargo test -p kairo-ecs-viz
cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-viz
cargo +stable-x86_64-pc-windows-gnu fmt -p kairo-ecs-viz --check
pwsh -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\05-window-kairo-ecs-viz\validate-state-snapshot.ps1 -SkipCargo
pwsh -NoProfile -File scripts\validate_conductor_setup.ps1 -SkipCargo
pwsh -NoProfile -File scripts\validate_track_coverage.ps1 -SkipCargo
cargo +stable-x86_64-pc-windows-gnu check --manifest-path crates\kairo-ecs-viz\Cargo.toml --no-default-features
cargo +stable-x86_64-pc-windows-gnu check --manifest-path crates\kairo-ecs-viz\Cargo.toml --no-default-features --tests
cargo +stable-x86_64-pc-windows-gnu check --manifest-path crates\kairo-ecs-viz\Cargo.toml --all-features --tests
cargo +stable-x86_64-pc-windows-gnu check --manifest-path examples\viz\headless-snapshot\Cargo.toml
cargo +stable-x86_64-pc-windows-gnu check -p kairo-ecs-core --no-default-features
cargo +stable-x86_64-pc-windows-gnu tree -p kairo-ecs-core --no-default-features
pwsh -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\05-window-kairo-ecs-viz\validate-state-snapshot.ps1
cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-core
cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-state
npm --prefix website run build
cargo +stable-x86_64-pc-windows-gnu fmt --all --check
```

Result: all Track 05 evidence gates passed on the repo's usable GNU toolchain; the initial default-target `cargo test -p kairo-ecs-viz` failed only at host linker startup because Git's `link.exe` shim was selected. Git cleanup state before edits was clean. Closeout status is recorded in `conductor/phase-closeout.yaml` with commit SHA `27ce9204174275b627198f58a3d14cc1d7e84a4b` and pushed ref `origin/main`; final strict clean-tree validation must be rerun after this closeout edit is committed. Next-phase decision: Track 05 is Done; future WGPU/Bevy renderer work belongs in a new track or Track 24 rather than reopening this headless release slice.
