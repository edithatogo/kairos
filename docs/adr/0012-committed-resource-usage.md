# ADR 0012 Committed staged-resource occupancy

The staged FIFO scheduler is the direct source for the first neutral occupancy
export for Dapane. This work covers admission, release, explicit quantities,
window integration and replay in the existing forward-only staged runtime.
Movement and optimistic export retain their existing track gates.

- The existing unit request delegates to an atomic quantity-one request.
  A larger request waits at the head until its whole quantity is free.
  One accepted release returns the units held by that lease.
- The lease records actual admission and release events. At a same-tick handoff
  the releasing event can admit the next requests. Their stable keys incorporate
  run, scenario, branch, resource, request and lease revision. Slice keys add
  both window bounds. The runtime binds
  model, engine, RNG and tick scale when usage export is selected.
- The export clips occupancy to a half-open historical window. An actual
  release past the endpoint stays absent in that window. It includes only
  occupancy, with an exact checked total of resource unit-ticks. A resource's
  physical quantity unit is fixed at creation. Downstream models supply prices
  and utilities.
- Export requires a reached endpoint and all scheduled actions through it.
  A successful export seals that endpoint against new same-tick submissions
  and releases. Queued, rejected and pending actions contribute only when
  admitted. Deterministic reruns preserve the exported IDs and values.
- A Time Warp adapter would need a committed cutoff and a checkpoint-aware
  source. That adapter remains outside this exported API.

The conformance cases in the staged DES test suite cover strict FIFO with
multi-unit requests, same-tick admissions, duplicate release, sealed endpoints,
censoring, split-window conservation, replay and arithmetic overflow.
Existing unit-quantity clients retain the original methods and fields.
Review scenario IDs must distinguish genuinely different branch executions,
and comparison callers must avoid summing overlapping windows.
