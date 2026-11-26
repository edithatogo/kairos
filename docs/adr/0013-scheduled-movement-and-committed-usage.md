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

The [movement fixture](../../crates/kairo-ecs-abm/tests/movement_allocation.rs)
drains the allocator before processing an arrival and again after scheduling
its lease release. The fixture checks FIFO admission for requests submitted
before the arrival and at the arrival tick. A later request stays scheduled
for its own tick. The [allocator fixture](../../crates/kairo-ecs-des/tests/maturity_resource_conservation.rs)
checks an exhausted budget inside a same-tick pair and a cancelled close
before a future live event.
