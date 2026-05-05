# 40 Time-Travel Debugging & Interactive Stepping — plan.md

## Phase 0 — Track startup

- Read `conductor/workflow.md`, `conductor/contracts/conformance-contract.md`, and the Track 01 and Track 12 handoff notes.
- Confirm owned paths: `crates/kairo-ecs-debug/`, `docs/debugging/`, `website/time-travel-demo/`.
- Review the deterministic scheduler's event dispatch hook points for trace recording.
- Create `agent-contract.md`, `risk-register.md`, `test-matrix.md`, and `handoff.md`.

## Phase 1 — Contract alignment

- Define the event trace format: snapshot + delta encoding, tick-aligned recording.
- Define the trace versioning scheme with forward-compatibility rules.
- Align trace snapshot format with conformance fixtures from Track 12.
- Define the debugger CLI commands and output format.
- Define the browser timeline scrubber data protocol.
- Propose contract changes through ADR if required.

## Phase 2 — Scaffold

- Create the `crates/kairo-ecs-debug/` crate.
- Add `docs/debugging/` with trace format specification and CLI reference.
- Add a smoke test that records and replays a trivial trace.
- Wire the crate into the workspace `Cargo.toml`.

## Phase 3 — Implementation

- Implement the event trace recorder: hooks into the core scheduler to capture snapshot + event delta at each tick.
- Implement snapshot-keyed delta replay: reconstruct state from nearest snapshot by replaying forward or backward.
- Implement debugger CLI: `step`, `back`, `goto <tick>`, `inspect <entity>`, `break on <event-kind>`, `list-breakpoints`.
- Implement trace serialization to Arrow IPC format.
- Implement the browser-based timeline scrubber with play/pause/step controls, event dots, and state inspector.
- Add unit tests for trace recording, replay, snapshot reconstruction, and breakpoint logic.
- Add integration tests for forward/backward parity.

## Phase 4 — Cross-track integration

- Validate that recorded traces match conformance fixture snapshots at each tick (Track 12).
- Ensure Arrow IPC trace serialization is compatible with the telemetry schema (Track 04).
- Integrate the debugger CLI as a subcommand of the experiment runner (Track 22).
- Run owned tests plus affected shared tests.
- Update docs and release notes.

## Phase 5 — Closeout

- Complete `handoff.md`.
- Record remaining decisions, trace format trade-offs, and follow-up tasks.
- Confirm CI gates including trace round-trip and forward/backward parity.
- Mark the track ready for the next implementation wave.

## Worker 5 evidence — 2026-05-06

Completed with artifact evidence:

- Phase 1 trace contract scaffold: `docs/debugging/trace-format.md` defines `kairo.ecs.trace.v1`, sparse snapshots, event deltas, and forward-compatibility rules.
- Phase 1 CLI contract scaffold: `docs/debugging/cli-reference.md` lists `step`, `back`, `goto`, `inspect`, `break`, and `list-breakpoints`.
- Phase 2 debug crate scaffold: `crates/kairo-ecs-debug/` contains a standalone crate with trace snapshots, deltas, reconstruction, stepping, backward movement, tick seek, inspection, and breakpoint matching.
- Phase 2 smoke tests: `crates/kairo-ecs-debug/src/lib.rs` includes unit tests for replay reconstruction, stepping/back/goto, breakpoint matching, and trace schema encoding.
- Phase 3 browser scrubber scaffold: `website/time-travel-demo/` provides a static timeline with play/pause, step/back, event selection, and state inspection.

Validation evidence:

- `cargo check --manifest-path crates\kairo-ecs-debug\Cargo.toml --tests` passed.

Not marked complete:

- `cargo test --manifest-path crates\kairo-ecs-debug\Cargo.toml` did not execute because the local Windows linker setup is broken: `where link` resolves to Git's `usr\bin\link.exe`, which failed with Win32 error 5; retrying with `RUSTFLAGS='-C linker=rust-lld'` failed because `kernel32.lib`, `ntdll.lib`, `userenv.lib`, `ws2_32.lib`, and `dbghelp.lib` are not on `LIB`.
- Arrow IPC serialization and experiment-runner integration remain future work because they require Track 04/22 integration outside this worker's write paths.
