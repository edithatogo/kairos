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
awaiting pruning. Consumers own model state, entity lifetime and RNG checkpoints.
FFI and other scheduler variants require their own contracts.

The `scheduler_checkpoint` conformance target compares complete dispatched
events, counters, future ID allocation and same-tick priority/insertion order.
It verifies independent cancellation in a sibling and rejects an incompatible
version. Paignia combines this native state with actual policy and RNG state in
its own T17.16 replay fixture.

## Durable Rust checkpoint format

The opt-in `checkpoint-json` feature adds `Scheduler::checkpoint_json` and
`Scheduler::from_checkpoint_json`. The preview API uses version 1 JSON with
typed integers, explicit entity handles and a pending array ordered by increasing
insertion sequence. Both clock and event ticks retain all 128 bits. Interpret
these fields with a parser that preserves integer precision. The version field
selects the decoder before the version-specific state is read.

For the v1 scheduler, one successful schedule advances the sequence, event
index and scheduled total together. The event generation wraps at 32 bits.
The format records one counter for these correlated values. Cancelled events
remain counted and their tombstones can be discarded. On decode, checked event
counts and strictly increasing live sequences preserve pending membership,
reservation of retired IDs and deterministic dispatch. The decoder rejects
malformed data, inconsistent state and incompatible versions. A nonzero clock
requires a previous dispatch. An exhausted schedule counter is reported by
both checkpoint operations, preserving the contract that a restored scheduler
can still allocate an event.

Consumers provide integrity-checked bytes and publish the scheduler, model and
RNG state as one logical checkpoint at the same event boundary. A structurally
valid edit can change model behavior, so format validation alone provides no
integrity guarantee. This API leaves default features, the existing in-process
checkpoint, the beta binding surface and event scheduling semantics intact.

The `scheduler_checkpoint_json` target fixes the v1 representation, tests
generation wrap and malformed inputs, and compares a fresh-process queue model's
full suffix, next IDs, RNG state and final state with an uninterrupted run.
