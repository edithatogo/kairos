# Composing movement with staged allocation

A model associates its `MovementId` with the `StagedRequestId` that supplies
the journey's lease. It owns the decision to request resources, choose a route,
and keep or relinquish a lease across replanning.

Use `StagedResourceRuntime::step_with_scheduler` to dispatch one live event
through an inclusive tick. The allocator takes precedence at a tie, and each
scheduler keeps its priority and insertion order. Only the selected scheduler
advances. Use the returned tick for reactions. Inspect the request state
after allocation and start movement on admission. Use a model event at the
reaction tick for handlers that read the model scheduler's clock, such as
replanning. Pass the returned arrival event to the movement handler.

When an arrival ends the journey, the model cancels a movement, the actor
retires, no replacement route is available, or a move fails to start, call
`close_request` at the current event tick. The next step considers that
close alongside the remaining model events. Successful replanning keeps the
same movement handle and lease. Each call can count toward a shared event
budget. An eligible event earlier than either scheduler's last dispatch is
rejected. For allocator-only dispatch, use `run_until_or_for`.

`close_request` can also withdraw a future submission or a queued request.
Withdrawal produces no occupancy for a request that never held capacity.
Removing a FIFO head allows the next compatible request to be considered.
Closing an active request returns its entire admitted quantity and records
the actual release tick once. A duplicate close at the same or a later tick
is a no-op. A close at an earlier available tick supersedes a pending later
release.

The Rust fixture checks a lease admission that starts movement, allocator
requests before and at the arrival, and a lease release before the next
same-tick model event. It also covers cancellation, replanning, actor
retirement and failure to start, with occupancy and capacity assertions.

## Dispatching through the model's own runner

Use `StagedResourceRuntime::peek_with_scheduler` when the model runs its own
event handlers during dispatch. The preview applies the same clock, limit and
tie rules and leaves both queues ready to dispatch. For
`StagedDispatchPreview::Allocation(at)`, run the allocator through `at` with
an event budget of one. For `StagedDispatchPreview::Model`, let the model
dispatch its next event, for example with `BehaviorSimulation::run_for(1)`.
Apply the reaction at that dispatch tick, then preview again.

The [ABM behavior fixture](../../crates/kairo-ecs-abm/tests/abm_behavior_update_v1.rs)
uses this sequence to execute an agent update and close its lease at the
same tick. A preview describes the current queues. Preview again after
intervening scheduling, cancellation or dispatch.
