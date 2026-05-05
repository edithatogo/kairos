# Event Ordering

DES event dispatch is deterministic when events are applied in total order:

1. `timestamp_ns`
2. `entity_id`
3. `delta`

The CPU fallback parity harness uses this total order.

GPU kernels may batch commutative effects with atomics when all events in the batch commute. Non-commutative mutations must be split into deterministic passes or rejected by the backend until a deterministic ordering strategy is available.

Any benchmark using atomic dispatch must report that it validates aggregate state, not per-event mutation order.
