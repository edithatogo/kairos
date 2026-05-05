# Handoff — 40 Time-Travel Debugging & Interactive Stepping

## Summary

Track 40 has moved beyond spec-design into an offline-testable scaffold. The `kairo-ecs-debug` crate now defines the trace schema, sparse snapshots, event deltas, reconstruction, stepping, backward movement, tick seek, inspection, breakpoint matching, and trace-line validation. The browser demo renders a versioned fixture and has a Node-based smoke validator that checks the trace fixture and step/back controls without browser dependencies.

## Files changed

`crates/kairo-ecs-debug/`, `docs/debugging/`, `website/time-travel-demo/`, and this handoff file.

## Contracts consumed

`conductor/workflow.md`, `conductor/contracts/conformance-contract.md`, Track 01 handoff notes (deterministic scheduler hooks), Track 12 handoff notes (conformance snapshot format), Track 22 experiment runner CLI interface.

## Contracts changed

No upstream scheduler or experiment-runner contracts were changed. The debug crate exposes `TRACE_SCHEMA = "kairo.ecs.trace.v1"` and `validate_trace_lines` as the local trace conformance surface for this scaffold.

## Tests added

- Unit tests in `crates/kairo-ecs-debug/src/lib.rs` cover replay reconstruction, step/back/goto, breakpoint matching, schema encoding, and trace-line validation.
- `website/time-travel-demo/validate-demo.mjs` validates the fixture schema, monotonic ticks, initial render, and step/back behavior with a minimal DOM harness.

## Known risks

The trace file size for large simulations (10M+ events) is the primary scalability concern and will require delta encoding with sparse snapshots from day one. Forward/backward parity is the critical correctness property — a single divergence would undermine user trust in the debugger. The browser timeline scrubber must remain a non-core dependency so the workspace builds without it.

## Integration notes

Next step: integrate the trace recorder with scheduler observer hook points and replace the line-oriented scaffold encoding with the Track 04 Arrow IPC trace serialization once that schema is available in this worker's writable scope.
