# Experimental Flow queue migration

The Flow facade is an experimental, single-world API. It changes the timing of
resource commands and the shape of claim ownership. This guide describes the
current source surface; it is not a stable API promise or a semver guarantee.
The [`kairo-ecs-des` API review](../design/api-reviews/flow-runtime-q0.1.md)
classifies the Flow roots as experimental, and Track 25 compatibility and
release review remain open.

## From `Resource` to `FlowRuntime`

The legacy [`Resource`](../../crates/kairo-ecs-des/src/lib.rs) helper grants a
free unit synchronously through `request(entity) -> bool`. When full, it queues
the entity FIFO; `release() -> Option<EntityId>` immediately returns the next
waiter. `DESContext::new(_seed)` retains its current signature but ignores that
argument; do not infer a new or deterministic RNG stream from it. The
[FIFO migration example](../../crates/kairo-ecs-des/examples/flow_fifo_migration.rs)
shows three equal-priority claimants and compares labels, not IDs from separate
worlds.

Flow uses an opaque resource ID, actor ID, request ID and lease ID. A typical
manual claim is:

```rust
let actor = flow.spawn_actor()?;
let resource = flow.create_resource(1)?;
let request = flow.acquire(resource)
    .owner(actor)
    .priority(0)
    .submit()?;
flow.step()?; // dispatch commits admission/grant work
let lease = flow.request(request)?.lease.ok_or(FlowError::InvalidState)?;
flow.release(lease, flow.now())?;
flow.step()?; // dispatch commits release
```

The builder's current methods are `owner`, `at`, `for_work`, `timed_work`,
`can_preempt`, `preemptible`, `priority`, `deadline`, `scheduler_priority`, and
`submit`; see [`AcquireBuilder`](../../crates/kairo-ecs-des/src/flow.rs). A
submitted request begins `Pending`. Dispatch moves it to `Active` or `Queued`.
Release schedules a command for the actual active lease; the lease is not freed
until that command dispatches. A released lease cannot be reused.

`priority(level)` orders claims for a resource; equal levels retain admission
order. `scheduler_priority(value)` orders scheduled commands and is a separate
axis. `can_preempt(true)` gives the incoming claim permission to replace an
eligible holder. The holder's `.preemptible(PreemptionStrategy::...)` selects
what happens to that holder. These settings apply to different requests and are
not interchangeable.

## Deadlines, cancellation and interruption

A waiting deadline is inclusive for expiry: when `deadline <= dispatch time`,
the waiting request expires before grant arbitration. A successful first grant
clears its waiting deadline; later Suspend or Restart does not restore it.
This is a first-grant wait limit, not a work-completion deadline. See the
[Q5.1 boundary map](../../conductor/design/queue/q5.1-conformance-20261004.md)
and the [deadline-index contract](../../conductor/design/queue/q5.2-deadline-index-20261004.json).

`FlowRuntime::cancel(request_id, at)` schedules cancellation of a Flow resource
request. It does not cancel an arbitrary scheduler event. The core
[`Scheduler::cancel(EventId)`](../../crates/kairo-ecs-core/src/lib.rs) is a
separate API. The Q5.1 fixture map covers scheduled-event cancellation with a
separate core oracle; it does not claim that Flow exposes it. Cancellation is
committed at dispatch, like other buffered Flow commands.

For timed work, `Suspend` retains the typed in-memory context and remaining
duration for a later resume. `Abort` ends the interrupted work. `Restart` uses
the registered restart factory to create fresh context and resets remaining
duration to the original duration; the engine does not implicitly re-sample.
Any sampling performed by the caller's factory is caller behavior. These are
same-runtime behaviors covered by the Flow lifecycle fixtures; they do not
serialize callbacks or context. The [staff/bed/cleaning example](../../crates/kairo-ecs-des/examples/flow_staff_bed_cleaning.rs)
demonstrates Suspend with staged one-resource claims and compares continuous
execution with pause/continue in the same live runtime.

## Lifecycle records and continuation

`LifecycleRecord.snapshot` captures the queue, allocation and progress values at
that transition. Downstream exhaustive struct literals must add the field; do
not reconstruct intermediate values from the final World. The
`resource_lifecycle.v1` encoder validates contiguous ordinals and uniqueness
within the supplied batch only. No whole-run ordinal uniqueness writer is
provided. The [Q5.2 integration context](../../conductor/design/queue/q5.2-queue-integration-context-20261004.md)
contains the implementation boundary.

`run_for`/`step` and the example's pause/continue path continue the same live
`FlowRuntime`. [`WorldSnapshot` and Track 22's checkpoint handoff](../../conductor/design/queue/q0.3-snapshot-track22-handoff-20260930.md)
do not provide a portable Flow snapshot or restore format: queue state, leases,
scheduler state, components and RNG continuation are not encoded there. Track 22
owns a later coordinated checkpoint/restore contract.

The examples have distinct stdout oracles. The FIFO migration example prints
the three exact lines in [`examples/flow/README.md`](../../examples/flow/README.md).
The staff example's source-level stdout oracle is its ordered `continuous.records`
loop at [`flow_staff_bed_cleaning.rs:488`](../../crates/kairo-ecs-des/examples/flow_staff_bed_cleaning.rs#L488),
after the full continuous result is asserted equal to the pause/continue result
at line 487. Each row uses
`t=<ticks> <label> <transition> priority=<n> queue=<n> active=<n>`.
The example also checks terminal
states and resource conservation. The coordinator records the exact stdout for
the current source; this page does not claim a captured runtime receipt. These
checks are synthetic API examples, not clinical rules or validation.
