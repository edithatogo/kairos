# ADR-0008 — C20 experimental transit retry ingress

Status: proposed for C2 coordinator and Track03 owner acceptance; implementation
authorization is pending. No stable API acceptance follows from this proposal.
Date: 2026-10-06. Owners: Track03/21; API/MSRV Track25/30.

## Context

C20 assigns one-shot retry provenance to private calibration `BoundIntrinsicWork`:
it retains runtime identity, carrier, kind, pending source EventId, priority and
the returned `FlowDispatch` receipt. A rejected planned start or progress callback
consumes its source event but transactionally retains `TransitContext.expected_event`
and `expected_due`. The existing ABM start helper only binds a context with no
expected event, and these fields are private. Calibration cannot safely make a
bare replacement schedule: the actual context would treat its new EventId as
stale. The existing DES API has `FlowRuntime::schedule_domain_and_bind`, which
schedules before an infallible typed-context binder runs.

## Proposed API

Add one hidden, experimental ABM function for the calibration adapter's C20 retry
path:

```rust
#[doc(hidden)]
pub fn schedule_transit_retry(flow:&mut FlowRuntime,carrier:WorkId,
    kind:EventKind,rejected:&FlowDispatch,priority:i32)
    ->Result<EventId,FlowError>;
```

The helper checks that the actual context belongs to `flow`, is Ready or Moving,
has the exact expected EventId and due matching `rejected.event`, `rejected.at` and
`flow.now()`, and that the dispatch contains exactly one Rejected callback-batch
receipt. The regular Flow scheduler check validates the actual context type and
registered carrier/kind. It schedules the same carrier/kind at actual now and
uses `schedule_domain_and_bind`; its binder changes only `expected_event`. The due
is unchanged because the retry time equals the consumed source dispatch time. The
caller supplies the priority retained in its unique `BoundIntrinsicWork`; only
that adapter may call the helper in C20. It must validate its runtime, carrier,
kind, pending ID, priority and actual returned dispatch before the call.

`FlowDispatch` is a public caller-supplied value, not an unforgeable scheduler
capability. This helper provides structural cross-checks, not cryptographic or
hostile-caller authenticity. The first-party C2 adapter's preconditions are the
provenance boundary. Callers outside that adapter must not use a fabricated
`FlowDispatch` as proof of actual dispatch. A stale/replayed ID, mismatched
runtime/carrier/kind/time, accepted or malformed receipt, invalid phase, or
scheduling failure must not change context or queue. Replaying the old dispatch
after a successful retry fails because the actual context expects the replacement
ID. Controls remain owned by Bound and use the existing typed control scheduler
with the retained action; this API handles only expected ordinary start/progress
events. No automatic retry is added.

## Required tests

1. A real rejected start retains Ready state and the old expected ID; one explicit
   retry schedules at current time on the same carrier/kind/retained priority;
   accepted delivery begins movement once.
2. A real rejected progress event preserves edge, elapsed and remaining progress;
   retry rebinds once and results in one arrival and one acquire.
3. Replaying the old rejection after success fails; accepted, malformed/multiple,
   foreign-runtime, wrong-carrier/kind/event/time and invalid-phase inputs fail
   without scheduling.
4. Scheduling failure preserves expected ID/due and route progress.
5. Calibration tests show the unique Bound adapter checks actual returned
   dispatch identity/receipt and retained carrier, kind and priority before
   invoking this helper. A rejected initial start is covered even though the
   context's carrier field is not committed until an accepted callback; Bound must
   validate its retained carrier/kind. Bound exposes no priority override, so a
   retry cannot silently change it. Document the trusted-caller boundary for
   caller-constructible FlowDispatch.
6. Existing controls retain separate ownership/retry behavior; the full C20
   fixture still emits exactly one accepted actual service request.

## Ownership and acceptance

The implementation touches Track03-owned `crates/kairo-ecs-abm`; Track03's
agent contract requires an explicit owner handoff for that path. This ADR is only
a proposal until the C2 coordinator and Track03 owner accept the exact API and
bounded write reservation. Track25/30 release compatibility gates remain open.
The function is experimental and hidden from generated docs, with no stable Rust
API promise. No DES source, event ordering, callback cause, schema, ABI, binding,
random stream, or Flow receipt shape changes.

## Non-goals

No unforgeable retry token, generic retry API, DES scheduler change, carrier/task
recreation, portable checkpoint, C2.1 acceptance or clinical validation is
introduced.
