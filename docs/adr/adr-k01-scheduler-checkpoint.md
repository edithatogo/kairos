# Native scheduler checkpoints for bounded replay

Paignia T17.16 requires actual native event identities and order to survive a
resume. The single-threaded scheduler owns current time, live pending IDs, the
priority heap, cancellation tombstones, counters and the next ID and sequence.
Reconstructing the queue from dispatched or preview records would lose part of
that state.

`Scheduler::checkpoint` captures an owned `Versioned<SchedulerCheckpoint>`.
The opaque Rust checkpoint contains the complete scheduler state at capture.
`Scheduler::from_checkpoint` validates `SCHEDULER_CHECKPOINT_VERSION_V1` and
restores an independent copy. The original and either branch subsequently own
their own queue and cancellation state. Event IDs remain local to each scheduler.
Callers retain a branch or run identity when comparing sibling event records.

The API is additive to the Rust scheduler. The in-process snapshot's space and
capture/restore cost scale with the scheduler queue, including cancelled entries
awaiting pruning. Model state, entity lifetime and RNG checkpoints are separate
consumer obligations. Durable serialization, FFI and other scheduler variants
require their own contracts.

The `scheduler_checkpoint` conformance target compares complete dispatched
events, counters, future ID allocation and same-tick priority/insertion order.
It verifies independent cancellation in a sibling and rejects an incompatible
version. Paignia combines this native state with actual policy and RNG state in
its own T17.16 replay fixture.
