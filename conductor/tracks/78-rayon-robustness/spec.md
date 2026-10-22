# Rayon bounded independent-work execution

Maturity: planned experimental evaluation/adoption; no runtime work started. Priority class: foundation. Apply [value policy](../../dependency-value-policy.md).

## Inputs

Accepted prerequisites 72, 77; exact live source, relevant contracts and independent synthetic/standard fixtures. External gates: [{"repository": "careops-sim", "milestone": "rand_distr_sampling_20261022:R25", "status": "unresolved", "gate_task": "0.1", "required_receipt": "Accepted parent sampling adoption and exact upstream pin"}]. Historical status does not prove capability.

## Outputs

A documented adopt/defer/reject decision, then only if justified a library-backed replacement, deleted/retained-code inventory and consumer acceptance receipts.

## Replacement and benefit

Repeated std::thread::scope partition/spawn/join machinery in ensemble, sensitivity and particle workloads; vector-env code may be synthetic and must not inflate deletion claims.

## Contract

Use bounded reusable Rayon pools for independent work only. Seeds remain logical run/entity/purpose keyed, outputs indexed, floating reductions fixed-order. Coordinate a process CPU budget with Polars; do not use pool scheduling as DES event ordering or nest unbounded pools.

## Owned paths

Lead: experiment-agent. Prospective implementation: crates/kairo-ecs-cli/src/ensemble.rs; crates/kairo-ecs-ml/src/lib.rs. Planning owns this folder and coordinated registry entries. Shared manifests/lockfiles/CI/FFI/schema changes require current-owner handoff and separate bounded claim. Existing CLI/game-theory reservations must be reconciled before those surfaces can be written.

## Blocked paths

Unrelated or actively claimed paths, other repositories and substantive scheduler/resource/clinical semantics. No whole-repository rewrite or dual-backend framework.

## Dependencies

Registry prerequisites: 72, 77. External gates above require actual receipts. Shared path owners serialize; IDs are not priority order. Read-only review may proceed independently; implementation starts only with the value gate and prerequisites accepted.

## Acceptance criteria

1/2/4 worker exact traces/results, skewed tasks, small/large ensembles, panic/error propagation, cancellation ownership and thread count/RSS/latency versus baseline.

Stop criterion: Reject if total wrapper and scheduling complexity grows without a demonstrated benefit or accepted workloads regress materially; no claimed real RL capability from synthetic batches.

## Quality gates

Dependency-specific behavioural gate `dependency-78-behaviour` plus `dependency-value-proof` and `phase-closeout-check`; all pending. Exact pinned features/targets and maintenance qualification, independent negative tests, replay compatibility and baseline-derived cost budgets. Phase 0 must name concrete executable tests before replacement. See [test matrix](test-matrix.md).

## CareOps handoff

CareOps consumes accepted generic APIs/pins only when used by an actual domain/application path. Parent coordinator verifies immutable Kairos commit, versions/features, contract fingerprints and raw behaviour/cost/recovery receipts before a bounded consumer/pin update. Do not install another direct dependency merely because it is present upstream. Any needed parent change is owned by its current domain/application owner, not this upstream writer.

## Release implications

No publication or release authority. Public API/ABI/schema/checkpoint changes require ADR, compatibility assessment, conformance and red-team response. Deferred/rejected evaluation is not implementation acceptance.

## Reference to requalify

https://docs.rs/rayon/latest/rayon/
