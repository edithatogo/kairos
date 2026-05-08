# Test Matrix — 05 The Window: kairo-ecs-viz Visualization

Last verified: 2026-05-08

## Required tests

- `cargo check --manifest-path crates/kairo-ecs-viz/Cargo.toml --no-default-features` to prove the visualization crate compiles without renderer features or GUI dependencies.
- `cargo check --manifest-path crates/kairo-ecs-viz/Cargo.toml --no-default-features --tests` to prove the headless frame contract tests compile without linking GUI dependencies.
- `pwsh -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\05-window-kairo-ecs-viz\validate-state-snapshot.ps1` to prove the Track 01 `WorldSnapshot` to Track 05 `RenderFrame` path exists and compiles through linker-safe gates.
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

## Current Track 05 validation — 2026-05-08

- `cargo +stable-x86_64-pc-windows-gnu fmt -p kairo-ecs-viz --check` — pass.
- `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-viz` — pass, 10 tests.
- `cargo +stable-x86_64-pc-windows-gnu check --manifest-path crates\kairo-ecs-viz\Cargo.toml --no-default-features` — pass.
- `cargo +stable-x86_64-pc-windows-gnu check --manifest-path crates\kairo-ecs-viz\Cargo.toml --no-default-features --tests` — pass.
- `cargo +stable-x86_64-pc-windows-gnu check --manifest-path crates\kairo-ecs-viz\Cargo.toml --all-features --tests` — pass.
- `pwsh -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\05-window-kairo-ecs-viz\validate-state-snapshot.ps1` — pass.
- `cargo +stable-x86_64-pc-windows-gnu check --manifest-path examples\viz\headless-snapshot\Cargo.toml` — pass.
- `cargo +stable-x86_64-pc-windows-gnu check -p kairo-ecs-core --no-default-features` — pass.
- `cargo +stable-x86_64-pc-windows-gnu tree -p kairo-ecs-core --no-default-features` — pass; tree contains no `kairo-ecs-viz` dependency in the core dependency tree.
- `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-core` — pass, 26 tests.
- `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-state` — pass, 6 tests.
- `npm --prefix website run build` — pass, 105 pages rendered.
- `cargo +stable-x86_64-pc-windows-gnu fmt --all --check` — pass.
- `pwsh -NoProfile -File scripts\validate_conductor_setup.ps1 -SkipCargo` — pass.
- `pwsh -NoProfile -File scripts\validate_track_coverage.ps1 -SkipCargo` — pass.
- Initial `cargo test -p kairo-ecs-viz` on the default MSVC target did not reach code execution because `link.exe` resolved to Git's Unix-link shim and failed with Win32 error 5; the same Track 05 test set passed on the repo's usable GNU toolchain.

The Track 05 review slice includes the formal `kairo_ecs.visualization.frame.v1`
fixture JSON output and deterministic no-GUI SVG rendering. Native WGPU/Bevy
runtime backends remain explicit post-review feature work.

## Current CI commands

```bash
cargo check --manifest-path crates/kairo-ecs-viz/Cargo.toml --no-default-features
cargo check --manifest-path crates/kairo-ecs-viz/Cargo.toml --no-default-features --tests
pwsh -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\05-window-kairo-ecs-viz\validate-state-snapshot.ps1
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
## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, phase-closeout ledger evidence, cleaned commit/push evidence, and blocker recording. At actual closeout, run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.
