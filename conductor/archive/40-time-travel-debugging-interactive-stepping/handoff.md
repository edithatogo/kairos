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

## Validation evidence — 2026-05-10

- `cargo +stable-x86_64-pc-windows-gnu test --manifest-path crates\kairo-ecs-debug\Cargo.toml --target x86_64-pc-windows-gnu` passed with `CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=rust-lld`.
- `cargo check --manifest-path crates\kairo-ecs-debug\Cargo.toml --tests --target x86_64-pc-windows-gnu` passed.
- `cargo fmt --manifest-path crates\kairo-ecs-debug\Cargo.toml --check` passed.
- `cargo clippy --manifest-path crates\kairo-ecs-debug\Cargo.toml --all-targets --target x86_64-pc-windows-gnu -- -D warnings` passed.
- `node website\time-travel-demo\validate-demo.mjs` passed.

## Closeout status

Track 40's owned paths now validate cleanly on the installed GNU Rust toolchain with `rust-lld`. No owned-path blocker remains.

## Known risks

The trace file size for large simulations (10M+ events) is the primary scalability concern and will require delta encoding with sparse snapshots from day one. Forward/backward parity is the critical correctness property — a single divergence would undermine user trust in the debugger. The browser timeline scrubber must remain a non-core dependency so the workspace builds without it.

## Integration notes

Next step: integrate the trace recorder with scheduler observer hook points and replace the line-oriented scaffold encoding with the Track 04 Arrow IPC trace serialization once that schema is available outside this Track 40-only write boundary.

## Worker 6 hardening evidence — 2026-05-06

- Tightened line-trace validation so delta event kinds must use the supported `custom:<u32>` encoding.
- Added a malformed event-kind rejection test and updated `docs/debugging/trace-format.md`.
- Added Track 40 coverage to the Track 36-40 aggregate offline validator alongside the static timeline demo validator.

## Follow-up issues

- Replace the line-oriented scaffold encoding with Track 04 Arrow IPC trace serialization once the schema is available in this worker's scope.
- Integrate trace recording with Track 01 scheduler observer hook points and compare recorded snapshots against Track 12 conformance fixtures.
- Add large-trace timeline validation after the browser demo supports aggregation or virtualization for 100K+ visible events.
## Phase closeout evidence

Ready for the next actual phase closeout once the shared registry surfaces are updated outside this track-only write boundary. Record `$conductor-review` findings, accepted fixes, deferred or blocked fixes, validation commands, cleanup state, commit SHA or explicit push blocker, pushed ref, strict `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` result, and next-phase decision here.
