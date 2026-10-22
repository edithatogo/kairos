# sha2 robustness

Planned; no runtime acceptance. Apply [value policy](../../dependency-value-policy.md) and [sequencing](../../dependency-sequencing.md).

## Inputs

Prerequisites: none beyond fresh source/owner/value qualification. Read-only cross-track inventory is allowed before prerequisite acceptance; replacement is not.

## Outputs

Explicit adopt/defer/reject decision, qualified bounded replacement and immutable consumer receipts.

## Owned paths

Lead: arrow-agent. Prospective target: kairo-ecs-arrow digest code and retained checkpoint/provenance consumers; CareOps separately owns api_runner.rs compute_sha256_hex. Planning owns this track folder. Runtime paths, shared manifests and fixtures require current owner handoff and a narrow claim; no blanket ownership transfer.

## Blocked paths

Other repositories, active reservations and unrelated runtime semantics. No implicit CI/publishing permission.

## Dependencies

Registry dependencies []. No dependency on completed Polars for Tracks 93/94/96: this prevents a migration cycle. Owner-coordinated source inventory decides deleted versus retained targets.

## Contract

Replace sha256_simple (which is not SHA-256) only where retained contracts require SHA-256. Delete the obsolete pseudo-Parquet codec through Track 70 rather than repairing it into a parallel format. Qualify standard SHA-256 primitives and retained consumers before Track 70; do not require completed Polars outputs for this track to close. Version any changed digest/format contract; preserve historical artifacts and reject incompatible formats explicitly. Keep noncryptographic identity hashes and RNG stream assignments unchanged. CareOps already declares sha2 but still maintains handwritten SHA-256: package presence is not migration.

## Acceptance criteria

Independent standard empty/abc/multiblock/boundary-length vectors; Python hashlib or another independent implementation; single-bit corruption, truncated/invalid digest, full-byte coverage including high tick bits; old-format rejection/version tests. Do not treat the old custom hash as a SHA-256 oracle.

Record exact features/version, maintenance/security/licence/MSRV/target qualification and wrapper-inclusive code/cost budgets before adoption. No net benefit means defer/reject, not implemented.

## CareOps handoff

Accepted immutable Kairos receipt before consuming upstream API changes. Direct CareOps implementations/tests remain parent-owned in the paired sha2/tempfile/thiserror/proptest tracks. Optional libraries need a bounded parent packet and concrete consumer; no duplicate direct dependency solely because upstream uses it.

## Quality gates

`dependency-93-behaviour`, `dependency-value-proof`, `phase-closeout-check`. Tests in [matrix](test-matrix.md) remain future work.

## Release implications

Public contract/digest/schema/FFI changes require normal compatibility review and versioned migration. No release or runtime claim follows from planning.
