# Tract inference qualification

Maturity: planned experimental evaluation/adoption; no runtime work started. Priority class: conditional. Apply [value policy](../../dependency-value-policy.md).

## Inputs

Accepted prerequisites 78; exact live source, relevant contracts and independent synthetic/standard fixtures. External gates: []. Historical status does not prove capability.

## Outputs

A documented adopt/defer/reject decision, then only if justified a library-backed replacement, deleted/retained-code inventory and consumer acceptance receipts.

## Replacement and benefit

OrtSession::run copies inputs to output shapes instead of executing a model; select real inference only for an actual model requirement.

## Contract

Qualify Tract against required ONNX models/operators, target platforms, numerical tolerances and thread budgets. Retain inference tick-phase/fallback policy. ort remains an alternative requiring a separate decision if Tract fails, not a second backend installed now.

## Owned paths

Lead: ml-integration-agent. Prospective implementation: crates/kairo-ecs-ml/. Planning owns this folder and coordinated registry entries. Shared manifests/lockfiles/CI/FFI/schema changes require current-owner handoff and separate bounded claim. Existing CLI/game-theory reservations must be reconciled before those surfaces can be written.

## Blocked paths

Unrelated or actively claimed paths, other repositories and substantive scheduler/resource/clinical semantics. No whole-repository rewrite or dual-backend framework.

## Dependencies

Registry prerequisites: 78. External gates above require actual receipts. Shared path owners serialize; IDs are not priority order. Read-only review may proceed independently; implementation starts only with the value gate and prerequisites accepted.

## Acceptance criteria

Independent reference outputs for real model bytes, unsupported operator/model rejection, shape/type/nonfinite/oversized inputs, timeout/fallback and pinned model/version hashes.

Stop criterion: No adoption without a required model or if supported operators/targets fail. Replacing a placeholder is new capability, not proven code-reduction or preserved inference parity.

## Quality gates

Dependency-specific behavioural gate `dependency-89-behaviour` plus `dependency-value-proof` and `phase-closeout-check`; all pending. Exact pinned features/targets and maintenance qualification, independent negative tests, replay compatibility and baseline-derived cost budgets. Phase 0 must name concrete executable tests before replacement. See [test matrix](test-matrix.md).

## CareOps handoff

CareOps consumes accepted generic APIs/pins only when used by an actual domain/application path. Parent coordinator verifies immutable Kairos commit, versions/features, contract fingerprints and raw behaviour/cost/recovery receipts before a bounded consumer/pin update. Do not install another direct dependency merely because it is present upstream. Any needed parent change is owned by its current domain/application owner, not this upstream writer.

## Release implications

No publication or release authority. Public API/ABI/schema/checkpoint changes require ADR, compatibility assessment, conformance and red-team response. Deferred/rejected evaluation is not implementation acceptance.

## Reference to requalify

https://github.com/sonos/tract

## CareOps-Sim impact and opportunity review

Closeout includes the [lightweight downstream review](../../dependency-value-policy.md#careops-sim-impact-and-opportunity-review). Kairos supplies its accepted immutable receipt; the CareOps-Sim coordinator and consumer owner record implications, worthwhile opportunities and one of the four outcomes in `conductor/dependency-handoffs.json`, keyed by `owner_track: 89-tract-robustness`. Reuse existing consumer tracks; no automatic direct dependency or duplicate implementation track. For already accepted phases this is downstream follow-through, not reopening or changing historical acceptance. Review remains pending until evidenced separately from implementation.
