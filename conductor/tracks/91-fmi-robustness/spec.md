# rust-fmi adapter qualification

Maturity: planned experimental evaluation/adoption; no runtime work started. Priority class: evaluation. Apply [value policy](../../dependency-value-policy.md).

## Inputs

Accepted prerequisites 78; exact live source, relevant contracts and independent synthetic/standard fixtures. External gates: []. Historical status does not prove capability.

## Outputs

A documented adopt/defer/reject decision, then only if justified a library-backed replacement, deleted/retained-code inventory and consumer acceptance receipts.

## Replacement and benefit

Hand-maintained FMI function tables/import/export/schema glue; assess complete adapter replacement before growing archive/XML replacements.

## Contract

Evaluate maintained rust-fmi import/schema/export surfaces for required FMI2/3 modes and target binaries; smaller ecosystem demands conformance and maintenance checks. Retain Kairos co-simulation clock, lifecycle and state ownership. Do not adopt work-in-progress simulation master as a replacement scheduler.

## Owned paths

Lead: fmi-agent. Prospective implementation: crates/kairo-ecs-fmi/. Planning owns this folder and coordinated registry entries. Shared manifests/lockfiles/CI/FFI/schema changes require current-owner handoff and separate bounded claim. Existing CLI/game-theory reservations must be reconciled before those surfaces can be written.

## Blocked paths

Unrelated or actively claimed paths, other repositories and substantive scheduler/resource/clinical semantics. No whole-repository rewrite or dual-backend framework.

## Dependencies

Registry prerequisites: 78. External gates above require actual receipts. Shared path owners serialize; IDs are not priority order. Read-only review may proceed independently; implementation starts only with the value gate and prerequisites accepted.

## Acceptance criteria

Independent reference FMUs, lifecycle errors, get/set/step, platform loading, schema and supported export roundtrip, restore if required and leak/cleanup failures.

Stop criterion: Reject if modes/platforms or maintenance do not meet need; then use bounded zip/quick-xml replacements. If accepted, supersede their redundant direct-adoption scopes through documented owner handoff.

## Quality gates

Dependency-specific behavioural gate `dependency-91-behaviour` plus `dependency-value-proof` and `phase-closeout-check`; all pending. Exact pinned features/targets and maintenance qualification, independent negative tests, replay compatibility and baseline-derived cost budgets. Phase 0 must name concrete executable tests before replacement. See [test matrix](test-matrix.md).

## CareOps handoff

CareOps consumes accepted generic APIs/pins only when used by an actual domain/application path. Parent coordinator verifies immutable Kairos commit, versions/features, contract fingerprints and raw behaviour/cost/recovery receipts before a bounded consumer/pin update. Do not install another direct dependency merely because it is present upstream. Any needed parent change is owned by its current domain/application owner, not this upstream writer.

## Release implications

No publication or release authority. Public API/ABI/schema/checkpoint changes require ADR, compatibility assessment, conformance and red-team response. Deferred/rejected evaluation is not implementation acceptance.

## Reference to requalify

https://github.com/jondo2010/rust-fmi
