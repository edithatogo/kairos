# API Review: metadata-aware native publication

## Affected Surface

- Exact root: crates/kairo-ecs-arrow.
- Family/stage: additive Rust API, local alpha preview.
- Proposed: metadata-aware Parquet/Zstd and IPC stream helpers, plus RunOutput
  write_batch_with_metadata/write_batch_to_with_metadata. Input is a deterministic
  string BTreeMap; physical schema metadata replacement is rejected.
- Existing APIs, Arrow fields, C symbols/headers, host roots, event/replay/domain
  behavior, dependency manifests and generic run-manifest shape stay unchanged.

## Compatibility and migration

Additive only. Existing calls delegate with empty metadata, verified byte-equal.
New consumers opt into explicit provenance; metadata content/semantics and domain
manifest binding remain caller-owned. No stable/beta or publication claim.
ADR8 records the original publisher path and why a sink rewrite is invalid.
No package/catalog root change or new generic backend. Existing runtime/FFI
source receipts remain independent; no new pin is automatically promoted.

## Required evidence and objections

- Real populated Parquet/Zstd/IPC data plus custom schema metadata, typed nulls,
  empty frames and independent PyArrow readback.
- Native durable receipt hashes the exact bytes containing metadata; canonical
  ordering/parts/watermarks and manifest-last validation survive.
- Failed output records no receipt. Reserved ARROW:schema rejects before write.
- Caller may lie in arbitrary metadata: helpers preserve declarations, not truth;
  authoritative consumer manifests/column checks reject inconsistent provenance.
- Metadata can vary per part, so no speculative generic cross-part schema layer.
- C Data buffer ownership and thread-budget behavior are not changed by this API.

## Decision

Needs prototype, actual focused receipts and independent source review. Retain
full hosted/package/current-source parent gates before integrated acceptance.

## Local prototype disposition

Independent source review is ready with no actionable findings. Actual100native
Arrow tests pass,0ignored;9specific controls include exact empty-metadata byte
compatibility, typed empty tables, reserved/oversized/empty-call rejection,
bounded actual writer failure and metadata-bearing durable receipts. Independent
PyArrow26 verifies actual Parquet footer/IPC schema metadata and data/part hashes.
All-target warnings-denied Clippy/docs and fmt pass. The checked64KiB metadata
budget is distinct from unchanged upstream row bounds and caller byte budgets.
Accept local alpha engineering; retain actual hosted/package/parent gates and
caller responsibility for truthful domain/source declarations.
