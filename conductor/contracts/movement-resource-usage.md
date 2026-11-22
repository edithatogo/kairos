# Movement, allocation and resource-usage contract

Track 114 and scientific obligation S17 own this contract. The Rust API below
defines the scheduled movement lifecycle. Resource usage and allocation
composition have separate [replay](usage-replay.md) and
[composition](movement-allocation-composition.md) contracts.

## Existing source to reuse

- ABM `routing.rs`: `SpatialTopology`, `SpatialRoute`, access filtering, topology
  revisions and tick-based duration. Route queries remain pure; the existing
  `test_des_boundary_route_rendering_does_not_schedule_events` guards this.
- ABM `geometry.rs` and `spatial_index.rs`: geometry/interpolation and updates;
  reuse qualified `petgraph`, `geo` and `rstar` integrations.
- DES `boundary.rs`: generic handover/transit, acknowledgement, cancellation and
  expiry. A boundary handover is not automatically a continuous spatial journey.
- DES `staged_runtime.rs`: scoped actor/resource/request handles, scheduler-backed
  FIFO contention, atomic quantities, leases and forward-only usage windows.

## Separate contracts

| Contract | Required semantics | Excluded domain meaning |
| --- | --- | --- |
| Location/route | Optional location; coordinate reference/units; stable topology/revision; access policy input; path and tie rule | Hospital site assumptions, clinical mobility rules, economic value |
| Movement | Operation ID; actor; origin/destination; departure/arrival ticks; checked duration; causal event; state and cancellation | Visual animation as engine state; universal travel-time law |
| Allocation | Resource kind/ID; request/lease IDs; quantity; admission/release; priority/queue policy; atomicity and preemption | Prices, QALYs, payer perspective |
| Usage | Run/scenario/branch/event IDs; actor/resource/lease; instant or interval; quantity/unit; cause and revision | Cost or utility inferred from a resource name |
| Economic valuation | Explicit mapping of usage and health outcomes to model-specific value | Scheduler/ECS implementation or replacement RNG |

## Movement lifecycle

The public API lives in `kairo_ecs_abm::movement`. A model owns one
`MovementRuntime` alongside its `World` and `Scheduler`. It registers a live,
generational actor's location with `place` and looks it up with `location`.
`schedule` takes a `MovementRequest` with actor, destination, departure tick,
access profile and scheduler priority. It returns a `MovementPlan` containing
an opaque runtime-scoped `MovementId`, revision, origin, destination, departure
and arrival ticks, the route and the exact scheduler event ID. `pending` reads
the current plan for a handle.

Scheduling allows one pending move per actor. A missing placement, retired
actor, past departure, unavailable route, non-tick cost or tick overflow
returns a typed `MovementError` before enqueueing. Schedule at checked
departure plus route duration, including a zero-duration route. Even then the
arrival goes through the queue. The scheduler's priority and insertion order
determine same-tick ordering. Route queries and display interpolation are pure.

Pass the native `DispatchedEvent` to `on_arrival` before scheduling more work
for that actor. Only the matching pending arrival can complete the move.
`MovementArrivalOutcome` is `Arrived`, `ActorRetired` or `TopologyChanged`.
Location changes only for `Arrived`. Repeated, superseded and unrelated events
return no outcome. Actor retirement or topology invalidation consumes the
arrival without changing a live actor's location.

`cancel` takes the movement handle and cancels its exact pending scheduler event.
An already finished or cancelled handle is an idempotent false. A foreign or
unissued handle, or an event already removed from the scheduler but awaiting
`on_arrival`, returns an error. Use `forget_actor` when retiring an actor before
arrival to close its movement and remove placement. Reused entity indices have
different generations and receive fresh placements and handles.

After a topology edit, call `replan` for each affected pending handle.
It uses the recorded access profile and the actor's last location and starts
at the later of the original departure and the current scheduler tick. It
validates the replacement before cancelling the old arrival, then enqueues it
with the original priority, the same handle and the next revision. The same
topology revision returns the existing plan. A topology replacement also
invalidates the route. An unavailable replacement leaves the old event in
place for explicit cancellation or a later `TopologyChanged` outcome. A model
that changes an actor's access cancels and reschedules with the new profile.
Continuous in-transit positions remain a model concern.

For model-owned checkpoints, clone the runtime with its corresponding world
and native scheduler checkpoint. Keep handles and emitted outcomes within
that replay branch.

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
references. Optimistic execution exports from a checkpoint at the runtime's
exclusive committed cutoff. Define late/corrected records and reject
duplicates with conflicting content. Keep Arrow/Parquet an optional encoding
of the same semantic records.

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
