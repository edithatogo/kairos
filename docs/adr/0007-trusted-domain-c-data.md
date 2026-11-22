# ADR7: Trusted owned C Data for declared native domain tables

## Status

Accepted for bounded local alpha prototype after independent safety review.
Hosted/package/parent adoption and stable release qualification remain open.

## Context

The event-log-only high-level interface cannot export CareOps patient,
calibration or theatre tables. Serializing Parquet/IPC is not direct C Data
interop. Raw ABI imports intrinsically require trusted producer invariants.

## Decision

Add declared-schema primitive domain exports to kairo-ecs-arrow, retaining its
unsafe-free boundary and pinned Polars Arrow types. Validate exact schema and
required-null constraints before creating populated owned arrays. Private opaque
export holders are constructible only by these validated safe producers. A
consuming into_raw handoff supports ordinary external C Data consumers, with
standard unsafe foreign-consumer obligations.

The existing kairo-ecs-ffi unsafe boundary offers safe consumption only of those
opaque trusted exports, returning owned schema/StructArray or a bounded-batch
iterator. Arbitrary caller-forged raw structs cannot enter that safe path.
No new query engine, storage backend, extension module or generic domain model.

## Consequences

Rust preview API is additive; fixed event-log C symbols/headers/schema and host
bindings are unchanged. Domain owners define versioned names, units, limbs,
paired-ID nulls and their own scientific meaning. Conversion follows the pinned
CompatLevel::oldest mapping; String may allocate, primitive buffers may share.
No universal zero-copy claim. Stream windows are produced on demand; the source
frame/chunks remain owned until released. A consumer that collects every batch
still allocates for the collection. Release/lifetime/negative controls and actual
parent consumer proof are required before promotion.
