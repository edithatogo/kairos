# ADR 0013 — Scheduled movement and committed staged usage

Accepted, 22 November 2026.

Movement is an ABM operation on the model's existing scheduler and topology.
The scheduler owns event order and cancellation. Movement owns location,
operation identity and route revision. The model composes movement with
staged allocation by closing the associated request when the journey ends.
See the [movement contract](../../conductor/contracts/movement-resource-usage.md)
and [composition contract](../../conductor/contracts/movement-allocation-composition.md).

Optimistic usage comes from a checkpoint corresponding to the exclusive
committed cutoff. The DES adapter reuses the forward occupancy schema from
[ADR 0012](0012-committed-resource-usage.md) and retains the producing LP and
cutoff as export context. An observation end strictly before the cutoff
makes release provenance stable even under rollback. See the
[replay contract](../../conductor/contracts/usage-replay.md).

## Bounded allocator dispatch

`StagedResourceRuntime::run_until_or_for` is an additive experimental Rust API.
It dispatches up to the supplied event budget through the inclusive model tick.
It uses the allocator's native order and returns the number of dispatched
actions. The allocator clock identifies the last dispatch. The caller owns the
model clock, the order between schedulers and any occupancy seal. Existing
request, lease and checkpoint contracts apply.

The [allocator fixture](../../crates/kairo-ecs-des/tests/maturity_resource_conservation.rs)
checks an exhausted budget inside a same-tick pair and a cancelled close
before a future live event.

## Coordinating allocator and model dispatch

`StagedResourceRuntime::step_with_scheduler` is an additive experimental Rust
API in `kairo-ecs-des`. It selects one live event through an inclusive limit.
Allocation takes precedence at a tie. Each scheduler retains its priority
and insertion order. The model gets an allocation tick or the original model
event and applies its reaction before requesting the next step. A one-event
budget lets the model pause even during a chain of same-tick reactions.

An eligible event earlier than either last dispatch is rejected before taking
effect. That check catches backdated work accepted by a lagging scheduler.
The step relies on the existing scheduler and the staged runtime's lease,
checkpoint and usage rules.

The [movement fixture](../../crates/kairo-ecs-abm/tests/movement_allocation.rs)
starts movement on allocation, runs the due allocator requests before the
arrival, and releases the lease before the next model event at that tick.
The earlier waiter is admitted first and the later tick's request remains
scheduled.

## Preview for a model-owned dispatcher

`StagedResourceRuntime::peek_with_scheduler` and `StagedDispatchPreview` are
additive experimental Rust APIs in `kairo-ecs-des`. Both the preview and
`step_with_scheduler` use the same ordering and clock checks. The preview
leaves live events and clocks in place, so a model such as
`BehaviorSimulation` can run its own handler through its regular dispatcher.
Scheduling, cancellation or dispatch can change which event is next. Take a
new preview after such changes.
[Native behavior conformance case](../../crates/kairo-ecs-abm/tests/abm_behavior_update_v1.rs)

## Preview across multiple model queues

`StagedResourceRuntime::peek_with_schedulers` and `StagedDispatchSelection`
are additive experimental Rust APIs in `kairo-ecs-des`. An eligible event is
selected by tick, then allocator precedence or model slice order. The
returned queue index distinguishes events with the same ID in separate
schedulers. The single-scheduler preview uses the same selector.

All participating queues use a common time unit and retain their positions.
The latest dispatch tick across all queues, including idle ones, sets the
lower bound. Each owner dispatches its selected event. The qualified case
uses one allocator and two model queues in a single process.

## Committed usage at a model observation tick

`StagedResourceRuntime::committed_usage_with_schedulers` is an additive
experimental Rust API in `kairo-ecs-des`. It uses the existing occupancy
format, interval calculation and seal. When a model queue reaches the
observation end while the allocator is idle, the export can use that tick.
An eligible event in any participating queue blocks export, even when
another clock has already reached the end. Finish the handlers and their
reactions through the endpoint before observing.

The clocks retain their actual dispatch ticks. A successful export clips
occupancy and seals the allocator through the endpoint. The
[conformance case](../../crates/kairo-ecs-abm/tests/movement_allocation.rs)
checks an unreached endpoint, due same-tick model work, a censored lease
and later arrival with release.

## Boundary wakeups in coordinated models

`BoundaryChannel::next_event_time` is an additive experimental Rust API in
`kairo-ecs-des`. It previews the earliest pending arrival or first tick past
a live transfer's inclusive deadline. A model can schedule that tick in its
DES queue alongside ABM updates and staged resource events.

At a wakeup, process expiry before collecting arrivals. Expiry batches
return operation IDs in ascending order so simultaneous expiry reactions
are reproducible. A model refreshes its scheduled wakeup when it changes
channel state and handles overdue ticks at the current coordinated tick.
Arrival batches follow scheduled arrival order, with departure order at
equal ticks. The receiver acknowledges a delivered transfer according to
the model's handover policy.
[Boundary scheduling contract](../design/boundary-channels.md#6-coordinated-boundary-wakeups)

## Representable boundary arrivals

`BoundaryChannel::depart` checks the scheduled arrival before committing
the departure and sampled random stream. The last representable tick is
valid. An arrival beyond it returns `BoundaryError::ArrivalOverflow`.
The new variant changes exhaustive matches on the experimental
`kairo-ecs-des` Rust error enum. Valid delay sampling and scheduled
timestamps retain their meaning.
[Boundary error migration](../design/boundary-channels.md#7-arrival-tick-overflow)
