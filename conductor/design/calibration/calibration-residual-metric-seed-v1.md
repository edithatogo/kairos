# Calibration residual, metric, and seed-purpose contract v1

Date: 2026-10-01. Status: C0.2 proposal for coordinator and owner review; no
runtime behavior, seed algorithm, statistical validity, or C0.2 acceptance is
established. This proposal consumes the [trace/time/mapping contract](c0.2-trace-contract.md),
[provenance and censoring contract](provenance-censoring-v1.md),
[fidelity and observed-ledger contract](calibration-fidelity-ledger-v1.md), and
[adapter proposal](calibration-adapter-v1.md). Parent record names and fields
refer to the parent `calibration-v1` schema and require a separate coordinator
join.

## Residual record

`calibration_residual.v1` is one study/candidate/replication/run/case/task/
occurrence/endpoint result, keyed by its versioned dataset, scenario, run and
mapping provenance. Its required identity/provenance includes `study_id`,
`candidate_id`, `replication_id`, `seed_schedule_id`, `seed_purpose`,
`seed_map_ref`, and `seed_contract_version`. Candidate ID remains experiment
provenance; it is not seed material. The record also carries fidelity, observed
and predicted ticks, residual status/sign/magnitude, the unclamped-prediction
assertion, anchor role, feasibility, censor status, parameter and optional graph
hashes, and nullable causal reference. `seed_map_ref` resolves the versioned
seed-purpose record, including `coupling_mode` and `candidate_binding`; do not
infer those choices from candidate IDs. Tick values use the trace contract's
unsigned nanosecond tick representation.

Define signed residual as:

```text
residual = predicted_ticks - observed_ticks
```

A positive residual means the predicted endpoint is later/longer than the
observed endpoint; a negative residual means earlier/shorter. When both ticks
are valid unsigned values, compare them first, subtract the smaller from the
larger, and store `residual_sign` as `positive`, `negative`, or `zero` and the
nonnegative difference as `residual_magnitude`. This produces the exact absolute
difference across the full u128 range without a signed-i128 intermediate or
wraparound. For computed residuals both ticks and the magnitude are required.

`predicted_ticks` records the point prediction before any downstream clamp;
`prediction_unclamped` is always `true`. Applying an observed clamp cannot
rewrite this value or turn a missing, late, failed, or infeasible probe into a
successful prediction. Do not compute a numeric residual unless both prediction
and observed endpoint are valid point ticks.

Use the schema's status vocabulary exactly:

| `residual_status` | Required meaning |
|---|---|
| `computed` | A valid unclamped prediction and an observed point endpoint exist; sign and magnitude are computed as above. |
| `missing_observed` | Required observed endpoint is missing, not known to be censored; `observed_ticks=null`, `residual_sign=undefined`, and `residual_magnitude=null`. |
| `censored` | The endpoint is left-, right-, or interval-censored under the outcome contract; no point residual is emitted. For the schema's missing/censored branch, `observed_ticks=null`, sign is `undefined`, and magnitude is null. An unknown or missing observation without an established censor bound uses `missing_observed`. |
| `probe_failed` | The probe failed before producing a valid endpoint prediction; preserve failure evidence and any valid observed tick, but emit no numeric residual. |
| `infeasible` | The evaluation completed with an explicit infeasibility result; preserve its feasibility/provenance status and emit no numeric residual. |

For every non-computed status, set `residual_sign=undefined` and
`residual_magnitude=null`; retain a valid `predicted_ticks` only when the
unclamped point prediction itself exists. Missing, censored, failed, infeasible,
late, or budget-exhausted work remains represented and counted; never replace
it with zero or silently drop it. A late but valid point prediction may still
have a computed residual; preserve its feasibility/deadline evidence rather
than clamping the predicted tick. If budget exhaustion prevents a valid point
prediction, retain `probe_failed` and the detailed reason in probe/run evidence.
`anchor_role` is one of
`observed_source_transition`, `none`, or `unknown`; `feasibility` is
`feasible`, `infeasible`, or `unknown`. Preserve the separate censor status and
all causal/seed/mapping/parameter provenance. An anchor is admissible only when
it is a source-defined observed transition available by that anchor time.

## Aggregate metric record and counts

`calibration_metric.v1` summarizes one named metric, algorithm version, endpoint,
stratum set, analysis window and units. Its provenance includes dataset, run,
mapping version, `seed_schedule_id`, and `seed_map_ref`; it also carries nullable
seed-contract version and parameter hash. Both seed-reference keys are required
by schema. When seeded simulation observations or attempts contribute, they
identify the schedule and seed map; null is reserved for a metric with no
seeded simulation provenance. The allowed metric IDs are `W1`, `KS_D`, `paired_residual_summary`, or a
versioned `other` method. Each metric record binds one simulation cohort to one
`seed_schedule_id` and `seed_map_ref`; separate records are required for
independent candidate schedules. Put candidate/arm grouping in `strata` when
needed to avoid mixing candidate populations. C0.3 owns objective weights,
tolerances, selection thresholds and inference validity; this record does not
choose any of them.

Counts are per input record instance on either the reference or simulation side,
within the named endpoint/strata/window. A case present on both sides is two
side-specific input instances for counting. `reference_count` and
`simulation_count` count valid point observations actually used in the two
empirical distributions, one per record instance, with equal empirical weight.
The other fields are independent diagnostic counts, not additive bins:

- `excluded_count`: records suppressed by a declared, versioned eligibility or
  window rule.
- `censored_count`: outcomes with a supported left-, right-, or interval-censor
  state and no observed point event time.
- `missing_count`: records without the required point endpoint for reasons other
  than an established left/right/interval censor state; missing/unknown evidence
  remains visible here.
- `unmatched_count`: records for which a declared paired estimand requires a
  counterpart but no valid match exists. W1 and KS D compare unpaired
  distributions and do not require one-to-one case matches, so lack of a pair
  alone does not increment this count for those metrics.
- `failed_count`: attempted simulation/probe records that failed or exhausted
  their execution budget before a valid result was produced; keep the specific
  reason in the linked probe/run evidence.
- `infeasible_count`: completed simulation evaluations explicitly marked
  infeasible under the declared feasibility rule. If a valid point prediction
  also exists, its inclusion in a particular metric follows that metric's
  versioned eligibility rule; the infeasibility count remains visible.

Diagnostic counts may overlap one another or the contributing sample counts
when the dimensions differ (for example, a declared exclusion may also be
censored, or a valid point prediction may be infeasible for a separate deadline
constraint). Do not sum them to infer a total denominator or silently discard
one condition. Keep each field's definition stable within the algorithm version
and report all applicable counts even when no metric value can be computed.

`status` is `computed` only when each required metric population contains valid
values and all mapping/provenance preconditions are satisfied. Use `empty` when
both populations contain zero valid values; `insufficient_data` when a required
population is empty; `invalid` for contradictory schema/time/stratum inputs; and
`unverified` when source, mapping, or provenance evidence cannot support the
comparison. For every non-computed status, `value` is null. Do not manufacture a
zero metric for empty or insufficient inputs. `uncertainty` is null unless a
separately named, versioned method and its assumptions are recorded; a KS D
value does not imply a classical KS p-value. Clustered or tied observations do
not become independent through aggregation.

## Deterministic W1 and KS D semantics

Both metrics use the eligible point observations in the declared endpoint,
strata, window and unit. Each valid observation has equal empirical weight;
there are no implicit weights, interpolation, or imputation. Values must use
compatible units from the same declared tick scale or a separately versioned
exact conversion. Stable-sort each side by ascending numeric value, then by the
stable logical key from the owning record schema for any record-level
presentation; never use file, worker, or completion order as a tie-breaker. Before updating an
empirical CDF, aggregate every observation with exactly equal numeric support.
This tie grouping is mandatory: input order within a tie cannot affect either
metric. Reductions follow the same ascending support order regardless of file,
chunk, worker, or completion order.

Let the distinct values in the union of both supports be
`x_1 < ... < x_k`. Let `R_i` and `S_i` be the reference and simulation counts
with value less than or equal to `x_i`, and let `n_R` and `n_S` be the valid
reference and simulation counts.

For one-dimensional empirical Wasserstein-1 (`W1`):

```text
W1 = sum(i = 1..k-1) (x_(i+1) - x_i)
     * abs(R_i / n_R - S_i / n_S)
```

The CDF gap is constant between adjacent distinct supports, and equal-valued
observations are accumulated before the gap is evaluated. W1 has the same units
as the support values.

For the two-sample Kolmogorov-Smirnov distance (`KS_D`):

```text
KS_D = max(i = 1..k) abs(R_i / n_R - S_i / n_S)
```

Evaluate the right-continuous empirical CDFs at each distinct support in their
union, after all tied mass at that value is accumulated. KS D is dimensionless
and lies in `[0, 1]`. Do not evaluate an arbitrary order among equal values or
use a left-limit at a tie as a separate candidate maximum.

Where the rational comparisons are made, compare integer cross-products rather
than rounded CDF floats. W1 sums exact support gaps weighted by the exact CDF
gap; implementations must use checked arithmetic that cannot wrap at u128 ticks
or large counts. Any final conversion to the schema's numeric `value` is applied
once, after the deterministic reduction, under the versioned algorithm
implementation. `algorithm_version` identifies the tie, reduction, arithmetic,
and output-conversion rules. This contract sets no tolerances or acceptance
thresholds.

## Versioned logical seed purpose and candidate coupling

The logical identity fields for `kairoecs.seed-purpose.v1` are exactly:

```text
(study_id, seed_schedule_id, replication_id, case_key, task_key, purpose)
```

`candidate_id` is experiment provenance and is deliberately excluded from this
tuple. For paired common-random-number comparisons, candidates reuse the same
`seed_schedule_id` and set `coupling_mode` to `paired_common_random_numbers` with
`candidate_binding` set to `shared_across_candidates`. For independent candidate runs, use a candidate-scoped `seed_schedule_id`, set
`coupling_mode` to `independent`, and set `candidate_binding` to
`candidate_scoped`. This pairing intent is explicit and
auditable; a shared schedule ID does not itself prove that runs were paired
correctly or that model inputs are otherwise equivalent. For a paired residual
summary, match rows on `(study_id, dataset_id, scenario_id, seed_schedule_id,
replication_id, case_key, task_key, occurrence, endpoint, seed_purpose,
seed_map_ref, mapping_version)`; exclude `candidate_id` and `run_id` from this
logical pairing key. A missing
required counterpart increments `unmatched_count`. This pairing rule does not
change W1 or KS D, which compare unpaired distributions.

The v1 purpose set is exactly `service`, `transit`, `behavior`, and
`calibration`. Each purpose has a separate logical stream and draw position, so
transit changes cannot shift service draws, and one purpose cannot consume
another purpose's sequence. The purpose vocabulary is fixed for v1; adding or
renaming a purpose requires a reviewed contract/schema version change. Repeated work is keyed by its logical identity, not
worker, thread, wall clock, or file order. `case_key` is the pseudonymous
trace-contract key; never use a direct patient identifier or hash raw patient
strings into seeds.

This contract freezes only the logical fields, purpose vocabulary and pairing
mode. It sets `contract_version` to `kairoecs.seed-purpose.v1`,
`approval_status` and `encoding_status` to `pending_track01`, and
`algorithm_id` to null. Stable ID normalization, root-seed treatment, tuple byte
encoding, derivation/hash algorithm, stream version, draw-position and
checkpoint rules remain pending exact Track 01 owner approval. The existing
`derive_entity_seed(run_seed, EntityId)` API does not approve this tuple or its
algorithm. No calibration execution may claim this contract is executable
before that owner gate passes.

## Review boundary

Track 21 owns residual semantics, metric definitions and evaluation validity;
Track 01 owns seed identity implementation and derivation; Track 04 reviews the
physical Arrow representation; Track 22 binds the schedule/candidate pairing to
its study manifest; the domain adapter owns source-specific endpoint mapping.
The accepted fidelity contract keeps observed data immutable and probes
isolated. These documents remain proposals until independently reviewed and
joined by the coordinator. No local ED source, statistical validity, runtime
calibration, or seed-algorithm acceptance is inferred.
