# Serde typed control contracts

Maturity: planned experimental evaluation/adoption; no runtime work started. Priority class: foundation. Apply [value policy](../../dependency-value-policy.md).

## Inputs

Accepted prerequisites 70; exact live source, relevant contracts and independent synthetic/standard fixtures. External gates: [{"repository": "careops-sim", "milestone": "polars_data_processing_20261022:R14", "status": "unresolved", "gate_task": "0.1", "required_receipt": "Accepted CareOps Polars completion and upstream Track 70 receipt"}]. Historical status does not prove capability.

## Outputs

A documented adopt/defer/reject decision, then only if justified a library-backed replacement, deleted/retained-code inventory and consumer acceptance receipts.

## Replacement and benefit

Manual control DTO conversion and escaping in scenario/debug paths; audit existing dev-only Serde usage before promotion.

## Contract

Derive typed serialization contracts; retain semantic validation and deterministic field/order rules. Do not move tables from Polars into JSON or adopt a generic serialization facade.

## Owned paths

Lead: contracts-agent. Prospective implementation: crates/kairo-ecs-cli/; crates/kairo-ecs-debug/. Planning owns this folder and coordinated registry entries. Shared manifests/lockfiles/CI/FFI/schema changes require current-owner handoff and separate bounded claim. Existing CLI/game-theory reservations must be reconciled before those surfaces can be written.

## Blocked paths

Unrelated or actively claimed paths, other repositories and substantive scheduler/resource/clinical semantics. No whole-repository rewrite or dual-backend framework.

## Dependencies

Registry prerequisites: 70. External gates above require actual receipts. Shared path owners serialize; IDs are not priority order. Read-only review may proceed independently; implementation starts only with the value gate and prerequisites accepted.

## Acceptance criteria

Roundtrip full-range IDs/ticks, optional fields and invalid versions; unknown/duplicate fields follow explicit reviewed policy; bounded input and no silent defaulting.

Stop criterion: Retain code if serialization cannot preserve identity/schema semantics; do not derive unused DTOs.

## Quality gates

Dependency-specific behavioural gate `dependency-74-behaviour` plus `dependency-value-proof` and `phase-closeout-check`; all pending. Exact pinned features/targets and maintenance qualification, independent negative tests, replay compatibility and baseline-derived cost budgets. Phase 0 must name concrete executable tests before replacement. See [test matrix](test-matrix.md).

## CareOps handoff

CareOps consumes accepted generic APIs/pins only when used by an actual domain/application path. Parent coordinator verifies immutable Kairos commit, versions/features, contract fingerprints and raw behaviour/cost/recovery receipts before a bounded consumer/pin update. Do not install another direct dependency merely because it is present upstream. Any needed parent change is owned by its current domain/application owner, not this upstream writer.

## Release implications

No publication or release authority. Public API/ABI/schema/checkpoint changes require ADR, compatibility assessment, conformance and red-team response. Deferred/rejected evaluation is not implementation acceptance.

## Reference to requalify

https://serde.rs/
