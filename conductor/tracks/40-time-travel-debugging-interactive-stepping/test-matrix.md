# Test Matrix — 40 Time-Travel Debugging & Interactive Stepping

## Focused Track 40 tests

- `cargo test --manifest-path crates\kairo-ecs-debug\Cargo.toml` for debug crate unit tests.
- `cargo test -p kairo-ecs-debug` remains the workspace-package equivalent used by central coverage checks.
- `cargo check --manifest-path crates\kairo-ecs-debug\Cargo.toml --tests` for a compile-only fallback when the local Windows linker is unavailable.
- `cargo fmt --manifest-path crates\kairo-ecs-debug\Cargo.toml --check` before crate handoff.
- `cargo clippy --manifest-path crates\kairo-ecs-debug\Cargo.toml --all-targets -- -D warnings`.
- `node website\time-travel-demo\validate-demo.mjs` for the static timeline scrubber smoke.

Central conductor scripts and core/state workspace tests are still required before merge, but they are outside this Track 40-owned hardening slice because this pass is constrained to `crates/kairo-ecs-debug/**`, `docs/debugging/**`, `website/time-travel-demo/**`, and this track folder.

- `pwsh -NoProfile -File scripts\validate_track_coverage.ps1 -SkipCargo` remains the central coverage gate for this track.

## Track-specific gates

- **trace-validation-smoke**: Validate the line-oriented scaffold encoding accepts encoded traces and rejects missing schema, unsupported schema, malformed delta fields, and out-of-order ticks.
- **trace-roundtrip**: Current R2 alias for `trace-validation-smoke` plus `replay-smoke`; Arrow IPC roundtrip is deferred below.
- **replay-smoke**: Reconstruct state from the nearest snapshot plus deltas and assert expected state at tick boundaries.
- **forward-backward-parity-smoke**: Step from the initial snapshot to the first delta, step forward, step back, and assert reconstructed state matches the expected forward state for each cursor tick.
- **forward-backward-parity**: Current R2 alias for `forward-backward-parity-smoke` over the offline trace model.
- **breakpoint-smoke**: Set a breakpoint on a specific event kind, run to the matching delta, and assert the cursor and reconstructed state are correct.
- **goto-tick-accuracy-smoke**: Seek to tick N and assert the debugger lands on the first delta at or after N.
- **timeline-render-smoke**: Load the fixture into the static timeline scrubber validator and verify initial render, step/back controls, event-dot selection, active marker, and state inspector refresh.

## Deferred integration gates

- **trace-roundtrip-arrow**: Record a benchmark run, serialize trace to Arrow IPC, deserialize, replay, and assert final state hash matches original. Deferred until Track 04 Arrow IPC schema is available in this worker's scope.
- **scheduler-hook-parity**: Record a deterministic core scheduler run and compare trace snapshots against Track 12 conformance fixtures. Deferred until Track 01/12 integration can be changed and tested outside the Track 40-only write boundary.
- **large-trace-timeline**: Validate browser timeline behavior with aggregated or virtualized traces at 100K+ visible events. Deferred until the demo has virtualization rather than static fixture rendering.

## Current focused commands

```bash
cargo fmt --manifest-path crates\kairo-ecs-debug\Cargo.toml --check
cargo check --manifest-path crates\kairo-ecs-debug\Cargo.toml --tests
cargo clippy --manifest-path crates\kairo-ecs-debug\Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path crates\kairo-ecs-debug\Cargo.toml
node website\time-travel-demo\validate-demo.mjs
powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\36-streaming-real-time-processing\validate-track36-40.ps1 -SkipCargoTests
```
