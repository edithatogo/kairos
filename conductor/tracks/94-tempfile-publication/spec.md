# tempfile robustness

Planned; no runtime acceptance. Apply [value policy](../../dependency-value-policy.md) and [sequencing](../../dependency-sequencing.md).

## Inputs

Prerequisites: none beyond fresh source/owner/value qualification. Read-only cross-track inventory is allowed before prerequisite acceptance; replacement is not.

## Outputs

Explicit adopt/defer/reject decision, qualified bounded replacement and immutable consumer receipts.

## Owned paths

Lead: arrow-agent. Prospective target: Kairos repeated test temp paths and staging helpers; CareOps repeated TestTempDir and local output staging. Planning owns this track folder. Runtime paths, shared manifests and fixtures require current owner handoff and a narrow claim; no blanket ownership transfer.

## Blocked paths

Other repositories, active reservations and unrelated runtime semantics. No implicit CI/publishing permission.

## Dependencies

Registry dependencies []. No dependency on completed Polars for Tracks 93/94/96: this prevents a migration cycle. Owner-coordinated source inventory decides deleted versus retained targets.

## Contract

Replace naming/cleanup with TempDir and NamedTempFile where useful. Establish tested reusable staging primitive and test cleanup before Polars; Track 70 retains end-to-end durable publication acceptance. Stage beside destination; handle overwrite/no-clobber explicitly. persist alone does not prove crash durability: file/directory sync where required, manifest-last ordering, disk-full and recovery remain owner responsibilities. No filesystem randomness enters simulation seeds or output identities.

## Acceptance criteria

Concurrent directory creation/isolation; cleanup on normal/error paths; intentional retained diagnostics; same-filesystem publication, failure before/after persist, no-clobber and old-output preservation. End-to-end process-kill/disk-full/manifest recovery remains mandatory in Polars and is not certified by this prerequisite.

Record exact features/version, maintenance/security/licence/MSRV/target qualification and wrapper-inclusive code/cost budgets before adoption. No net benefit means defer/reject, not implemented.

## CareOps handoff

Accepted immutable Kairos receipt before consuming upstream API changes. Direct CareOps implementations/tests remain parent-owned in the paired sha2/tempfile/thiserror/proptest tracks. Optional libraries need a bounded parent packet and concrete consumer; no duplicate direct dependency solely because upstream uses it.

## Quality gates

`dependency-94-behaviour`, `dependency-value-proof`, `phase-closeout-check`. Tests in [matrix](test-matrix.md) remain future work.

## Release implications

Public contract/digest/schema/FFI changes require normal compatibility review and versioned migration. No release or runtime claim follows from planning.
