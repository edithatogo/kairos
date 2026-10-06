# C1 synthetic source mapping profile v1

Coordinator proposal for a private Rust conformance adapter; no stable public API, local feed or standards conformance claim. Physical output follows C1 physical schema v1. The adapter is not an external sorter, streaming ingestion service or full C1.3 pipeline.

## Input envelope

Required: profile_version=c11.synthetic-map-v1, dataset_id, mapping_version, origin_utc, case_key_field, source_family, shape (wide/long), ordered event_bindings, rows. Nonempty identifiers retain exact UTF-8 spelling; no trim, normalization or case folding. Blank/whitespace-only identifiers reject. Each binding explicitly specifies kind, canonical decimal rank, occurrence index (UInt32), occurrence_field, key_field and order_field; optional recorded_field, message_field, knowledge_field. No field-name inference or role fallback. Unknown columns are retained as raw source evidence; duplicate column names must be rejected by the physical reader before a JSON object loses them. Event bindings have unique kinds/ranks and field mappings, fixed list order; no hash-map iteration defines expansion order.

Wide rows expand declared event bindings; a null required occurrence produces an exclusion candidate, not a skipped event. Long rows are complete event candidates, with an explicit event_kind field choosing the binding; field/value transport grouping is an upstream adapter with required case/source-row identities and duplicate field rejection. Thus this profile does not mislabel arbitrary field/value rows as complete events. Raw source row count and expanded candidate count are distinct.

## Clock inputs

Each mapped clock is an object: raw string or null, representation (RFC3339/relative_integer/classified), declared precision, lineage object. RFC3339 requires explicit numeric offset or Z, strict Gregorian calendar date and explicit known offset (-00:00 is unknown and rejected). More than nine fractional digits are accepted only when every excess digit is zero, without rounding; nonzero excess precision rejects. Leap seconds, naive local, date-only and unsupported minute raw point input reject. Integer clocks declare unit s/ms/us/ns and canonical signed decimal source value (negative relative time produces pre_origin); checked multiplication and origin addition never round or saturate. Unit changes do not replace declared raw precision. Raw clock values/offset/zone and lineage remain in output.

A separate resolved-minute helper control may accept Minute precision; it cannot be used to admit raw minute input through the strict source profile. Classified fold/gap/unresolved inputs are trusted synthetic classifier controls with explicit classification provenance. They only prove excluded-result preservation, never IANA timezone resolution. A future raw named-zone resolver must pin its library/TZDB and policy separately.

Origin is a single explicit UTC instant for the dataset. Clocks normalize to exact signed i128 UTC nanoseconds and nonnegative u128 ticks. These integer codecs are distinct from the calendar codec, whose supported UTC range is Gregorian years 0001 through 9999. After checked unit multiplication and origin addition, the adapter independently checks final delta fits u128 and final UTC instant is calendar-representable; an out-of-calendar instant yields overflow exclusion retaining raw value/unit/origin. It never clamps or fabricates RFC3339 UTC. Pure helper/physical byte boundary tests may exercise full i128/u128 ranges without claiming those values have a calendar text representation. Optional clock absence stays null, with explicit unknown lineage. Optional clock failure excludes that event candidate rather than borrowing occurrence time.

## Identity and failure precedence

Envelope/profile errors fail the dataset first; next validate all identities (missing/blank keys before duplicate keys), rank map and source-order types; then normalize clocks. Validation collects class failures deterministically; source row order must not pick a winner. Missing/duplicate source_event_key fails the whole dataset, with zero accepted/excluded records and failed_units=candidate_units. Never fabricate a source_event_key from row number. Identical external encounter IDs are allowed when event keys are distinct; case key is not a seed.

## Semantic and knowledge guards

Explicit forbidden synthetic role transforms: meta.lastUpdated cannot establish source-recorded event time; MSH-7 cannot establish event occurrence; ADT A08 alone cannot establish physical movement; OMOP visit-end cannot establish observed physical departure without separately evidenced lineage. Return unverified mapping status and preserve raw evidence; these are counterexamples, not standard parsers. Administrative episode_end and later physical_departure are separate kinds; valid boarding after episode_end is retained.

Known features require known status and independently sourced available_at <= prediction cutoff; equality is eligible. Future or unknown availability remains ineligible. Outcomes never become inputs by default. Interval reversed/open checks use existing temporal helper; no capacity validator or imputed endpoints are introduced. Missing triage is explicit missingness and cohort denominator remains cohort count, not triage count.

## Output and proof

Return full C0 trace_event/exclusion/outcome records and a result partition with source_rows,candidate_units,accepted_units,excluded_units,failed_units,unresolved_units; integers represented as canonical decimal where no bound exists. Per-result candidate conservation required. Censored outcome counts are a separate observation population, not an extra event partition. Normalized records preserve raw rows and mapped clock lineage.

Raw fixture inputs contain the envelope and source rows only; expected records are separate reviewed goldens. Rust executes the adapter without reading goldens. Physical encoder consumes actual returned records; independent reader checks exact schema, fields, metadata, nulls and bytes. Negative controls mutate raw inputs and verify changed outputs or declared failures. No copied expected projection may be presented as actual mapper output.

## ADR and compatibility disposition

Coordinator accepted private experimental fixture adapter direction after independent objections review on 2026-10-04. No public Rust symbol, stable API/ABI or engine-core dependency change is approved. Calibration stays Rust1.88; chrono0.4.45 (std only; clock/IANA defaults disabled) and serde_json1.0.149 (arbitrary_precision) reuse exact locked versions; kairo-ecs-arrow supplies existing role-preserving helpers. The JSON fixture protocol is versioned independently; incompatible changes require a new profile. Red-team objections about calendar range, arbitrary integer precision, role fallback, input-vs-golden circularity and classifier claims are addressed above. Full C1, external sorting, hosted/release acceptance and actual IANA resolver remain separate gates.
