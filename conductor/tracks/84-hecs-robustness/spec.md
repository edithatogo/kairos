# hecs ECS storage qualification

Maturity: planned experimental evaluation/adoption; no runtime work started. Priority class: evaluation. Apply [value policy](../../dependency-value-policy.md).

## Inputs

Accepted prerequisites 78, 64; exact live source, relevant contracts and independent synthetic/standard fixtures. External gates: []. Historical status does not prove capability.

## Outputs

A documented adopt/defer/reject decision, then only if justified a library-backed replacement, deleted/retained-code inventory and consumer acceptance receipts.

## Replacement and benefit

Custom sparse component storage, registry/query construction and entity management; preservation adapters may erase net savings.

## Contract

Evaluate hecs first as storage/query replacement beneath existing DES semantics. Compare standalone bevy_ecs only in the decision evidence, not as another adopted backend. No speculative dual-ECS abstraction. Keep full generational IDs, deterministic iteration/effects and checkpoint/FFI contracts explicit.

## Owned paths

Lead: ecs-agent. Prospective implementation: crates/kairo-ecs-state/. Planning owns this folder and coordinated registry entries. Shared manifests/lockfiles/CI/FFI/schema changes require current-owner handoff and separate bounded claim. Existing CLI/game-theory reservations must be reconciled before those surfaces can be written.

## Blocked paths

Unrelated or actively claimed paths, other repositories and substantive scheduler/resource/clinical semantics. No whole-repository rewrite or dual-backend framework.

## Dependencies

Registry prerequisites: 78, 64. External gates above require actual receipts. Shared path owners serialize; IDs are not priority order. Read-only review may proceed independently; implementation starts only with the value gate and prerequisites accepted.

## Acceptance criteria

Representative spawn/despawn, sparse/dense query and mutation benchmarks; full ID/reuse limits, relationships, snapshot/rollback, consumer bindings and replay after restore; count all compatibility code.

Stop criterion: No adoption unless net maintained complexity falls and required replay/identity/rollback behaviour survives. Record no-go if mapping recreates most of current storage; retain working ECS.

## Quality gates

Dependency-specific behavioural gate `dependency-84-behaviour` plus `dependency-value-proof` and `phase-closeout-check`; all pending. Exact pinned features/targets and maintenance qualification, independent negative tests, replay compatibility and baseline-derived cost budgets. Phase 0 must name concrete executable tests before replacement. See [test matrix](test-matrix.md).

## CareOps handoff

CareOps consumes accepted generic APIs/pins only when used by an actual domain/application path. Parent coordinator verifies immutable Kairos commit, versions/features, contract fingerprints and raw behaviour/cost/recovery receipts before a bounded consumer/pin update. Do not install another direct dependency merely because it is present upstream. Any needed parent change is owned by its current domain/application owner, not this upstream writer.

## Release implications

No publication or release authority. Public API/ABI/schema/checkpoint changes require ADR, compatibility assessment, conformance and red-team response. Deferred/rejected evaluation is not implementation acceptance.

## Reference to requalify

https://docs.rs/hecs/latest/hecs/
