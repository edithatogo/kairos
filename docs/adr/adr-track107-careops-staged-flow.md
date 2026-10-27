# ADR 107-1: Purpose-specific staged resource runtime

## Status

Accepted after reviewer-agent assessment; experimental alpha API only.

## Context

The historical `FlowRuntime` was a broad runtime with callback, checkpoint, and fidelity behaviors removed by Track70. CareOps R10.3 uses only a small scheduler-backed resource acquisition/release contract. Restoring a partial type under the old name would falsely imply compatibility with the removed surface.

## Decision

Add `StagedResourceRuntime` in the Rust API root `crates/kairo-ecs-des`, which this ADR proposes adding to the protected-surface inventory. It models request submission, active/queued state, revisioned leases, scheduled release and FIFO promotion for staged allocation consumers. It remains explicitly experimental and does not replace or restore `FlowRuntime`.

## Compatibility and scope

Additive API, `compatible` classification at alpha because it adds a new explicitly named experimental API without changing an existing surface. CareOps consumers must make a source migration from the removed `FlowRuntime` calls to `StagedResourceRuntime`; this is not source compatibility with `FlowRuntime`. No scheduler order, RNG, replay, binding, or parent pin changes. No beta-or-later release claim until migration and compatibility review are complete.

## Evidence required

API review, consumer-shaped Rust conformance test and versioned fixture, rustdoc and migration note, reviewer-agent assessment, and exact test evidence.
