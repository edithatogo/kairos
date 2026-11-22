# Composing movement with staged allocation

A model associates its `MovementId` with the `StagedRequestId` that supplies
the journey's lease. It owns the decision to request resources, choose a route,
and keep or relinquish a lease across replanning.

Use one model clock. Process allocation events due through the current tick
using `StagedResourceRuntime::next_event_time` and `run_for`. Inspect the
request state and start movement only after admission. When the model
dispatches an arrival, cancels a movement, retires the actor, abandons an
unavailable replacement route or fails to schedule a move, call
`close_request` at that tick and drain the resulting allocation actions.
Successful replanning keeps the same movement handle and lease. Each
scheduler retains its own priority and insertion order.

`close_request` can also withdraw a future submission or a queued request.
Withdrawal produces no occupancy for a request that never held capacity.
Removing a FIFO head allows the next compatible request to be considered.
Closing an active request returns its entire admitted quantity and records
the actual release tick once. A duplicate close at the same or a later tick
is a no-op. A close at an earlier available tick supersedes a pending later
release.

The Rust fixture covers successful arrival, cancellation, a route that cannot
be replaced and failure to start. It measures the resulting occupancy and
checks that capacity is available again.
