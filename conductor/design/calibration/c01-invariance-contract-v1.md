# C-01 exact invariance qualification v1

Reviewed by root coordinator 5 October 2026 for the explicit user C-01 request.
Scope is C1.4.evidence/C-01 only; neither C1.4 nor release/clinical acceptance.
Owners 21 (private Rust semantics), 04/12 (actual physical conformance). No new
public API, dependency, physical schema, scheduler or existing runtime change.
Baseline Kairos 18ee41e42efc4bd5d6def0e8199d9958a6d4f32c; parent2675da5.

## Oracle frozen before execution

Normalized populations are events.ndjson, quarantine.ndjson, exclusions.ndjson,
and outcomes.ndjson, compact serde_json BTree JSON plus LF, serialization
serde-json-btree-ndjson-v1. Exact bytes, parsed ordered records, rows, bytes and
SHA-256 must equal each profile's baseline, not merely equal another run.
Independent Python checks C0 schemas, candidate conservation and physical rows.
Hash equality does not substitute for byte equality. Input file/physical file
hashes, diagnostic source positions, execution evidence, chunk/run limits and
spill/pass counters legitimately vary and are not canonical record hashes.
Keep diagnostics and semantic reason/count summaries visible; do not omit
excluded, quarantined or censored populations to manufacture equality.

## Fixture profiles and capture interface

New private cfg(test) ingestion_c01 module calls the existing ingest function.
Three declared profiles: long-valid (existing C13 7 rows, 7 candidates, 6 events,
1 exclusion, 2 outcomes); wide-valid (same two complete cases plus one all-missing
case, 3 raw rows, 9 candidates, 6 events, 3 exclusions, 2 outcomes);
long-quarantine (change Z bed_entered clock from1s to8s, keep departure4s; policy
Quarantine, 7 source/candidates, 3 events, 3 quarantined, 1 exclusion, 2 outcomes).
Original clock lineage, source precision, immutable origin and stable event
keys/source ordinals remain fixed. Wide bindings use separate declared
{kind}_at, {kind}_id and {kind}_seq fields, with no inference.
Profile baselines are actual pipeline outputs, not copied hand-authored goldens.

KAIROS_C01_CAPTURE_DIR selects a new directory containing profile subdirectories,
each with template.json, source.json (raw array), manifest.json, all four
normalized NDJSON populations, diagnostics.ndjson, config.json (plain evidence
for the frozen validation policy) and baseline receipt. Max100 raw rows/1MiB
raw JSON; profile/window/identity/state bounds stay fixed. Capture must fail on
existing files/unsupported profile rather than silently reuse a prior result.

## Rust normalization matrix

Unignored self-contained test covers each profile, forward/reversed raw rows,
normalization max_chunk_rows {1,2,64}, max_run_rows {1,2,64}, max_run_bytes
{4096,1048576}, merge_fan_in2; other limits remain fixed. This is108 runs.
Each point compares exact bytes and semantic accounting to its baseline. Require
row-triggered spills and byte-triggered spills at row64/bytes4096 (runs>1,
merge_passes>0), and single-run control at row64/bytes1048576 for every profile.
If a fixture cannot fit4096, fail qualification and return revise; no weakening.

Ignored env-only actual_transport_matrix requires KAIROS_C01_TRANSPORT_INDEX
and KAIROS_C01_RESULT_DIR. Index JSON lists every actual reader-derived raw
source array path/hash, profile and physical axes. It must cover3 profiles ×
8 physical layouts ×9 Rust writer format/limit variants =216 input bundles.
Do not sort or reorder these raw arrays: source keys/ordinals stay stable while
physical order remains the actual reader order. For each bundle execute the
18 chunk/row/byte settings above, yielding3888 actual pipeline runs. Emit
every point's canonical hashes/counts, exact-byte comparison, semantic counts,
source fingerprint and observed spill/pass counters. Reject absent/empty,
duplicate, missing or unsupported matrix points; no skipped-only green.
Retain representative populations plus every manifest/receipt; no target caches.

## Actual physical dimensions

New optional conformance/c01/physical_dimensions.py uses existing pinned
PyArrow25 codec and unchanged Rust ingestion_physical_v1 adapter. For each
profile encode normalized trace events plus quarantine as one trace_event.v1
table, exclusions and outcomes as their own physical-v2 tables. Expand
batch_rows {1,2} × row_group_rows {1,3} × row_order {forward,reverse}:
8 independently varied layouts/profile (24 actual adapter invocations).
Preserve exact schema/metadata, source units/nulls and typed fields.
Parquet UNCOMPRESSED is explicit. Record actual IPC batch and Parquet row-group
boundaries and ordered decoded row hashes, proving variations really occurred.

For each layout verify all27 Rust outputs in exact adapter order (input reversed)
against its expected populations, independently declared schema and metadata.
Reconstruct source from each event+exclusion pair for limits{1,2,3} and formats
{ipc_file,ipc_stream,parquet}:216 source bundles total, no selected first-file
shortcut. Deduplicate wide expansion by exact(case,canonical raw source row)
only, preserve first-seen physical order, validate each event key and ordinal
against its declared binding, and compare raw-source multiset to baseline.
Each reconstructed array must retain its actual order, not sort by source_order.
The index records profile, batch, rowgroup, order, format, writerlimit, path/SHA
and rows; top level has capture_dir and schema_version c01.transport-index.v1.

## Negative oracles and acceptance

Rust negative: mutate a schema-valid raw source marker field. Ingestion must
succeed while exact population equality to baseline fails and event SHA changes.
Python negative: mutate one valid physical output payload under the frozen
schema without altering count. Ordered/payload verification must reject and
produce no index/source output; original control remains valid. Retain failures.
All population counts and policy summaries must reconcile; independent C0
validation runs on all3 actual baselines. Qualify Rust1.88 self-contained matrix
and current1.99 actual matrix. No assertion about hostile decoder peak RSS,
unseen schemas, real EHRs, clinical validity, release or full C1.4 acceptance.
