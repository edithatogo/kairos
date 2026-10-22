# serde_json metadata and control encoding

Maturity: planned experimental evaluation/adoption; no runtime work started. Priority class: foundation. Apply [value policy](../../dependency-value-policy.md).

## Inputs

Accepted prerequisites 74; exact live source, relevant contracts and independent synthetic/standard fixtures. External gates: []. Historical status does not prove capability.

## Outputs

A documented adopt/defer/reject decision, then only if justified a library-backed replacement, deleted/retained-code inventory and consumer acceptance receipts.

## Replacement and benefit

Hand-assembled JSON/control/trace metadata and custom escaping; existing serde_json dev dependencies are not proof runtime paths use it.

## Contract

Replace only JSON control encoders/readers with serde_json using accepted typed contracts. Full tabular traces stay in Polars/Parquet. Reject nonfinite values explicitly; never narrow IDs/ticks through f64.

## Owned paths

Lead: contracts-agent. Prospective implementation: crates/kairo-ecs-debug/; crates/kairo-ecs-cli/. Planning owns this folder and coordinated registry entries. Shared manifests/lockfiles/CI/FFI/schema changes require current-owner handoff and separate bounded claim. Existing CLI/game-theory reservations must be reconciled before those surfaces can be written.

## Blocked paths

Unrelated or actively claimed paths, other repositories and substantive scheduler/resource/clinical semantics. No whole-repository rewrite or dual-backend framework.

## Dependencies

Registry prerequisites: 74. External gates above require actual receipts. Shared path owners serialize; IDs are not priority order. Read-only review may proceed independently; implementation starts only with the value gate and prerequisites accepted.

## Acceptance criteria

Independent parser reads escaped Unicode/control characters and exact integer/string representations; malformed, duplicate-key and oversized inputs reject as specified.

Stop criterion: No direct dependency where an existing accepted upstream API already serializes the same object; no second JSON representation.

## Quality gates

Dependency-specific behavioural gate `dependency-75-behaviour` plus `dependency-value-proof` and `phase-closeout-check`; all pending. Exact pinned features/targets and maintenance qualification, independent negative tests, replay compatibility and baseline-derived cost budgets. Phase 0 must name concrete executable tests before replacement. See [test matrix](test-matrix.md).

## CareOps handoff

CareOps consumes accepted generic APIs/pins only when used by an actual domain/application path. Parent coordinator verifies immutable Kairos commit, versions/features, contract fingerprints and raw behaviour/cost/recovery receipts before a bounded consumer/pin update. Do not install another direct dependency merely because it is present upstream. Any needed parent change is owned by its current domain/application owner, not this upstream writer.

## Release implications

No publication or release authority. Public API/ABI/schema/checkpoint changes require ADR, compatibility assessment, conformance and red-team response. Deferred/rejected evaluation is not implementation acceptance.

## Reference to requalify

https://docs.rs/serde_json/latest/serde_json/
