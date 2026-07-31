# Test Matrix — 40 Time-Travel Debugging & Interactive Stepping

## Focused Track 40 tests

- `cargo +stable-x86_64-pc-windows-gnu test --manifest-path crates\kairo-ecs-debug\Cargo.toml --target x86_64-pc-windows-gnu` for debug crate unit tests on this Windows host.
- `cargo test -p kairo-ecs-debug` remains the workspace-package equivalent used by central coverage checks.
- `cargo check --manifest-path crates\kairo-ecs-debug\Cargo.toml --tests --target x86_64-pc-windows-gnu` for a compile-only fallback when the local Windows linker is unavailable.
- `cargo fmt --manifest-path crates\kairo-ecs-debug\Cargo.toml --check` before crate handoff.
- `cargo clippy --manifest-path crates\kairo-ecs-debug\Cargo.toml --all-targets -- -D warnings`.
- `node website\time-travel-demo\validate-demo.mjs` for the static timeline scrubber smoke.

Central conductor scripts and core/state workspace tests are still required before merge, but they are outside this Track 40-owned hardening slice because this pass is constrained to `crates/kairo-ecs-debug/**`, `docs/debugging/**`, `website/time-travel-demo/**`, and this track folder.

- `pwsh -NoProfile -File scripts\validate_track_coverage.ps1 -SkipCargo` remains the central coverage gate for this track.

## Track-specific gates

- **trace-record-replay**: Validate the line-oriented scaffold encoding accepts encoded traces and rejects missing schema, unsupported schema, malformed delta fields, and out-of-order ticks.
- **trace-roundtrip**: Legacy coverage-check alias for `trace-record-replay`; keep central gate naming on `trace-record-replay`.
- **replay-smoke**: Reconstruct state from the nearest snapshot plus deltas and assert expected state at tick boundaries.
- **fwd-back-parity**: Step from the initial snapshot to the first delta, step forward, step back, and assert reconstructed state matches the expected forward state for each cursor tick over the offline trace model.
- **forward-backward-parity**: Legacy coverage-check alias for `fwd-back-parity`; keep central gate naming on `fwd-back-parity`.
- **breakpoint-smoke**: Set a breakpoint on a specific event kind, run to the matching delta, and assert the cursor and reconstructed state are correct.
- **goto-tick-accuracy-smoke**: Seek to tick N and assert the debugger lands on the first delta at or after N.
- **timeline-render-smoke**: Load the fixture into the static timeline scrubber validator and verify initial render, step/back controls, event-dot selection, active marker, and state inspector refresh.

## Deferred integration gates

- **trace-record-replay-arrow**: Record a benchmark run, serialize trace to Arrow IPC, deserialize, replay, and assert final state hash matches original. Deferred until Track 04 Arrow IPC schema is available in this worker's scope.
- **scheduler-hook-parity**: Record a deterministic core scheduler run and compare trace snapshots against Track 12 conformance fixtures. Deferred until Track 01/12 integration can be changed and tested outside the Track 40-only write boundary.
- **large-trace-timeline**: Validate browser timeline behavior with aggregated or virtualized traces at 100K+ visible events. Deferred until the demo has virtualization rather than static fixture rendering.

## Current focused commands

```bash
cargo fmt --manifest-path crates\kairo-ecs-debug\Cargo.toml --check
cargo check --manifest-path crates\kairo-ecs-debug\Cargo.toml --tests --target x86_64-pc-windows-gnu
cargo clippy --manifest-path crates\kairo-ecs-debug\Cargo.toml --all-targets --target x86_64-pc-windows-gnu -- -D warnings
set CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=rust-lld
cargo +stable-x86_64-pc-windows-gnu test --manifest-path crates\kairo-ecs-debug\Cargo.toml --target x86_64-pc-windows-gnu
node website\time-travel-demo\validate-demo.mjs
powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\36-streaming-real-time-processing\validate-track36-40.ps1 -SkipCargoTests
```
## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, phase-closeout ledger evidence, cleaned commit/push evidence, and blocker recording. At actual closeout, run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.
