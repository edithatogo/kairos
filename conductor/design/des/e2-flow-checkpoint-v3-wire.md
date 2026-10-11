# Explicit Flow checkpoint V3 wire owner contract

Status: Owner contract reviewed by root and GPT-6.1-sol. Preparation only, no code,
wire acceptance or implementation dispatch. Base: f96c3a86eb816ac244a53171933edcc5c9ad6a11; Rust/Cargo 1.99.0 only. Track03 owns
Flow wire semantics; Track22 owns downstream composite joins. This consumes the
accepted deferred-allocation ADR and explicit native V3 contract. Native V3 is
currently being implemented separately, not accepted by this document.

## Dependencies and future owned paths

Freeze the actual accepted native V3 commit, DTO/conversions, native error joins
and input hashes before binding wire implementation. No placeholder native type,
unchecked conversion, duplicate restore engine or V3-to-V2 projection is valid.
The future leaf should own `crates/kairo-ecs-des/src/flow/checkpoint/wire.rs` and
new `crates/kairo-ecs-des/tests/flow_checkpoint_wire_v3.rs`, plus result/logs.
Include v3.rs only for a reviewed native-helper need. Protect native validation,
flow.rs, legacy tests, calibration and manifests. Bind exact paths at dispatch.

This is explicit wire only. Keep FlowCheckpointImage, FlowCheckpointRef, their
command iterator and current routed capture/encode/decode endpoints unchanged.
The current router continues to reject V3 magic. Do not add a dummy V3 branch or
wire method that merely returns unsupported. The subsequent routed-view/coherent
join owns all exhaustive match changes and must retain complete Flush commands.

## Exact experimental API and layout

Use the accepted FlowCheckpointV3 and FlowCheckpointWireLimits:

```rust
impl FlowCheckpointV3 {
    pub fn encode_wire_v3(&self, limits: FlowCheckpointWireLimits)
        -> Result<Vec<u8>, FlowCheckpointWireError>;
    pub fn decode_wire_v3(bytes: &[u8], limits: FlowCheckpointWireLimits)
        -> Result<Self, FlowCheckpointWireError>;
}
```

These are doc-hidden experimental methods like V1/V2. Add no limit/error enum
field and no alternate decoder fallback. Bytes are canonical little-endian:

1. Eight-byte magic `b"KFLOWV3\0"`.
2. u16 wire schema 1, followed by u32 image version 3.
3. All V2 common fields in existing V2 order, using complete FlowCommandV3 rows.
4. Existing resource_grant_flags count and (ResourceId,bool) rows.
5. Existing started_frontier option tag and optional SimTime.
6. allocation_modes count and (ResourceId,mode-tag) rows.

ResourceId is the existing EntityId encoding, u64 index then u32 generation.
Count encoding is existing checked usize-to-u64. Modes are u8: 0 Eager, 1
Deferred. Boolean uses only 0/1 with the existing InvalidBoolean distinction.
Frontier uses existing option tag 0/1 and optional u128 time. Mode rows are
13 bytes each after the count. Do not serialize a second pending Flush index.

FlowCommandV3 tags 0..12 exactly retain V2 payloads:

| Tag | Command payload |
| --- | --- |
| 0 | Submit(RequestId) |
| 1 | Release(LeaseId) |
| 2 | Capacity(ResourceId,u32) |
| 3 | Remove(ResourceId) |
| 4 | Despawn(EntityId) |
| 5 | Deadline(RequestId) |
| 6 | Cancel(RequestId) |
| 7 | Reprioritize(RequestId,i32) |
| 8 | Completion(RequestId,LeaseId,u64,SimTime) |
| 9 | Notify |
| 10 | Domain(WorkId,EventKind) |
| 11 | DomainControl(WorkId,EventKind,FlowDomainControl) |
| 12 | GrantEnabled(ResourceId,bool) |
| 13 | AllocationFlush(ResourceId) |

Do not alter existing EventKind, work/request/lease, callback, restart,
notification, nested FlowDispatch or FlowError encodings. Schema and image version are separate checks.

## Existing implementation seams

Reuse wire.rs Reader/Writer, scan_flow_v2 and nonowning scan_* helpers,
preflight_counts_v2, validate_canonical_order_v2 and explicit encode/decode
field traversal. Bind accepted source hashes at dispatch. Report the complete
field map and scan/measurement/ownership boundary with one rich runtime fixture
before broad edits. Borrowed common views are allowed; stripped owned V2 DTOs
and unrelated legacy refactors are not.

## Full scan and ownership boundary

Add a separate `scan_flow_v3(bytes, limits) -> Result<(), FlowCheckpointWireError>`.
It consumes the entire input, including the last mode row/tag, and checks finish
before any DTO collection or string ownership. It must not call owning decode
with scanning flags that return empty collections or fabricate an empty image.

The scan uses bounded Reader raw/scalar/count operations. UTF8 keys are borrowed
and validated; payload bytes are borrowed and advanced. No reserve/push/collect,
String creation/clone, map/set insertion or DTO construction occurs in the scan.
Nested scheduler/world rows, callback registrations, context/restart stores and
factory keys, component rows, queues and active allocations must all be consumed.
Queue/allocation order can be checked with a previous scalar key. Reuse existing
nonowning scan helpers, not their owning get_* equivalents. All option/bool/enum
tag and count checks still execute. Do not skip nested canonical checks merely
because a scan retains no values.

Decode order: Reader::new byte cap, complete nonowning scan and finish, then a
fresh Reader for owning complete V3 decode, finish, bounded canonical validation.
Cross-field joins use bounded DTO validation. Prove ownership entry with a
thread-local counter and valid positive control; no unsafe global allocator.

Encode order: image-version check, borrowed count/aggregate preflight, full
Writer::measure traversal, bounded canonical validation, output allocation and
writing. The canonical validator may allocate bounded uniqueness sets only after
full byte/key/payload measurement. A late payload budget error must occur before
that allocation phase. Test-only thread-local canonical-entry counters are
acceptable and must have a valid positive control.

## Actual limits and checked accounting

FlowCheckpointWireLimits has exactly `flow: FlowCheckpointLimits`,
`max_wire_bytes` and `max_total_records`. All collection/aggregate native limits
below are nested under `limits.flow`; do not invent top-level max_resources or
new wire-limit fields. Keep accepted defaults and legacy field behavior.

| Data | Existing bound |
| --- | --- |
| Entire encoded/decoded bytes | limits.max_wire_bytes |
| Every collection row count, including flags and modes | limits.max_total_records aggregate |
| Resources, resource_grant_flags, allocation_modes independently | limits.flow.max_resources |
| Commands, including Flush | limits.flow.max_commands |
| Releases + despawns + gates + Flush | limits.flow.max_pending_operations aggregate |
| World slots/free/live IDs | limits.flow.max_entities |
| Scheduler entries | limits.flow.max_scheduler_entries |
| Requests and nested queue/allocation keys | limits.flow.max_requests |
| Actors and actor-domain rows | limits.flow.max_actors |
| Work registration rows | limits.flow.max_works |
| Notifications | limits.flow.max_notifications |
| Registrations including nested callback IDs/stores/manifests | limits.flow.max_registrations aggregate |
| Builtin/context/restart component rows | limits.flow.max_component_rows aggregate |
| Corresponding sparse dimensions | limits.flow.max_sparse_slots aggregate |
| All registration/runtime/codec/factory keys | limits.flow.max_key_bytes aggregate |
| All context/restart payload bytes | limits.flow.max_payload_bytes aggregate |

Check every sum/conversion with overflow detection. Count mode rows in records
and bytes. Count Flush alongside gate commands in aggregate pending operations;
never derive this cap only from a cache or only from modes. Preserve accepted V1
and V2 pending/cap semantics. Check minimum record widths as lower bounds, not
fixed lengths; valid optional-empty records must remain readable. Exercise a
rich image whose relevant sums are positive. Distinguish u64::MAX causing
aggregate IntegerOverflow from count2 with cap1 causing LimitExceeded.

## Canonical validation and native restore boundary

Use the same common canonical policies as existing V2 wire for resource/request/
actor/work/registration/store/command/pending/scheduler collections, uniqueness
and nested queue/allocation ordering. Preserve flag exact coverage and derived
frontier checks. Mode rows must be sorted, unique and exactly match canonical
resource IDs including generations. Missing/extra/wrong/duplicate mode rows do
not default Eager. Flush is retained in the complete command vector.

Wire decode returns detached structural data. It does not certify a runtime,
preemption policy, parent admission frontier or complete composite checkpoint.
Native V3 restore owns full command/live scheduler membership, live Deferred
Flush target, kind/entity/time, unique resource/tick, pending Remove, mode/cache
and all work/context/restart joins. Test malformed join bytes that decode
structurally, then fail native restore before codec callbacks. Do not duplicate
the native restore engine to make wire decode seem stronger. Any reusable
structural validator must preserve the complete V3 state.

## Errors and precedence

Keep FlowCheckpointWireError exactly as it exists. V3 follows traversal order;
legacy V1/V2 error precedence is unchanged.

| Violation | Existing error |
| --- | --- |
| Unknown or opposite-version magic | InvalidTag |
| Unsupported u16 schema | UnsupportedSchema(actual) |
| Wrong u32 image version | InvalidValue("Flow checkpoint version") |
| Unknown command/mode/option/enum tag | InvalidTag |
| Malformed boolean | InvalidBoolean |
| Invalid key UTF8 | InvalidUtf8 |
| Checked arithmetic/conversion overflow | IntegerOverflow |
| Insufficient input/minimum rows | Truncated |
| Unconsumed trailing input | TrailingBytes |
| Byte/record/aggregate cap | LimitExceeded(existing label) |
| Failed bounded owning allocation | AllocationFailed |
| Common order/duplicate violation | InvalidValue("noncanonical order or duplicate entry") |
| Mode/resource coverage mismatch | InvalidValue("allocation modes") |

The mode coverage reason InvalidValue("allocation modes") is owner-reviewed
for V3; it uses the existing variant and introduces no new error type.
Existing V2 static reasons, including resource grant flags and started frontier,
retain their meanings. Header/primitive/count errors occur during scan in encoded
field order; trailing rejection completes before ownership. Bounded DTO canonical
checks follow. Encoder performs all measurement before allocating canonical sets.
Assert precise errors. V1/V2 readers and current router reject V3; V3 reader
rejects V1/V2. No decoder fallback.

## Required proof families

| Family | Mandatory positive and negative controls |
| --- | --- |
| Structural wire tags | A canonical detached DTO roundtrips/reencodes all tags0..13, Pause/Resume, true/false flags and both mode values. Label it WIRE ONLY: synthetic command combinations may be runtime-invalid and must not be credited as restore/continuation proof. |
| Rich runtime state | Independently build a VALID runtime with nonempty contexts/restart factory/template payloads, at least two actual queued keys AND at least two active allocation entries, mixed modes, gate and Flush commands. Explicit native capture -> V3 wire -> explicit native restore -> recapture equality and full suffix. Never use the structural tags DTO here. |
| Mixed interior cut | Both real completion insertion orders, cut after one tick6 completion with other readiness/Flush pending; restore then close admission and flush. Preserve low[1,4), urgent[4,6), B[6,7), low[7,14), no low zero-length resume/preempt6, exact useful/remaining/sample lineage. |
| Budget and current tick | Actual preflush halt, complete image/wire recapture and equal repeated halt; started-now pending Flush survives restore and same-tick valid readmission after committed Flush. No impossible progress requirement under unchanged budget. |
| Exact bounds | Both encode AND decode exact and cap+1 input controls for all positive aggregates, independently modes and combined four-kind pending count. Include nested registrations/factory keys/restart payloads and huge-overflow versus small-cap errors. |
| Before ownership | Malformed LAST mode tag and trailing bytes on the rich image reject via public V3 decode with owned-entry counter0; matching valid rich image reaches ownership and equals original. Assert at least two queue keys and two active allocation entries before whole-image reversal/duplication; use suitable resources/capacities. Never helper-only buffers. |
| Measurement | Late payload/byte limit rejects before allocating canonical validation with counter0; matching valid image enters canonical phase and reencodes identically. |
| Discriminators and malformed modes | Opposite/unknown magic, schema/version, all invalid primitive classes, truncation at representative boundaries, missing/extra/duplicate/stale/unsorted modes and wrong frontier; exact encoder/decoder error boundary with valid baseline. |
| Native joins and legacy parity | Malformed Flush scheduler kind/entity/resource/mode/Remove joins are encoded into input before decode and fail native restore pre-codec, valid codec control succeeds. Retain all old native/wire/goldens and full 19 legacy envelope literal digests plus accepted V2 process cases. |

Mutate proven schema offsets or unique full payloads; assert original IDs/counts.
Observe required transitions unconditionally with exact EventIds/full suffixes.
Freeze exact test names in the packet and match recorder names without suffixes.

## Bounded workflow and remaining acceptance

Bind a clean accepted native base and actual commands/packages separately.
One writer, early field/scanner review, three Rust1.99 gates, two failed gate
executions total, preserved attempts and frozen valid result precede independent
root/Sol review and commit. Protect legacy tests/manifests. All raw harness
output stays private and excluded from archives; gate receipts are evidence.

This document defines explicit wire scope only; no wire implementation acceptance. Routed views, V3 consumers, parent pin, ED
recovery/bindings and full delivery remain separate.
