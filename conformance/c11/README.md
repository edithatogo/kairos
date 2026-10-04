# C1.1 synthetic logical oracles v1

## Scope and accounting

The corpus separates synthetic source rows, expected C0 logical records, and companion counts. `source_rows` is the physical input-row count; `candidate_units` is the logical candidate count; accepted, excluded, failed, and unresolved units partition that count. These companion diagnostics are not a `dataset_manifest`, and no final manifest is emitted.

The checker pins the accepted schema digest and `jsonschema==4.26.0`, applies an explicit RFC3339 checker, validates UTC/tick equality and u128 bounds using integer-only arithmetic, checks raw retention/case coverage/count conservation, and runs fixture and comparator tamper probes. Optional `--actual PATH` compares a future consumer's exact per-case classification, logical records, exclusion reason, accounting, and semantic `oracle` diagnostics plus companion counts. This generic JSON comparison shape is not a Rust API. The default says `consumer_status=not_executed`; no mapper runs.

## Review index (all listed families are under 24 KB serialized individually)

- Shape/clock/units/offset: wide-long equivalence, distinct clocks, ns/us/ms/s, a common-instant unit-equivalence control, explicit offset.
- Precision/timezone: minute-only/date-only profile exclusions, separate pre-resolved-minute helper boundary, preclassified missing-zone/fold/gap.
- Invalid time/identity: sub-nanosecond, pre-origin, relative-u128 overflow raw text; duplicate/missing identity dataset failures without invented exclusion keys.
- Ordering/interval/event semantics: source-backed same-time rank/order, reversed/open intervals, distinct episode end, boarding, and later physical departure.
- Quality/outcomes/knowledge: missing triage denominator, right censoring, cutoff-equal/future/unknown availability.
- Source counterexamples: FHIR `meta.lastUpdated` is a resource update time, not event-occurrence or source-recorded time; HL7 MSH-7 is not occurrence; A08 alone is not physical movement; OMOP visit end without ETL lineage is not observed departure.

## Limits

The synthetic profile excludes minute-only and date-only point candidates as coarse while preserving raw precision. A separate pre-resolved-minute case documents only that the pure helper receives an already-resolved instant and does not parse or exclude source precision. DST cases are preclassified; no IANA resolver or TZDB claim is made. Standards-labelled examples are semantic counterexamples, not full parsers, local mappings, or conformance claims. Arrow physical schemas, bidirectional external-reader parity, production normalization, and full C1.1 acceptance remain separate gates.

Additional integrity bindings: wide and long raw event fields (key, kind, occurrence, rank, source order) must map to expected records; stable-order raw rank/order and right-censor raw risk interval are bound. The relative overflow literal is explicitly decimal nanoseconds from the pinned origin and must exceed u128 max. Minute exclusion preserves its declared precision in raw fields. Offset hours/minutes are range-checked independently of Python parsing. `tick_resolution` is optional; raw, UTC, integer ticks, precision, and lineage are the required time-value evidence.
