# Test Matrix — 40 Time-Travel Debugging & Interactive Stepping

## Required tests

- `cargo test -p kairo-ecs-debug` for all debug crate unit and integration tests.
- `cargo test -p kairo-ecs-core` to confirm trace recording hooks do not break the scheduler.
- `cargo test -p kairo-ecs-state` to confirm snapshot serialization remains compatible.
- `cargo fmt --all --check` before any crate or contract edit is handed off.
- `cargo clippy -p kairo-ecs-debug --all-targets -- -D warnings`.
- `pwsh -NoProfile -File scripts\validate_conductor_setup.ps1 -SkipCargo` to confirm conductor setup remains intact.
- `pwsh -NoProfile -File scripts\validate_track_coverage.ps1 -SkipCargo` to confirm Track 40 is registered.

## Track-specific gates

- **trace-roundtrip**: Record a 100K-event benchmark run, serialize trace to Arrow IPC, deserialize, replay, and assert final state hash matches original.
- **forward-backward-parity**: For a deterministic run with known state hashes at each tick, step backward to each tick and assert state hash matches the forward hash at that tick.
- **breakpoint-smoke**: Set breakpoint on a specific event kind, run simulation, and assert execution pauses at the correct event dispatch.
- **goto-tick-accuracy**: Record trace, go to tick N, and assert state hash matches the forward run at tick N.
- **timeline-render-smoke**: Load trace into browser timeline scrubber and verify it renders without JavaScript errors for traces up to 100K events.

## Current CI commands

```bash
cargo build --workspace
cargo fmt --all --check
cargo clippy -p kairo-ecs-debug --all-targets -- -D warnings
cargo test -p kairo-ecs-debug
cargo test -p kairo-ecs-core
cargo test -p kairo-ecs-state
pwsh -NoProfile -File scripts\validate_conductor_setup.ps1 -SkipCargo
pwsh -NoProfile -File scripts\validate_track_coverage.ps1 -SkipCargo
```
