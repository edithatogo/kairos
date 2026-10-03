# Calibration fidelity and observed-ledger contract v1

**Status:** C0.2 fidelity proposal for coordinator and owner review. The
trace/time/mapping and provenance/censoring leaves are consumed as accepted
contracts. This file defines no Rust API or runtime capability and does not
claim C0.2 acceptance.

**Scope:** Track 21 owns calibration semantics and probes; Track 03 supplies
engine execution hooks; Track 22 owns orchestration and resume; Track 01 owns
shared identity/order and seed derivation; Track 04 owns Arrow encoding. Domain
policy remains in the adapter. The parent contract/schema inputs were bound at
SHA-256 `26157fa0e379bed2492e3f1229119cbf02934b6e15da6bb13c129eba3f460c48`
and `b573fc9dffe776b002ae50dc4eb0cc7406dcc6f7bfad56a16e60d4ad170c48f5`.

## Fidelity decision at task admission

Resolve one policy when each task is admitted, using the first configured value
in this precedence order:

| Rank | Scope | Meaning |
|---:|---|---|
| 0 | `entity_subsystem` | Override for this entity and subsystem pair. |
| 1 | `entity` | Entity-specific override. |
| 2 | `subsystem` | Subsystem default. |
| 3 | `global` | Global default. |

The admitted `fidelity_decision.v1` records the task, selected policy, winning
scope and matching rank, admission time, mapping version, resolver version,
and `effective_for_task=true`. Schema validation requires the rank to match
the scope. The decision is immutable for the task lifetime, including while it
is suspended. An unresolved policy is an admission error; no implicit value is
chosen. This contract selects no ED policy or global default.

Recognized policies are `ShadowAnchored` and `FreeRunning`. In
`ShadowAnchored`, only explicit, source-defined observed transitions available
by the anchor time can anchor a probe. Model-inferred and future observations
are never promoted to anchors. In `FreeRunning`, the probe advances from its
declared initial snapshot without injecting later observed transitions. Both
policies use isolated mutable probe state and preserve the same input and seed
provenance.

## Policy updates preserve admitted work

Represent a requested change as `fidelity_policy_update.v1` with scope,
new policy, resolver version, request time, status, quiescent-boundary
reference, and the task keys to which it applies. `staged` records have no
effective task keys. An update becomes effective only when status is `applied`
and a named quiescent boundary is recorded; it applies to later admissions,
never by reinterpreting an existing task. `rejected` updates have no effective
task keys. Schema validation enforces these record relationships.

A boundary is quiescent for the affected scope only when there are no active
or suspended tasks and no outstanding probe work that depends on the old
policy/context. Do not cancel, drop, rewrite, or silently restart work to force
quiescence. If quiescence cannot be established, keep the request staged or
reject it. A task's effective policy cannot change in place. Preserve the old
decision, update record, boundary identity and later task decisions so resume
cannot silently apply a different policy history.

## Immutable observed ledger and isolated probes

The authoritative observed ledger contains source trace events and
source-supported anchors. Preserve the trace contract's stable event identity,
occurrence, raw source event, normalized kind/time, mapping version and
per-value lineage. Keep `provisional`, `realized`, and `unknown` disposition
distinct. Keep knowledge availability distinct from occurrence time; unknown
or not-yet-known information is not a feature available at an earlier anchor.
Later record revisions do not rewrite the historical knowledge state.

Start each `calibration_probe.v1` from an immutable context snapshot. The
record binds the run/case/task, selected fidelity, observed-ledger frontier,
snapshot hash and start time. It also requires explicit references for:

- `anchor_event_ref` and `anchor_role`, which identify a source-defined
  observed transition or explicitly identify no anchor;
- `seed_map_ref`, which identifies the separately versioned logical seed map;
- `parameter_hash` and `adapter_hash`, which bind the parameter inputs and
  model adapter used for the probe; and
- `observed_ledger_mutation=false`, which records the required isolation
  invariant.

The probe owns its mutable world, event queue, task progress and random-stream
positions. It may read only its admitted snapshot and declared inputs. Its
predictions, inferred events, clamps, checkpoints and failures live in
prediction/probe records; they never append to, backfill, reorder or revise the
observed ledger. A probe cannot inspect another probe's mutable state or draw
positions. Observed replay may continue while a probe is running, and probe
completion order cannot alter observed data or another probe's inputs.

The seed-map reference does not approve a derivation algorithm. Before any
calibration execution, Track 01 must accept the exact stable logical identity,
purpose separation, encoding/derivation and draw-position rules referenced by
that map. Until then, a probe may be specified but cannot be treated as
executable evidence. Never derive seeds from direct patient identifiers,
wall-clock time, worker order or thread IDs. The adapter reference likewise
binds a versioned adapter/configuration and its digest; a mismatch on resume
must fail closed rather than silently restart under new inputs.

## Clamp, prediction, residual and incomplete work

An observed clamp is an input/anchor permitted by the declared fidelity and
source evidence. It remains an observed record with its own lineage. A model
prediction is a separate output. Capture predicted ticks before any downstream
clamp; applying an observed value cannot rewrite the prediction or convert it
into an observed outcome. Residual sign and unsigned magnitude are computed
from the unclamped prediction and observed endpoint only when both are valid.

Late, censored, missing, unreachable, failed, infeasible, or budget-exhausted
probe outcomes remain status-bearing records and counts. They are not numeric
zero, successful completion, or grounds to discard already admitted work. A
resume restores the same probe identity, context frontier, seed map, adapter,
parameters and progress, or rejects incompatible provenance. It never
silently allocates a replacement probe or duplicates a target result.

## Required later evidence

Conformance fixtures and runtime tests must establish that:

1. Every decision follows the declared precedence and its recorded scope/rank
   agrees; absent policy fails admission.
2. Updates cannot change active or suspended tasks, staged/rejected updates do
   not apply, and applied updates require an identified quiescent boundary.
3. Policy changes preserve all work and take effect only for later admissions.
4. Ledger content/frontier is unchanged by probe count, completion order,
   failure, clamp, checkpoint or resume.
5. No probe reads future knowledge, another probe's mutable state, or a
   different seed/adapter/parameter identity.
6. An observed clamp remains separate from the predicted value and residual;
   the residual uses the unclamped prediction.
7. `ShadowAnchored` accepts only source-defined anchors; `FreeRunning` injects
   no later observation; incomplete and censored results remain explicit.

These are test obligations, not test results from this documentation packet.
Track 12 owns portable fixtures; Track 21 and 03 own semantic/runtime review;
Track 01 must approve seed semantics; Track 04 must verify any physical Arrow
encoding. Local source availability, clinical validity and operational
acceptance remain separate evidence gates.
