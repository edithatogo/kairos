# Calibration provenance, censoring, and knowledge contract v1

**Status:** C0.2 proposal for coordinator review. It consumes the accepted
trace/time/mapping contract. It specifies semantics; it does not implement Rust
or Arrow readers, establish a local data source, prove source conformance, or
close C0.2.

**Ownership:** Track 21 owns normalized outcome/parameter semantics and
validation. Track 04 owns Arrow physical encoding. Track 01 owns core identity,
ordering, and seed derivation. Track 22 owns study orchestration and durable
artifacts. The domain adapter owns source-specific interpretation, dictionaries,
privacy controls, and evidence of local mapping.

## Provenance and lineage

Every outcome, parameter, and time value retains its own lineage. Use the
lineage statuses exactly as defined by the trace contract:

| Status | Meaning |
|---|---|
| `observed` | Present in an identified source record, subject only to documented parsing and normalization. |
| `derived` | Computed from identified inputs by a named, versioned transformation. |
| `defaulted` | Supplied by an explicit, versioned fallback because the source did not provide the value; retain the missing-source reason. |
| `unknown` | Origin or transformation cannot be established from the available evidence. |

The v1 lineage object carries `status`, `mapping_version`, nullable
`evidence_ref`, and nullable `derivation`. A digest establishes byte identity,
not semantic correctness, completeness, or source validity. Do not promote
unknown provenance to observed. Missing values remain null with an explicit
status/reason; they are never imputed to zero, endpoint success, task
completion, or realized disposition.

Keep event occurrence, source-recorded time, message-created time, and
knowledge-availability time distinct, each with its own raw representation,
precision, clock role, and lineage. A resource update time cannot replace any
of those clocks. Preserve source precision and raw values during normalization;
do not silently round, truncate, or infer a more precise instant. A derived
interval or endpoint carries lineage for the derivation and references to its
source event(s).

## Outcome observation and censoring

`outcome_observation.v1` is one case/endpoint record, separate from raw trace
events and predictions. Here `event_observed` means the exact endpoint event
instant is observed, not merely that evidence suggests an event occurred within
an interval. Its v1 fields are `dataset_id`, `case_key`, `endpoint`,
`risk_start`, `last_observed`, `event_observed`, `event_time`, `event_cause`,
`censor_status`, `censor_reason`, `cluster_ids`, and `lineage`, with the schema
version and record type. A null `risk_start` or `last_observed` means the bound
is unavailable; it is not a zero-duration bound.

Allowed `censor_status` values and meanings:

| Status | Meaning and required treatment |
|---|---|
| `not_censored` | The endpoint event is observed; `event_observed=true` and `event_time` is present. |
| `right` | No event is observed through `last_observed`; require that time, set `event_observed=false`, and leave `event_time=null`. |
| `left` | The event is known to have occurred by the first observation, but its instant is not observed. Keep `event_time=null`; retain justified bounds and their source evidence separately. |
| `interval` | The event is known only within a supported interval. Keep `event_time=null`; do not substitute an invented point time. Preserve interval bounds and lineage in source/mapping evidence because v1 has no bound fields. |
| `unknown` | Evidence cannot establish event state or a valid censor boundary. Do not infer event time or last observation. |
| `missing` | Required endpoint evidence is absent. This does not mean event absent, event present, or successful follow-up. |

An observed event requires `event_time` and `censor_status=not_censored`. For
`right`, `unknown`, and `missing`, `event_observed` must be false and
`event_time` null. Contradictory combinations are invalid; reject or quarantine
with a reason rather than silently repairing them. Left/interval records do
not have a point `event_time` unless the instant itself is observed under the
endpoint definition. Preserve any supported bounds in the referenced evidence
and do not imply that the present scalar schema stores them.

Compute at-risk duration only from valid ordered bounds under the declared
dataset origin. A missing risk start, reversed interval, unresolved endpoint
mapping, or invalid time conversion makes the observation unusable for that
estimand; it does not produce zero duration. Keep eligible, observed, censored,
missing, excluded, unmatched, failed, and infeasible counts visible in each
aggregate/stratum where applicable. Every exclusion has a versioned reason.
Complete-case results disclose their denominators and exclusions; omitted
outcomes cannot improve a score by disappearing.

## Knowledge time and prevention of future-outcome leakage

The trace `knowledge_availability` status (`known`, `not_yet_known`, `unknown`)
and nullable `available_at` timestamp describe when an event/record became
available to the modeled decision process. Parameter evidence separately uses
`time_of_knowledge` and `availability_status` (`available`,
`not_yet_available`, `unknown`, `not_applicable`). These fields serve different
record types and must not be conflated. When knowledge time is unknown, retain
that status; do not derive it from event time, row order, or file arrival.

At a prediction or anchor time, a value is an eligible feature only when its
availability by that time is evidenced. A value with unknown availability is
not eligible as a known-at-prediction input. It may remain a retrospective
target in the immutable observed ledger if declared as such. Later outcomes,
revised records, and held-out labels cannot enter the prediction context.
Record revisions do not rewrite the historical knowledge state; preserve the
applicable source/mapping revision and availability evidence.

The observed ledger is immutable. Predictions and probe state live in separate
records with explicit causal/anchor references; they cannot append, backfill,
or revise observations. An anchor is admissible only if its transition is
source-defined and was available by the anchor time. Unknown availability or
lineage stays explicit and cannot be resolved by choosing the most favorable
interpretation.

## Parameter evidence and roles

The v1 `parameter_evidence.v1` record identifies dataset, case, stable
`parameter_id`, role, value, units, time of knowledge, availability status,
cluster IDs and a `cluster_status` (`identified`, `unknown`, or
`not_applicable`), lineage, mapping version, and nullable source reference. An
empty cluster array is only meaningful together with that status; it does not
by itself assert independence. Assign
exactly one role per parameter in a versioned study definition:

| Role | Meaning |
|---|---|
| `exogenous` | External input that may condition a model only when available to the modeled process at the relevant decision time. |
| `primitive` | Directly measured or primitive quantity supplied to the model, with units and source lineage. This role alone does not make it a calibration target. |
| `clamp_only` | Observed value permitted only to constrain a declared transition; it cannot silently become a generated input, covariate, or target. |
| `target` | Outcome used for evaluation/calibration. It is not available to prediction generation unless an independently evidenced earlier value is recorded as a separate feature. |

Keep role, units, availability, mapping, and lineage together. `time_of_knowledge`
and `availability_status` are related but distinct: preserve a known future
availability time even when the value was not available at the earlier decision
cutoff. A missing value
does not become a default without an explicit versioned rule and `defaulted`
lineage. v1 does not define parameter-bound fields; do not imply that bounds
are serialized or invent domain ranges. Any future bounds require a reviewed
versioned schema extension. A role change requires a new mapping/config version.

`cluster_ids` are zero or more stable identifiers for sampling/dependence units
in their declared namespace/version. Pseudonymize subject-derived identifiers
under an evidenced rule; never put direct identifiers in portable artifacts.
Missing cluster membership stays explicit. Do not infer independence from an
empty list, use cluster labels as RNG seed material, or split dependent units
across partitions without an explicitly reviewed design. The statistical
method and weights remain a separate objective-validity decision.

## Report 30 and source evidence boundary

Report 30 references `C0-F001` through `C0-F011` are provisional candidates,
not canonical case, event, or parameter IDs. The downloadable detailed
crosswalk and local source-feed evidence were absent from the reviewed inputs.
Therefore local availability and local conformance remain `unknown` for every
provisional reference. Standard-level semantic descriptions such as `exact` or
`partial` against a named edition do not establish that a local field exists,
is populated, or is correctly transformed. Do not invent source rows, counts,
evidence references, or local mappings from provisional IDs.

Keep these assertions separate in any eventual mapping bundle:

1. semantic strength against a named standard edition (`exact`, `partial`,
   `unverified`, or `unavailable`);
2. local source availability (`available`, `unavailable`, or `unknown`), backed
   by inspected source evidence;
3. local transformation/conformance, backed by a separately inspected and
   validated extract.

Until the local source and crosswalk are supplied and reviewed, preserve
`unknown` for availability/conformance. Public standard documentation cannot
establish local availability.

## Residual and aggregate accounting

A numeric residual requires both a valid prediction and an observed endpoint.
Missing or censored outcomes have no numeric residual; keep their status-bearing
row or an equivalent explicit accounting record rather than dropping them.
Store the predicted value before any clamp. A clamp cannot turn missing,
infeasible, or late work into success. Represent sign and unsigned magnitude
separately to avoid overflow across the full unsigned tick range.

Aggregates retain reference/simulation denominators and counts for censored,
missing, excluded, unmatched, failed, and infeasible records. Empty
or insufficient inputs have an explicit status and no fabricated numeric
value. C0.3 owns objective weights, tolerances, and inference-validity rules.

## Handoff and review boundary

This contract consumes
[`c0.2-trace-contract.md`](c0.2-trace-contract.md) for event/time/knowledge
semantics and the adapter proposal's immutable-ledger and leakage boundaries.
The parent schema's `outcome_observation.v1` and `parameter_evidence.v1` define
the logical record fields; this document explains their evidence meaning and
limits. Track 04 still reviews physical Arrow representation, and the parent
coordinator must separately accept cross-record validation and schema
integration. No runtime, local source, standards conformance, or C0.2 acceptance
is asserted here.
