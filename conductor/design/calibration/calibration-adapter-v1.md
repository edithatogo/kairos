# Calibration adapter and runner proposal v1

Date: 2026-10-01. Status: accepted local C0.1 architecture/interface proposal;
not an implemented or published Rust API. Owner 21 defines the semantic protocol,
22 resolves/executes studies, 03 supplies engine hooks, and CareOps supplies ED
policies. [Placement ADR](ADR-0005-calibration-placement.md) controls dependencies;
[VVUQ contract](../../contracts/vvuq-contract.md) controls legacy runner semantics.

## Versioned model adapter hooks

`calibration_adapter.v1` is the proposed protocol ID. Names and semantic obligations
below are selected for later implementation. Concrete Rust DTOs, wire schemas and
checkpoint encoding are frozen in C0.2 with their owners, not inferred from these
names. Use associated model scenario/parameter/context/probe/error types so the
library does not depend on CareOps. Each evaluation owns a fresh isolated world;
no ambient singleton, wall-clock seed or shared mutable prediction state.

| Hook (proposed Rust method) | Input -> result | Obligation |
| --- | --- | --- |
| `validate` | scenario, parameters, mapping, fidelity, seed map -> checked compatibility or typed error | Verify adapter/model/config/schema versions, bounds, units, required graph/hash and permitted anchors before mutation |
| `map_event` | normalized event and explicit mapping version -> mapped model event or unmapped/ambiguous reason | Preserve source identity/occurrence and observed/derived/unknown lineage; never infer occupancy from assignment |
| `checkpoint_context` | admitted authoritative anchor context -> immutable context snapshot | Capture only information available by the anchor; include assignment/resource/route revisions and active/suspended work; no future observations |
| `macro_inputs` | immutable context, checked parameters, logical seed map -> declared macro inputs | Preserve target/input roles and conditioning; prohibit reading held-out targets as generated input |
| `start_probe` | immutable context, target ID, fidelity, checked parameters, seed map -> isolated probe state | Clone/isolate required context; disclose assumptions about missing actor/state; cannot mutate observed ledger |
| `advance_probe` | exclusive probe state and fixed tick/event budget -> pending/completed/failed outcome | Target detection is model-defined and registered; enforce exact scheduler ticks/order and report unreachable/missing/budget failures |
| `checkpoint_probe` | probe state -> versioned model checkpoint payload | Preserve active/suspended progress, RNG state, target identity and overdue status; no raw pointer persistence |
| `restore_probe` | model checkpoint payload plus checked provenance -> exclusive probe state or incompatibility | Restore exactly or reject; no silently restarted probes, double endpoints or new seed identities |

Result envelopes include logical run/candidate/replication/target identity,
checked integer predicted ticks when available, failure/censoring/feasibility
reason and assumption/provenance references. Missing prediction is not zero.
Signed residual is predicted minus observed, computed with checked sign/magnitude
handling over u128 ticks; no lossy float controls scheduling. C0.2 owns encoding.

The authoritative observed ledger never borrows probe mutation state. A late
probe may continue in isolation while the authoritative ledger proceeds at its
observed anchors; lateness must remain a residual/feasibility result, not be
clamped away. Models cannot detect targets using future labels unavailable at
an anchor. Patient/task/calendar rules stay in domain adapters; generic engine
hooks do not prescribe clinical behavior.

## Track 22 orchestration contract

Proposed opt-in flag: `--calibration-config <path>` on a future extended replay
path; proposed new `calibrate` and `compare-runs` operations. These do not exist
at the audited base. Preserve existing flags `--scenario`, `--seed-manifest`,
`--output` and legacy no-opt-in replay behavior. Do not reinterpret seed streams
or overloaded flat scenario sections through the existing parser.

The separate document uses proposed version `kairoecs.calibration-config.v1`.
It references/hashes the legacy scenario and seed manifest, model/adapter
version, normalized trace/mapping, parameters, fidelity/anchors, objective,
split and candidate/budget manifests, graph if required, output policy and
seed-map version. C0.2/C0.3 freeze serialization and accepted values. The new
loader must reject absent required references, digest mismatch, unknown versions,
unsupported capability and model/schema incompatibility before any execution.
Do not treat legacy acceptance as evidence that a calibration config was read.

Orchestration order: validate references and split integrity; enumerate stable
candidate IDs; bind replication and logical seed identities; allocate isolated
evaluations; gather every success/failure; complete the fixed comparison batch;
reduce in canonical candidate/replication/target order with frozen tolerances;
rank with stable candidate-ID ties; persist full history and final disposition.
Failure/missing/late outcomes cannot improve an objective through silent omission.
Fixed penalties/thresholds are C0.3 validity decisions, not selected here.

Track 21 owns search/evaluation/objective semantics. Track 22 owns scheduling,
manifest handling, durable run inventory, file routing and resume reconciliation.
Arrow adapters serialize ordinary semantic records and do not run the study.
The runner consumes the shared parameter catalogue by stable IDs/config keys;
it must not maintain a competing ED parameter table or invent values for gaps.

## Resume, seeds and compatibility

Completed evaluation keys are `(study, candidate, replication, adapter/model,
seed-map version)` with bound input/config hashes; exact serialization is C0.2.
Partial evaluations retain model checkpoint plus RNG/probe/ledger progress,
completed target IDs and failure state. Resume rejects provenance mismatch and
re-evaluates only explicitly unfinished work; duplicate keys/targets fail.
Worker finish order cannot change seed identities, budgets, reductions or ranking.
No claim follows from the current CLI checkpoint/request scaffold.

Use the existing core/RNG ownership. Entity-derived streams are implemented, but
logical case/task/purpose/replication derivation is still a C0.2/01 contract.
Do not hash patient strings or thread IDs into ad hoc seeds. Paired Macro/Micro
service draws remain stable under transit changes through the later approved map.

## Handoff and acceptance boundaries

C0.1 selects responsibility and interface boundaries and repairs the missing
Track 22 reference. C0.2 freezes schemas/time/mapping/seed contracts; C0.3 freezes
fixture/objective/split/tolerance/dependency options; C0.4 reviews the whole phase.
D2 gates implementation, and downstream C1–C6 require their own accepted runtime
fixtures. Public API introduction needs 25 review; no owner sign-off or runtime
capability is inferred from this local proposal. No renderer, Python runtime,
GPU/PDES transport, real EHR import or second experiment framework is introduced.
