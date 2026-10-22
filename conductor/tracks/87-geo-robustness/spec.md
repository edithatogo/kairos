# geo geometry and route-position primitives

Maturity: planned experimental evaluation/adoption; no runtime work started. Priority class: conditional. Apply [value policy](../../dependency-value-policy.md).

## Inputs

Accepted prerequisites 73; exact live source, relevant contracts and independent synthetic/standard fixtures. External gates: []. Historical status does not prove capability.

## Outputs

A documented adopt/defer/reject decision, then only if justified a library-backed replacement, deleted/retained-code inventory and consumer acceptance receipts.

## Replacement and benefit

Future route geometry, distances/bearings and route-position interpolation must use library algorithms rather than a bespoke GIS stack.

## Contract

Expose only consumer-needed generic geometry primitives. Declare geographic vs projected coordinates, units and transforms; use geodesic distances for geographic data or a qualified projection. No straight-line distance as road travel time, no traffic rules in geometry code.

## Owned paths

Lead: abm-api-agent. Prospective implementation: crates/kairo-ecs-abm/. Planning owns this folder and coordinated registry entries. Shared manifests/lockfiles/CI/FFI/schema changes require current-owner handoff and separate bounded claim. Existing CLI/game-theory reservations must be reconciled before those surfaces can be written.

## Blocked paths

Unrelated or actively claimed paths, other repositories and substantive scheduler/resource/clinical semantics. No whole-repository rewrite or dual-backend framework.

## Dependencies

Registry prerequisites: 73. External gates above require actual receipts. Shared path owners serialize; IDs are not priority order. Read-only review may proceed independently; implementation starts only with the value gate and prerequisites accepted.

## Acceptance criteria

Known line/route distances and positions, endpoint/zero-length/multi-segment paths, coordinate-order and unit errors, geographic projection roundtrip and monotonic route progress.

Stop criterion: Activate only with accepted geographic route consumer; if boundary types alone suffice, use geo-types and revisit this track explicitly rather than install unused algorithms.

## Quality gates

Dependency-specific behavioural gate `dependency-87-behaviour` plus `dependency-value-proof` and `phase-closeout-check`; all pending. Exact pinned features/targets and maintenance qualification, independent negative tests, replay compatibility and baseline-derived cost budgets. Phase 0 must name concrete executable tests before replacement. See [test matrix](test-matrix.md).

## CareOps handoff

CareOps consumes accepted generic APIs/pins only when used by an actual domain/application path. Parent coordinator verifies immutable Kairos commit, versions/features, contract fingerprints and raw behaviour/cost/recovery receipts before a bounded consumer/pin update. Do not install another direct dependency merely because it is present upstream. Any needed parent change is owned by its current domain/application owner, not this upstream writer.

## Release implications

No publication or release authority. Public API/ABI/schema/checkpoint changes require ADR, compatibility assessment, conformance and red-team response. Deferred/rejected evaluation is not implementation acceptance.

## Reference to requalify

https://docs.rs/geo/latest/geo/
