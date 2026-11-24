# Native debugger replay

The Rust debug crate uses `Scheduler::checkpoint` and
`Scheduler::from_checkpoint` to save and restore the pending queue, clock
and counters, including cancellation counts. An application supplies a cloneable model and a
deterministic dispatch function. The model owns the state and RNG used by the
function. Its clones must provide independent mutable state. Shared immutable
data or copy-on-write pages can keep clones inexpensive.

`ReplaySession` numbers each dispatched event separately, including events
at the same tick and events scheduled during a transition. Cursor zero is the
starting state. A checkpoint captures the scheduler and model after the
transition at its cursor. Seek restores the latest checkpoint at or before the
requested position and executes the suffix against retained event identities,
ticks, priority, order, kind and entity. A seek that encounters a divergent
scheduler event leaves the current state in place. The event comparison
checks scheduler replay. Model correctness also depends on the dispatch
function and its snapshot contract.

If a step unwinds during a transition or model clone, the session requires a
seek to a retained position before continuing. The interrupted model and
scheduler stay inaccessible until restore. Seeking to the same position is
enough to restore the last recorded boundary.

The caller selects a nonzero checkpoint interval and count. Replay between
retained positions executes fewer events than one interval. A session holds at
most the selected count of scheduler and model checkpoints. Retained event
metadata is bounded by the interval times that count. The number of bytes per
checkpoint depends on the model and scheduler. A fork captures its own
scheduler and branch model at the current position, applies the caller's edit,
and begins retaining a new timeline there.

## Arrow dispatch export

The opt-in `arrow-ipc` feature reuses the native Arrow event-log v2 writer.
For example, `cargo run -p kairo-ecs-debug --features arrow-ipc -- replay
--dispatch-ipc dispatches.arrow --run-id example` publishes the stream when
the replay input ends successfully. The output contains completed dispatches
after the oldest retained checkpoint through the final cursor. A rewind
excludes later events, and an inherited fork boundary stays outside the
branch's dispatch interval. The stream records event identities, full-width
ticks and dispatch ordering. Its scope is dispatch records. Model snapshots,
RNG state and browser replay use their own contracts.

## Compatibility

`EventTrace` retains its in-memory types and v1 line format. Explicit
snapshots are end-of-tick values. The default trace starts empty so it can
apply real tick-zero deltas. Per-event inspection stops at the cursor's event
within the tick. The CLI's former placeholder commands now run inside
`replay`. The existing `record` statistics command still accepts the same
arguments. This API has in-process lifetime and relies on a qualified
model's determinism. The browser demo continues to play fixture data.

## Conformance

The [native replay fixture][replay-test] exercises tick-zero and same-tick
ordering, a cancelled event, a new event inserted during dispatch, a tick
above 64 bits, retained-history limits and isolation of forked state, RNG
and queues. It uses the existing canonical RNG vector. The [CLI test][cli-test]
runs the compiled binary through the same-tick step, back, inspect and
event-kind breakpoint commands. The optional export test checks a retained
window after eviction and rewind against an independently specified event.
The [size and cost example][memory-example]
compares periodic checkpoints with copy-on-write pages against full copies at
every event over the same retained range.

[replay-test]: ../../crates/kairo-ecs-debug/tests/native_replay.rs
[cli-test]: ../../crates/kairo-ecs-debug/tests/native_cli.rs
[memory-example]: ../../crates/kairo-ecs-debug/examples/replay_memory.rs
