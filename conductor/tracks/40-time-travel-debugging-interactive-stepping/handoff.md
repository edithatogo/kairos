# Handoff — 40 Time-Travel Debugging & Interactive Stepping

## Summary

Track 40 remains an offline-testable scaffold, not a scheduler-integrated debugger. The `kairo-ecs-debug` crate defines the trace schema, sparse snapshots, event deltas, reconstruction, stepping, backward movement, tick seek, inspection, breakpoint matching, and trace-line validation. This hardening slice fixed first-step cursor behavior, added run-to-breakpoint/current-state helpers, tightened line validation, and expanded the browser demo smoke validator.

## Files changed

`crates/kairo-ecs-debug/`, `docs/debugging/`, `website/time-travel-demo/`, and Track 40 conductor files.

## Contracts consumed

`conductor/workflow.md`, `conductor/contracts/conformance-contract.md`, Track 01 handoff notes (deterministic scheduler hooks), Track 12 handoff notes (conformance snapshot format), Track 22 experiment runner CLI interface.

## Contracts changed

No upstream scheduler or experiment-runner contracts were changed. The debug crate exposes `TRACE_SCHEMA = "kairo.ecs.trace.v1"` and `validate_trace_lines` as the local trace conformance surface for this scaffold.

## Tests added

- Unit tests in `crates/kairo-ecs-debug/src/lib.rs` cover replay reconstruction, initial-snapshot-to-first-delta stepping, step/back/goto, run-to-breakpoint, schema encoding, and trace-line validation rejection cases.
- `website/time-travel-demo/validate-demo.mjs` validates the fixture schema, monotonic ticks, initial render, active event marker, step/back behavior, no duplicate event dots after re-render, event-dot selection, and state inspector refresh with a minimal DOM harness.

## Validation evidence — 2026-05-06

- `cargo fmt --manifest-path crates\kairo-ecs-debug\Cargo.toml --check` passed.
- `cargo check --manifest-path crates\kairo-ecs-debug\Cargo.toml --tests` passed.
- `cargo clippy --manifest-path crates\kairo-ecs-debug\Cargo.toml --all-targets -- -D warnings` passed.
- `node website\time-travel-demo\validate-demo.mjs` passed.

Blocked:

- `cargo test --manifest-path crates\kairo-ecs-debug\Cargo.toml` failed before test execution because `C:\Users\60217257\scoop\apps\git\current\usr\bin\link.exe` returned Win32 error 5 while creating a signal pipe.
- `$env:RUSTFLAGS='-C linker=rust-lld'; cargo test --manifest-path crates\kairo-ecs-debug\Cargo.toml` failed before test execution because the MSVC import libraries `kernel32.lib`, `ntdll.lib`, `userenv.lib`, `ws2_32.lib`, and `dbghelp.lib` are not on the linker search path.

## Known risks

The trace file size for large simulations (10M+ events) is the primary scalability concern and will require delta encoding with sparse snapshots from day one. Forward/backward parity is the critical correctness property — a single divergence would undermine user trust in the debugger. The browser timeline scrubber must remain a non-core dependency so the workspace builds without it.

## Integration notes

Next step: integrate the trace recorder with scheduler observer hook points and replace the line-oriented scaffold encoding with the Track 04 Arrow IPC trace serialization once that schema is available outside this Track 40-only write boundary.
