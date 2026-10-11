# Public Flow event preview

`FlowRuntime::peek_next(&mut self) -> Option<ScheduledEventPreview>` returns a
copied view of the next live scheduled event without dispatching it. The view
includes event identity, time, scheduler priority, insertion sequence, entity,
and kind. `None` means there is no live event.

The method delegates to the shared scheduler's existing preview. It may prune
cancelled heap tombstones, while virtual time, scheduler counters, live pending
membership, and Flow domain state remain unchanged. It includes live stale
Completion entries; it does not decide whether an event is useful or discard
one because of its revision. Preview remains available after a transition
budget halt and does not clear that halt.

The exclusive mutable borrow permits the scheduler's tombstone cleanup. The
returned value is an observation, not a reservation: later scheduling or
cancellation may change the next event. Calling preview does not grant a
resource, deliver a callback, consume transition budget, or advance time.

Callers own horizon policy. A caller that admits events through horizon `H`
dispatches the previewed event when `at <= H`; an event at `H + 1` remains
pending and previewable without dispatch or clock advance.

This additive experimental Rust API does not change core scheduler behavior,
checkpoint layouts, bindings, or existing event ordering. The seven named
`flow_preview_` runtime proofs and source-stage checks are evidence for this
implementation only; combined qualification and API acceptance remain separate
gates.
