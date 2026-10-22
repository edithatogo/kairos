# proptest robustness

Planned; no runtime acceptance. Apply [value policy](../../dependency-value-policy.md) and [sequencing](../../dependency-sequencing.md).

## Inputs

Prerequisites: none beyond fresh source/owner/value qualification. Read-only cross-track inventory is allowed before prerequisite acceptance; replacement is not.

## Outputs

Explicit adopt/defer/reject decision, qualified bounded replacement and immutable consumer receipts.

## Owned paths

Lead: conformance-agent. Prospective target: Expand existing kairo-ecs-core dev dependency to bounded identity/resource/checkpoint contract tests; CareOps owns domain invariants. Planning owns this track folder. Runtime paths, shared manifests and fixtures require current owner handoff and a narrow claim; no blanket ownership transfer.

## Blocked paths

Other repositories, active reservations and unrelated runtime semantics. No implicit CI/publishing permission.

## Dependencies

Registry dependencies []. No dependency on completed Polars for Tracks 93/94/96: this prevents a migration cycle. Owner-coordinated source inventory decides deleted versus retained targets.

## Contract

Reuse existing proptest rather than introduce a second property framework. Establish independent reference models, reproducible seeds, bounded cases/shrinking and saved minimal regressions before replacing internals. Initial acceptance covers existing behaviour; future dependency tracks add their own properties, so acceptance does not wait for their implementations. Never make the implementation its own oracle or remove conformance/fuzz tests.

## Acceptance criteria

Mutation/negative controls show properties detect broken conservation, ordering and invalid-input rollback; seed replay and shrinking work; generated IDs include full ranges; test budgets enforced. New-format resume properties remain Track 70 acceptance responsibility.

Record exact features/version, maintenance/security/licence/MSRV/target qualification and wrapper-inclusive code/cost budgets before adoption. No net benefit means defer/reject, not implemented.

## CareOps handoff

Accepted immutable Kairos receipt before consuming upstream API changes. Direct CareOps implementations/tests remain parent-owned in the paired sha2/tempfile/thiserror/proptest tracks. Optional libraries need a bounded parent packet and concrete consumer; no duplicate direct dependency solely because upstream uses it.

## Quality gates

`dependency-96-behaviour`, `dependency-value-proof`, `phase-closeout-check`. Tests in [matrix](test-matrix.md) remain future work.

## Release implications

Public contract/digest/schema/FFI changes require normal compatibility review and versioned migration. No release or runtime claim follows from planning.
