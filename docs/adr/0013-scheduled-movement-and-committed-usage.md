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
API. It selects one live event through an inclusive limit. Allocation takes
precedence at a tie. Each scheduler retains its priority and insertion order.
The model gets an allocation tick or the original model event and applies its
reaction before requesting the next step. A one-event budget lets the model
pause even during a chain of same-tick reactions.

An eligible event earlier than either last dispatch is rejected before taking
effect. That check catches backdated work accepted by a lagging scheduler.
The step relies on the existing scheduler and the staged runtime's lease,
checkpoint and usage rules.

The [movement fixture](../../crates/kairo-ecs-abm/tests/movement_allocation.rs)
starts movement on allocation, runs the due allocator requests before the
arrival, and releases the lease before the next model event at that tick.
The earlier waiter is admitted first and the later tick's request remains
scheduled.
