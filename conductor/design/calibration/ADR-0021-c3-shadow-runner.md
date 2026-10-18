# ADR 0021: private C3 shadow runner and isolated probe protocol

Date: 2026-10-10. Status: coordinator decision for bounded implementation;
independent implementation review and C-03 qualification remain required.
Owners: 21 semantics, 03 native execution, 22 recovery, 04 existing sidecars.
Parent C1.4, C2.4 and Q4.5 are accepted; C2 source is `63a72aa563258777686ee3d2e25794628daae4ec`.

## Decision

Implement private calibration modules under `shadow`. No public API or schema
change is required. Use Rust 1.99.0 for every new gate. Reuse C1 trace order,
the accepted seed keys, C2 full runtime images and C4 signed residuals/sidecars.
The observed ledger, probe pool and output projection have separate writers.
Coordinator owns module wiring, shared types, the model integration and joins.

The ledger owns validated normalized events. Source event keys are unique;
distinct occurrences remain distinct. Input order is canonicalized with the C1
six-field comparator. Repeated identical source keys are errors, never implicit
deduplication. Resource claims have explicit IDs and units; release must name
an existing claim. Impossible occupancy produces explicit diagnostics and
strict resource replay rejects it without changing the committed frontier.
Diagnostic replay retains the historical event and reports infeasibility.
The ledger constructor receives caller-declared capacities and initial active
claim IDs/units. Units are counts of that resource's capacity, never time or
distance. Unknown resources and zero-unit claims are invalid. Snapshots preserve
capacity and individual claims, not just aggregate occupancy. Initial occupancy
is a declared assumption; its provenance is supplied in the snapshot assumptions.

Each step applies one observed transition at its original tick, then captures
the post-anchor snapshot. A snapshot contains only the visible historical
prefix, occupancy derived from available observations, the frontier, tick,
digest and explicit assumptions. An event with unknown/future knowledge time
cannot anchor a probe. Unavailable resource observations cannot contribute
hidden occupancy to a probe; their absence must be an explicit assumption.
At equal ticks, normalized source order still distinguishes snapshot frontiers.

The adapter receives an owned historical snapshot and `ProbeInput`, which has
target identity but NO observed target tick or future observed events. The
runner owns the target observation separately. Each adapter start/restore must
construct an exclusively owned runtime; immutable code/config may be shared,
mutable runtime, queue, context and RNG may not. This is a trusted model-adapter
contract, not a sandbox against malicious model code.

The adapter reports current time, next dispatch tick, one native dispatch step,
and first target detection. C3 enforces inclusive absolute tick horizon and
total event budget BEFORE dispatch. Completion at the horizon or on the final
allowed dispatch counts as completed. Empty queue without endpoint is missing;
Every native dispatch, including non-target events, consumes one event. Admission
target detection consumes zero. Horizon is an absolute endpoint tick and admission
rejects a horizon earlier than the snapshot. `ProbeStep.dispatched_at` must match
the prior `next_tick` and runtime time; first target must equal that dispatch tick.
pending beyond tick/event limit is censored; native error is failed. Adapter
time reversal, incorrect dispatch time or a target outside the dispatched
prefix is a contract error. Native fixtures use actual Flow dispatch order;
arithmetic toy runtimes alone do not establish C-03.

Probe IDs are caller-registered stable logical IDs, unique within the run.
Each binds trusted seed key, adapter/parameter hashes, target, anchor, snapshot
hash/frontier and budgets. Admission never exposes observed target values to
the adapter. Probe advancement cannot mutate the observed ledger. A late probe
continues in its own virtual time even after the observed target has passed.
Results are retained exactly once in canonical probe-ID order; repeated poll
of a terminal probe does not rerun or append a duplicate endpoint.

Predicted minus observed uses existing sign plus u128 magnitude. Missing,
censored and failed endpoints have no numeric prediction/residual. A valid
late point remains a numeric prediction with a separate infeasibility flag.
Unknown observed target remains missing comparison, even with a prediction.
Strict candidate policy uses a prespecified rational maximum late fraction,
with checked integer arithmetic, and separately counts missing/censored/failed
outcomes. Diagnostic policy retains every disposition. Neither changes anchors,
predictions or denominators. Resource-invalid diagnostic rows remain visible.
Strict policy rejects any missing observation/prediction, censoring, failure or
resource-infeasible result. Its late fraction denominator is paired point results;
empty evaluations reject. Validate numerator <= denominator and denominator > 0.
Diagnostic evaluation retains all rows. Resource-infeasible snapshots produce an
explicit Infeasible terminal result with no point prediction. A late valid point
has an infeasibility flag and a numeric residual, as permitted by C4. Replay role
is carried as the explicit `replay_role=ShadowAnchored` group stratum in existing
C4 sidecars, alongside existing anchor-role and feasibility fields.

Each probe spec carries existing C4 LogicalKey provenance plus run/candidate,
source anchor and target event keys. Its opaque seed key is from the accepted C2
seed registry, never derived by this runner. The trusted run binding is SHA-256
over current run/config/schema/input identities selected by caller code; restore
also compares every saved spec AND historical snapshot with the trusted inventory.
Per-runtime adapters see only ProbeInput and snapshot. Run-level checkpoint DTOs
freeze the pending byte image versus terminal outcome distinction and consumed
budgets. They are private intermediate records; portable wire encoding follows
in its own leaf and cannot be inferred from these Rust types.

## Recovery and qualification

The C3 checkpoint records ledger frontier, trusted run binding, all admitted
probe IDs/spec bindings, immutable snapshots, consumed budgets, pending native
images and terminal outcomes. Restore verifies the complete inventory and
provenance before exposing any runtime. It must reject duplicate IDs, changed
budgets/config/seeds/snapshots and inconsistent terminal/pending states.
Restore is staged; failures do not mutate a live destination. C2 images restore
individual native probe worlds. This does not implement C5 candidate scheduling.
Portable framing is bounded before allocation and identities are compared with
trusted current configuration; hashes detect corruption, not authenticity.

Required C-03 evidence includes early/exact/late points, exact fixed anchors,
missing target, impossible occupancy, distinct occurrences and duplicate keys,
interrupted native transit/work, inclusive limits, isolated historical states,
outstanding/overdue probe fresh-process recovery without duplicate outputs,
actual residual sidecars and the walk+work=5 ambiguity/independent-walk fixture.
The latter is a synthetic identifiability demonstration, not clinical inference.
Each leaf returns source/command/toolchain/input/output evidence; coordinator
reviews and qualifies the integrated tree before closing C3.1-C3.4.
