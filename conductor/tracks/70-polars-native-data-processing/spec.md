# Polars-native data processing and standard interchange

## Overview

Minimize maintained generic data machinery through library replacement. Companion: CareOps `polars_data_processing_20261022`; downstream starts after Phase 4 acceptance.

## Inputs and outputs

Inputs: current source, accepted simulation contracts, representative synthetic runs, existing fixtures and pinned toolchain. Outputs: versioned native tables, independent interoperability/replay evidence, deleted bespoke code and exact commit handoff.

## Ownership and dependencies

Dependencies: 04, 62, 63.
Owned implementation surfaces (prospective, require per-packet claims): crates/kairo-ecs-arrow/, crates/kairo-ecs-ffi/, bindings/, schemas/arrow/. Coordinator owns dependency manifests and consumer/CI/docs updates only with existing owner handoff. Blocked paths: scheduler/RNG/clinical semantics, unrelated tracks and active leases. Parallel-safe work: disjoint read-only review; shared manifest/FFI/registry writers serialize.

Maturity: planned experimental alpha replacement; no implementation qualified.

## Decisions and boundaries

Authority: user-approved conceptual plan and 22 October 2026 request to register it. Type: refactor. Both repositories; Kairos first, CareOps immediately after accepted upstream delivery. Clean alpha API/format break, no legacy readers or compatibility wrappers. Preserve historical files; regenerate only fixtures owned by this migration. Planning is not implementation, release or clinical acceptance.

Polars Rust is the standard tabular processing implementation, not a wrapper around retained bespoke algorithms. Default persisted tables are real Parquet/Zstd; Arrow IPC is interchange; CSV is explicit export. Small JSON configuration/provenance/checkpoint-control documents remain Serde-owned. No DataFusion dependency, backend abstraction, custom query language, cloud service, GPU requirement or new distributed runtime. Reconsider DataFusion only on a measured unsupported workload, not merely SQL or multi-file requirements.

Retain scheduler, ECS identity/lifecycle, resource arbitration, RNG, clinical policy, online counters, numerical solvers and checkpoint control semantics. Audit scripts for duplicated tabular work, but do not indiscriminately replace JSON/configuration or governance utilities. No new clinical outcomes, inferred spatial movements or invented data may be introduced to fill missing exports.

## Data and interface contract

Keep kairo-ecs-arrow as the generic boundary; expose upstream Polars DataFrame/LazyFrame in native data APIs, without a generic wrapper engine. Use Polars Arrow facilities first and direct Apache Arrow only for demonstrably necessary ABI/interchange gaps. Do not assume polars-arrow and arrow-rs have interchangeable Rust types or compatible versions. Prove the actual conversion and ownership path. Isolate Polars in native data consumers, absent from minimal core and browser/Wasm dependency graphs; native data CLIs enable it by default.

Replace alpha tabular schemas with explicit v2 identifiers. Generational IDs use UInt64 index plus UInt32 generation, both null together for nullable IDs. Simulation ticks use UInt64 high/low limbs, lexicographically ordered numerically; preserve full u128 range. Checked limb subtraction handles borrow before conversion to analytical units; do not subtract rounded floating-point endpoints. Centralize helpers, not a new arithmetic library. Required fields, units, nullability and canonical sort keys are defined once. Unknown versions, overflow and malformed IDs reject explicitly. CSV flattens limbs and never uses floating-point IDs/ticks.

Use library writers and populated Arrow arrays/streams. Exported immutable buffers outlive engine mutation/destruction; release exactly once, reject unsupported schema negotiation, and test partial construction failure and multiple batches. Remove descriptor-only/fabricated capsule fallback. Document copies and allocations; no universal zero-copy claim. Update affected C declarations and all reachable bindings; keep non-Python consumers on Arrow interchange. Python receives native tables without a Python-dictionary reconstruction detour.

Run manifests include run/replication identity, actual effective-input hashes (not scenario name hashes), schema/library/source versions, seeds, units, table paths, row counts, hashes and completion status. Preserve raw-input and normalized-effective-input hashes separately where both exist. File hashes attest files, not cross-version byte reproducibility. Explicit deterministic row ordering and logical equality establish data reproducibility.

## Statistical and execution contract

Exact parity for event sequence, RNG streams, IDs, ticks and counts. Derived finite floating-point results use abs(error) <= 1e-12 + 1e-10 * abs(reference), with reviewed per-metric justification for any exception. Preserve weighting, denominator, censoring, ddof, interval method, empty and singleton semantics; null/NaN/infinity equivalence is explicit, never hidden by tolerance. Integer sums must not wrap. Independent analytical oracles prevent perpetuating known defects: isolate intentional corrections in a reviewed task rather than silently updating golden outputs.

Validate join keys/cardinality before joins; missing references follow an explicit existing domain policy or fail, never silently drop/multiply patients. Trace capture occurs once during the original run. Write bounded batches/per-replication parts; post-process lazily after workers finish by default. No unbounded queue and no silent trace drops. Backpressure/failure changes wall-clock progress only, never virtual-time decisions or random streams. Configure native thread budget once before Polars pool initialization; never mutate global environment concurrently. Test multiple worker counts, skew, high cardinality, and disk exhaustion; streaming is not proof every operation is memory bounded.

Use unique run output directories, staged parts, final renames and manifest-last publication. Readers require a completed manifest and valid hashes. Do not overwrite completed runs. Persist checkpoint-to-trace partition boundary (replication ID, part ID and sequence watermark) so resume neither loses nor duplicates records. Test process interruption before/after part and manifest publication. Parquet telemetry alone is not a simulation checkpoint. Rollback is code/pin rollback with a fresh output directory; old output is not silently reinterpreted.

## Acceptance and release implications

Success means deleting generic bespoke machinery, not accumulating adapters. Record before/after non-generated maintained code, duplicate routines, dependency/build/binary costs and retained exceptions; do not game line counts or count generated dependencies as owned code. Preserve meaningful behavioural tests while removing tests of obsolete encodings. Require independently read Parquet/IPC, populated FFI, exact replay, statistical oracles, restart recovery and bounded-memory evidence. No fresh performance/zero-copy claim without raw receipts.

Record command, cwd, commit, toolchain, exact dependency/features, seed/input hashes, exit status and artifact paths. Local tests, hosted CI, integration, release and clinical acceptance remain distinct. No purchase, publication, protected-setting changes or deployment is authorized. This technical repository plan is not a structured business document; SharePoint document generation is not required.

## References (recheck at dependency qualification)

- https://docs.pola.rs/user-guide/misc/arrow/
- https://docs.pola.rs/user-guide/concepts/streaming/
- https://docs.pola.rs/user-guide/expressions/aggregation/
- https://datafusion.apache.org/library-user-guide/index.html

## Refinements from planning review

- Cross-repository order is serial: Kairos accepted commit -> CareOps receipt verification -> pin and data-layer adoption -> consumer acceptance. No historical track is reopened by this registration.
- Preserve schema/run identity and units in file metadata as well as the manifest. For CSV use the manifest as the typed metadata carrier. Assert metadata survives actual independent-reader roundtrips.
- Freeze actual Rust/Python version compatibility, row-index width, chunk sizing and thread budget during qualification; version names from web prose are not Cargo pins. Fail explicit size limits rather than truncate row indexes. Benchmark allocations and zero-copy claims separately from query speed.
- Output publication is a local-filesystem contract in this migration. Validate path containment and collision handling; do not assume filesystem rename semantics for object stores. Flush/sync data and directories where supported before completed-manifest publication; document platform crash guarantees.
- Capture pre-migration reference traces before deleting old implementations. Baselines are evidence, not permanent fallback code. Known methodological defects receive explicit review, not automatic golden-file acceptance.
