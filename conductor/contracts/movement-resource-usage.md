# Movement, allocation and resource-usage contract direction

Accepted design direction, 7 November 2026; **implementation pending** under
Track 114 and scientific obligation S17. Names below describe semantic fields,
not an assertion that these public Rust types or schemas already exist. Settle
the exact Rust API in a bounded contract task before implementation.

## Existing source to reuse

- ABM `routing.rs`: `SpatialTopology`, `SpatialRoute`, access filtering, topology
  revisions and tick-based duration. Route queries remain pure; the existing
  `test_des_boundary_route_rendering_does_not_schedule_events` guards this.
- ABM `geometry.rs` and `spatial_index.rs`: geometry/interpolation and updates;
  reuse qualified `petgraph`, `geo` and `rstar` integrations.
- DES `boundary.rs`: generic handover/transit, acknowledgement, cancellation and
  expiry. A boundary handover is not automatically a continuous spatial journey.
- DES `staged_runtime.rs`: scoped actor/resource/request handles, scheduler-backed
  FIFO contention and leases. Current lease capacity is one unit; snapshots
  alone are not a time-stamped usage ledger.

## Separate contracts

| Contract | Required semantics | Excluded domain meaning |
| --- | --- | --- |
| Location/route | Optional location; coordinate reference/units; stable topology/revision; access policy input; path and tie rule | Hospital site assumptions, clinical mobility rules, economic value |
| Movement | Operation ID; actor; origin/destination; departure/arrival ticks; checked duration; causal event; state and cancellation | Visual animation as engine state; universal travel-time law |
| Allocation | Resource kind/ID; request/lease IDs; quantity; admission/release; priority/queue policy; atomicity and preemption | Prices, QALYs, payer perspective |
| Usage | Run/scenario/branch/event IDs; actor/resource/lease; instant or interval; quantity/unit; cause and revision | Cost or utility inferred from a resource name |
| Economic valuation | Explicit mapping of usage and health outcomes to model-specific value | Scheduler/ECS implementation or replacement RNG |

## Movement lifecycle

Use checked integer simulation ticks and preserve the scheduler's same-time tie
order. A route with distance/effort cost cannot be converted to duration without
a domain-supplied conversion. Define zero-duration movement, time overflow,
destroyed actors, no route, stale topology/access and interrupted movement.
Arrival changes location once. Cancellation cannot leave a later arrival that
moves the entity. An operation's idempotency and generation prevent stale events
from affecting a reused entity ID. Replanning has a new revision and explicitly
cancels/supersedes the old arrival.

Movement may require resources, but does not imply it: a consumer composes a
movement operation with a lease through public APIs. Resource acquisition,
journey cancellation and resource release need one well-defined ordering and
no leak on partial failure. Keep route queries and display interpolation free
of side effects on simulation state. Rollback restores pending operations and
locations along with the scheduler; visual playback alone is not rollback.

## Usage semantics

Distinguish occupancy (quantity held over an interval), consumption (quantity
used once), and transfer (ownership/location change). A transfer is not another
consumption. Start occupancy at actual admission, not request submission. Use
half-open intervals `[start, end)` so release and next admission at the same
tick are not double-counted. Integrate quantity times overlap duration for an
observation window, retaining explicit tick scale and physical unit.

Open leases are censored at the observation endpoint without fabricating a
release. Repeated rejected releases emit no duplicate usage. Preemption splits
intervals; resume starts a new interval. Quantity changes have explicit segments.
Partial allocations are forbidden unless a separately named policy allows them.
Define FIFO/head-of-line versus bypass policy before quantity requests ship.

An authoritative usage record must include a stable idempotency key, schema
version, run/scenario/branch identity, causal event and engine/model/RNG version
references. For optimistic execution choose committed-only export or explicit
revision/retraction records; never silently sum provisional and rolled-back
usage. Define late/corrected records and reject duplicates with conflicting
content. Keep Arrow/Parquet an optional encoding of the same semantic records.

## Domain and HEOR responsibilities

CareOps supplies physical workflow, accessibility, resource classifications and
clinical outcomes. Innovate supplies adoption/implementation mechanisms. A
separate HEOR model supplies cost categories, price-year/currency conversion,
perspective, horizon, discounting and utility assumptions. Preserve uncertainty
draw and strategy identity throughout; otherwise paired VOI comparisons can be
invalid even when individual rows are well formed.

Use a tiny worked fixture: one actor, one move, a porter lease, a bed interval,
one consumable and an explicitly censored lease. First verify quantities and
timing by hand. Then apply two named valuations to the **same** usage to show
that changing perspective does not rerun or alter the engine. Keep transfers
and costs distinct, and test a bundled-rate double-counting counterexample.

## Completion and task ownership

The workpack separates arrival, cancellation, admission timestamps, quantity
atomicity, window integration and replay into bounded leaves. Numerical/replay
and public-schema decisions receive one specialist review; settled serializers,
fixture tests and adapters can be implemented by smaller local models. Use the
existing path-lease harness. No new approval board or universal workflow engine.

Acceptance requires executed Rust trajectories and negative cases, not schema
validation alone. Cross-repository fixtures live with their contract owner and
are consumed at immutable revisions. Consumer pins advance in consumer-owned
changes after their actual model passes.
