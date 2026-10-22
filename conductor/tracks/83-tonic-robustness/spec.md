# Tonic interoperable RPC transport

Maturity: planned experimental evaluation/adoption; no runtime work started. Priority class: conditional. Apply [value policy](../../dependency-value-policy.md).

## Inputs

Accepted prerequisites 82; exact live source, relevant contracts and independent synthetic/standard fixtures. External gates: []. Historical status does not prove capability.

## Outputs

A documented adopt/defer/reject decision, then only if justified a library-backed replacement, deleted/retained-code inventory and consumer acceptance receipts.

## Replacement and benefit

GrpcSocketTransport uses custom framed TCP; names alone do not establish standard gRPC interoperability.

## Contract

Use released Tonic client/server with accepted Prost/Tokio boundaries. Replace custom framing/connection machinery after parity. Retain migration authority, deduplication, GVT/rollback and causality rules. Qualify transport authentication, deadlines, limits and retry idempotency; do not equate newer gRPC-Rust previews with stable Tonic.

## Owned paths

Lead: distributed-agent. Prospective implementation: crates/kairo-ecs-grpc/. Planning owns this folder and coordinated registry entries. Shared manifests/lockfiles/CI/FFI/schema changes require current-owner handoff and separate bounded claim. Existing CLI/game-theory reservations must be reconciled before those surfaces can be written.

## Blocked paths

Unrelated or actively claimed paths, other repositories and substantive scheduler/resource/clinical semantics. No whole-repository rewrite or dual-backend framework.

## Dependencies

Registry prerequisites: 82. External gates above require actual receipts. Shared path owners serialize; IDs are not priority order. Read-only review may proceed independently; implementation starts only with the value gate and prerequisites accepted.

## Acceptance criteria

Independent non-Kairos gRPC peer, two-process failure/reconnect/backpressure and duplicate messages; wire version rejection, transport policy and unchanged PDES ordering.

Stop criterion: Stop if distributed RPC has no active consumer; do not add HTTP/2 overhead to in-process execution or retain two production transports indefinitely.

## Quality gates

Dependency-specific behavioural gate `dependency-83-behaviour` plus `dependency-value-proof` and `phase-closeout-check`; all pending. Exact pinned features/targets and maintenance qualification, independent negative tests, replay compatibility and baseline-derived cost budgets. Phase 0 must name concrete executable tests before replacement. See [test matrix](test-matrix.md).

## CareOps handoff

CareOps consumes accepted generic APIs/pins only when used by an actual domain/application path. Parent coordinator verifies immutable Kairos commit, versions/features, contract fingerprints and raw behaviour/cost/recovery receipts before a bounded consumer/pin update. Do not install another direct dependency merely because it is present upstream. Any needed parent change is owned by its current domain/application owner, not this upstream writer.

## Release implications

No publication or release authority. Public API/ABI/schema/checkpoint changes require ADR, compatibility assessment, conformance and red-team response. Deferred/rejected evaluation is not implementation acceptance.

## Reference to requalify

https://docs.rs/tonic/latest/tonic/
