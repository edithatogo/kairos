# 40 Time-Travel Debugging & Interactive Stepping — spec.md

## Mission

Leverage KairoECS's deterministic core to provide time-travel debugging — the killer state-of-the-art differentiator that no other open-source DES framework offers. Record an event trace during execution, then step forward and backward to any simulation tick, inspect entity and component state at that tick, and replay from any point without state divergence.

## Primary subagent

```text
timetravel-agent
```

## Dependencies

```text
Track 01 (deterministic core), Track 12 (conformance — snapshot fixtures).
Can design trace format in parallel after the core event-log contract and snapshot format are stable.
```

## Owned paths

```text
crates/kairo-ecs-debug/, docs/debugging/, website/time-travel-demo/
```

## Blocked paths

```text
.github/ — owned by Track 13 (CI/CD)
crates/kairo-ecs-core/ — owned by Track 01
crates/kairo-ecs-state/ — owned by Track 01
crates/kairo-ecs-cli/ — owned by Track 22
conformance/ — owned by Track 12
bindings/ — owned by Tracks 06-11
```

## Parallel-safe with

```text
Most tracks are parallel-safe after their contract inputs are accepted. See conductor/parallel-execution.md for the wave model.
```

## Inputs

- Deterministic scheduler and event queue from Track 01.
- Conformance snapshot format from Track 12.
- CLI infrastructure from Track 22.
- Arrow IPC for trace serialization (Track 04).

## Outputs

- Debug crate `crates/kairo-ecs-debug/` with:
  - Event trace recorder that stores a snapshot and event delta at each simulation tick.
  - Time-travel debugger CLI with commands: step forward, step backward, go-to-tick, inspect entity/component state, set breakpoints on event kind or entity ID.
  - Trace replay engine that reconstructs state at any tick from the nearest snapshot + delta replay.
- Documentation in `docs/debugging/` covering trace format, CLI usage, and extension points.
- Browser-based timeline scrubber in `website/time-travel-demo/` with:
  - Interactive timeline with event dots.
  - State inspector panel for entity/component data at selected tick.
  - Play/pause/step controls for trace navigation.

## Acceptance criteria

- Record and replay a 1M-event trace without state divergence from the original run.
- Step backward to tick N produces identical entity/component state as the forward run at tick N.
- Breakpoint-on-event-kind pauses execution when matching event is dispatched.
- Go-to-tick command reconstructs correct state from nearest snapshot + deltas.
- Timeline scrubber renders in browser with event dots, tick marker, and state inspector panel.
- Trace round-trip: serialize trace to Arrow IPC, deserialize, replay, produce identical final state.
- `handoff.md` is completed before merge.

## Release implications

Non-blocking for initial release but is a major adoption driver. Should ship before 1.0 if implementation is complete. The debug crate is additive — no changes to the core scheduler or state engine are required beyond exposing the event trace hook points. Trace file format must be versioned to support forward compatibility across releases.

## Non-goals

- This track does NOT implement a VS Code debugger extension (that is a follow-on from Track 27, DX).
- This track does NOT provide live debugging of a running simulation (use Track 36 streaming for that).
- This track does NOT modify the core scheduler's event dispatch — it observes, does not intercept.
- This track does NOT guarantee trace compatibility across major scheduler version changes.

## Quality gates

Use the gates in `conductor/quality-gates.md`. Track-specific gates must be listed in `test-matrix.md`.
