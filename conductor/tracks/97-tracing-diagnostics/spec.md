# tracing qualification

Planned; no runtime acceptance. Apply [value policy](../../dependency-value-policy.md) and [sequencing](../../dependency-sequencing.md).

## Inputs

Prerequisites: [78]. Read-only cross-track inventory is allowed before prerequisite acceptance; replacement is not.

## Outputs

Explicit adopt/defer/reject decision, qualified bounded replacement and immutable consumer receipts.

## Owned paths

Lead: contracts-agent. Prospective target: Run/worker/adapter/checkpoint diagnostic context only where existing logs cannot diagnose a named failure. Planning owns this track folder. Runtime paths, shared manifests and fixtures require current owner handoff and a narrow claim; no blanket ownership transfer.

## Blocked paths

Other repositories, active reservations and unrelated runtime semantics. No implicit CI/publishing permission.

## Dependencies

Registry dependencies [78]. No dependency on completed Polars for Tracks 93/94/96: this prevents a migration cycle. Owner-coordinated source inventory decides deleted versus retained targets.

## Contract

Optional structured spans/events; libraries do not install global subscribers. CareOps configures application logging. Diagnostics never replace authoritative event logs, alter virtual time, consume simulation RNG or emit patient/private data by default. No core sequencing dependency on tracing; no collector/service adoption implied.

## Acceptance criteria

Named diagnostic failure becomes explainable; disabled/enabled tracing yields identical simulation outputs; bounded overhead, field redaction, correlated run/worker IDs and backpressure behaviour.

Record exact features/version, maintenance/security/licence/MSRV/target qualification and wrapper-inclusive code/cost budgets before adoption. No net benefit means defer/reject, not implemented.

## CareOps handoff

Accepted immutable Kairos receipt before consuming upstream API changes. Direct CareOps implementations/tests remain parent-owned in the paired sha2/tempfile/thiserror/proptest tracks. Optional libraries need a bounded parent packet and concrete consumer; no duplicate direct dependency solely because upstream uses it.

## Quality gates

`dependency-97-behaviour`, `dependency-value-proof`, `phase-closeout-check`. Tests in [matrix](test-matrix.md) remain future work.

## Release implications

Public contract/digest/schema/FFI changes require normal compatibility review and versioned migration. No release or runtime claim follows from planning.
