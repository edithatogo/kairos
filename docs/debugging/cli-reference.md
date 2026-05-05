# Debugger CLI Reference

Planned commands:

- `step`: move the cursor to the next event delta.
- `back`: move the cursor to the previous event delta.
- `goto <tick>`: move to the first event at or after a tick.
- `inspect <entity-or-key>`: print reconstructed state at the cursor.
- `break on <event-kind>`: add an event-kind breakpoint.
- `list-breakpoints`: show active breakpoints.

Current scaffold status:

- The binary accepts the command names and prints a scaffold message.
- The library starts at the initial snapshot; the first `step` moves to the first delta.
- `back`, `goto <tick>`, `inspect`, event-kind/entity breakpoints, and `run_until_breakpoint` are covered by crate smoke tests.
- Full trace-file CLI execution remains deferred until the Track 22 runner integration is available.

## Tutorial: local trace smoke

Use these checks for docs examples and demo changes:

```bash
cargo check --manifest-path crates/kairo-ecs-debug/Cargo.toml --tests
node website\time-travel-demo\validate-demo.mjs
```

The Rust command proves the library and CLI scaffold compile against the trace
types. The Node command executes the static timeline demo in a minimal DOM
harness, validates the fixture schema, steps forward and back, selects an event
dot, and verifies the state inspector refreshes.

## Evidence boundary

Current evidence proves offline trace reconstruction, cursor behavior,
breakpoint matching, line-format validation, and static demo rendering. It does
not prove scheduler hook integration, Arrow IPC trace files, Track 12 fixture
parity, large-trace virtualization, or Track 22 debugger subcommands.
