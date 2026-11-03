# ADR8: Attach native provenance before durable part attestation

## Status

Accepted local alpha addition after independent source review and actual native/
independent-reader receipts. Hosted/package/parent promotion remain open.

## Context

B7's qualified RunOutput publisher hashes exact encoded Parquet bytes. Its
existing API cannot carry caller provenance metadata. A sink that changes bytes
after hashing invalidates receipts; duplicating publication would add a backend.

## Decision

Add deterministic metadata-aware Polars Parquet/Zstd and IPC helpers and opt-in
RunOutput batch/sink methods. Plain methods use empty metadata. Shared encoding
attaches metadata before file write and receipt creation. Preserve all existing
filesystem, sequencing, acknowledgment and failure semantics. Reject reserved
ARROW:schema replacement before emitting bytes. No new dependencies or schemas.

## Consequences

Additive Rust preview API, unchanged existing C/host interfaces and manifest
shape. The caller binds its declared metadata to domain manifests and actual
columns; generic publication does not certify scientific or source truth.
Metadata hashes attest actual files, not cross-version byte reproducibility.
Existing consumers can keep empty metadata. CareOps clean v2 outputs will require
and verify their versioned provenance; no legacy fallback is introduced there.
