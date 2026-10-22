# Sobol sequence qualification

Maturity: planned experimental evaluation/adoption; no runtime work started. Priority class: evaluation. Apply [value policy](../../dependency-value-policy.md).

## Inputs

Accepted prerequisites 72; exact live source, relevant contracts and independent synthetic/standard fixtures. External gates: []. Historical status does not prove capability.

## Outputs

A documented adopt/defer/reject decision, then only if justified a library-backed replacement, deleted/retained-code inventory and consumer acceptance receipts.

## Replacement and benefit

Home-grown direction-number construction in SobolSequence needs mathematical verification; rand_distr does not cover quasi-Monte Carlo.

## Contract

Evaluate sobol_burley only against required dimensionality, sequence length, scrambling and numerical precision. Keep sensitivity estimator/CRN policy separate; verify known sequences and convergence, not only output range.

## Owned paths

Lead: experiment-agent. Prospective implementation: crates/kairo-ecs-cli/src/ensemble.rs. Planning owns this folder and coordinated registry entries. Shared manifests/lockfiles/CI/FFI/schema changes require current-owner handoff and separate bounded claim. Existing CLI/game-theory reservations must be reconciled before those surfaces can be written.

## Blocked paths

Unrelated or actively claimed paths, other repositories and substantive scheduler/resource/clinical semantics. No whole-repository rewrite or dual-backend framework.

## Dependencies

Registry prerequisites: 72. External gates above require actual receipts. Shared path owners serialize; IDs are not priority order. Read-only review may proceed independently; implementation starts only with the value gate and prerequisites accepted.

## Acceptance criteria

Independent sequence vectors/reference, dimension/index limits, reproducible scrambling, multidimensional coverage/integration controls and estimator regression.

Stop criterion: Do not adopt a graphics-oriented sequence if precision/dimension requirements fail; record a bounded alternative search, never silently lower statistical requirements.

## Quality gates

Dependency-specific behavioural gate `dependency-92-behaviour` plus `dependency-value-proof` and `phase-closeout-check`; all pending. Exact pinned features/targets and maintenance qualification, independent negative tests, replay compatibility and baseline-derived cost budgets. Phase 0 must name concrete executable tests before replacement. See [test matrix](test-matrix.md).

## CareOps handoff

CareOps consumes accepted generic APIs/pins only when used by an actual domain/application path. Parent coordinator verifies immutable Kairos commit, versions/features, contract fingerprints and raw behaviour/cost/recovery receipts before a bounded consumer/pin update. Do not install another direct dependency merely because it is present upstream. Any needed parent change is owned by its current domain/application owner, not this upstream writer.

## Release implications

No publication or release authority. Public API/ABI/schema/checkpoint changes require ADR, compatibility assessment, conformance and red-team response. Deferred/rejected evaluation is not implementation acceptance.

## Reference to requalify

https://docs.rs/sobol_burley/latest/sobol_burley/
