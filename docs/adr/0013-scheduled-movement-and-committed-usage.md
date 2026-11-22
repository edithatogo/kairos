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
