# Q4 lifecycle snapshots and sidecar — experimental contract v1

Status: accepted by coordinator and independent bounded Track03/04/25 contract review for implementation (4 October 2026). Experimental source/release hold remains. Generic Rust-native telemetry; no clinical policy, binding changes or portable checkpoint support.

## Producer contract (Track03)

Add experimental `pub const fn entity_id(self) -> EntityId` on ResourceId, RequestId and WorkId, and `pub const fn request_id(self) -> RequestId` / `pub const fn revision(self) -> u64` on LeaseId. These are read-only accessors; keep private fields and no public raw constructors. They allow cross-crate telemetry without context/registry access.

Add public `LifecycleSnapshot` and `LifecycleRecord.snapshot: LifecycleSnapshot`. Reexport from DES lib. Snapshot fields:
- owner: EntityId; work: Option<WorkId>; priority_level: i32.
- capacity, queue_len, active_count: u32 (checked conversion).
- strategy: Option<PreemptionStrategy> (request's configured victim policy).
- preemptor_request: Option<RequestId> (only direct Preempted/Aborted eviction rows).
- causal_lease: Option<LeaseId> (lease removed by eviction/completion/release/cancel/cleanup, or current grant/resume/restart lease; separate from existing resulting record.lease).
- progress: Option<WorkProgress> (only timed requests; owned value projected with inspected(at), never a context clone).

Capture snapshot from staged ResourceStage and request/progress values at each existing record()/record_transition() call, immediately after that named mutation and before later mutations. Preserve existing row count/order/ordinal, request state and notification order. Preempted row captures the immediate victim removal *before* strategy-specific queue/restart/abort mutations; a later Aborted row captures its own state. Document that precise intermediate queue count. Zero-duration Queued/Granted/Completed rows must differ (queue1/active0, queue0/active1, queue0/active0). Do not reconstruct historical rows from committed World. WorkProgress::inspected projects current active effort without mutating authoritative state; overflow fails preflight before scheduler consumption. Full-plan rejection emits no rows. Existing boundary rollback truncates staged rows together. Cleanup snapshots retain IDs/progress before despawn. No caller callbacks or allocations with external effects in capture.

## Sidecar (Track04)

Add opt-in `resource-lifecycle` feature to kairo-ecs-arrow, default off. Existing pinned Arrow array/schema60.0.0 and optional DES dependency; no Arrow dependency in DES and no mandatory new dependency in default telemetry. Feature compiler floor1.88 follows existing ArrowIO lane; default dependency-floor assertions stay independent. Coordinator owns Cargo.toml, Cargo.lock and lib module wiring. Add a separate `resource-lifecycle-io` opt-in feature for interop tests/consumers, implying resource-lifecycle plus existing optional kairo-ecs-arrow-io with IPC. This avoids unconditional dev dependencies raising the default compiler floor; semantic conversion has no transport calls.

New `resource_lifecycle` module exposes RESOURCE_LIFECYCLE_STREAM = kairo_ecs.resource_lifecycle.v1, schema() -> SchemaRef, schema_fingerprint() -> String, encode(run_id: &str, records: &[LifecycleRecord]) -> Result<RecordBatch, LifecycleError>. Encoding consumes only immutable records (no runtime argument). Reject whitespace-only run ID, duplicate event/ordinal key in this batch, and non-contiguous ordinals per causal event; input order must be strictly event encounter blocks with ordinal0..N, never sort or silently deduplicate. An event may not reappear later in the batch. Reject any current/causal LeaseId whose request_id differs from record.request, and any progress value without a work ID. Other runtime-only handle validity is not reconstructible in the writer and is not claimed. Empty input returns typed zero-row batch. Batch-local uniqueness is not whole-run uniqueness across independent calls; writer owns that aggregation.

Physical field order/types/nullability:
1 schema_version UInt16 required (1)
2 run_id Utf8 required
3 causal_event_id FixedSizeBinary(12) required
4 transition_ordinal UInt32 required
5 time_ticks FixedSizeBinary(16) required
6 time_scale Utf8 required (ticks)
7 resource_id FixedSizeBinary(12) required
8 request_id FixedSizeBinary(12) required
9 owner_id FixedSizeBinary(12) required
10 work_id FixedSizeBinary(12) nullable
11 lease_revision UInt64 nullable (resulting record.lease)
12 causal_lease_revision UInt64 nullable
13 transition Utf8 required (queued/granted/released/cancelled/timed_out/preempted/resumed/restarted/completed/aborted)
14 request_state Utf8 required (matching Rust variant snake_case)
15 strategy Utf8 nullable (suspend/abort/restart)
16 preemptor_request_id FixedSizeBinary(12) nullable
17 priority Int32 required
18 queue_len UInt32 required
19 active_count UInt32 required
20 capacity UInt32 required
21 original_duration_ticks FixedSizeBinary(16) nullable
22 useful_elapsed_ticks FixedSizeBinary(16) nullable
23 remaining_ticks FixedSizeBinary(16) nullable
24 cumulative_busy_ticks FixedSizeBinary(16) nullable
25 attempt_revision UInt64 nullable
26 execution_revision UInt64 nullable
27 reason Utf8 nullable (v1 always null; do not invent inferred reasons)

IDs use existing event_log.v1 encoding: u64 index LE + u32 generation LE; all duration/time ticks use u128 LE. Preserve event_log.v1 source, machine schema, fingerprint, default serializers and existing tests byte-for-byte (except module declaration outside those surfaces). Real optional Arrow RecordBatch encoding; use existing ArrowIO IPC/Parquet transport outside semantic module. Tests must prove typed null/empty arrays, full width IDs/ticks, key rejection, frozen field fingerprint and actual IPC roundtrip (test-only existing ArrowIO transport). Schema artifact in schemas/arrow/resource_lifecycle_v1.schema.json matches this order. Do not call TSV/text a binary Arrow stream.

## Acceptance and compatibility

Public LifecycleRecord struct gains a field: additive semantics but source-breaking for exhaustive struct literals; experimental Track25 release hold remains, and migration docs show field access rather than fabricated stable compatibility. No event_log.v1 or FFI ABI change. Contract coordinator integrates Track03 source + Track04 encoding and reviewers independently examine domain/resource/arithmetic/schema oracles.

Producer tests cover same-event snapshots, urgent Suspend/Abort/Restart, causal victim lease/preemptor, elapsed/remaining/busy accounting, terminal cleanup, manual work nulls, rejection atomicity, stale completion no duplicate, same-runtime pause parity. Encoder tests cover schema physical types/order/nullability, empty/nullable/full-width IDs/ticks, error cases, actual ArrowIO roundtrip and unchanged event-log fingerprint. Q4.4 later supplies named staff/staged bed/cleaning example using public APIs only; Q4.5 integrates full phase evidence. Track22 portable checkpoint and whole-run output writer remain separately deferred.
