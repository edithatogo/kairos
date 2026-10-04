# ADR: validated optimistic message codec bridge

Status: architecture and independent red-team roles accepted for bounded alpha implementation on 4 October 2026. Track48 remains In Progress. This is a Track48 prerequisite, not Track49 production dispatch or distributed acceptance.

## Problem and decision

At merged PR199 source `786f50b97bf9149b4d4a39210398d4266ae4ab2a`, native messages expose immutable event fields and root identity parts but cannot expose a complete Output parent/ordinal or reconstruct a checked inbound message. Add exactly three methods under the existing `time-warp` feature:

- `LogicalEventId::output_parts(&self) -> Option<(&OptimisticEventOrderKey, u32)>`. Borrow the immutable complete parent and return the exact output ordinal; roots return None.
- `OptimisticEventOrderKey::try_from_parts(tick: Tick, source_lp: LpId, logical_id: LogicalEventId) -> Result<Self, OptimisticError>`. Validate the complete bounded ancestry before returning an ordering key.
- `OptimisticMessage::try_from_parts(event: RemoteEvent, logical_id: LogicalEventId, incarnation: u64, kind: OptimisticMessageKind) -> Result<Self, OptimisticError>`. Validate using the same key constructor, then preserve all supplied fields without rewriting or allocation of authority.

Retain private unchecked constructors for existing trusted internal paths. Do not change scheduling, ordering, RNG, storage, replay or existing error semantics. Use existing EnvelopeSourceMismatch, OutputNotStrictlyFuture and CausalDepthExceeded errors.

## Validation and ownership

Walk the full borrowed ancestry with a hard depth128 bound. Root ordering-key source must equal its root source; every Output key tick must be strictly greater than its complete parent key tick. Output actual emitter may differ from root origin and parent emitter. Do not require the output emitter to equal either ancestor. Reject invalid data before producing a message or mutating runtime membership.

Native Tick is SimTime/u128: preserve its full range without floating point, truncation or a u64 fixture-profile restriction. Preserve u32 LPs/ordinal, full u64 sequence/incarnation including zero/MAX, exact payload bytes, destination and message kind. Incarnation and a future transport authority epoch never enter logical ordering.

Use existing root/checked child construction bottom-up; no public mutable recursive DTO or serialization dependency. The Track49 decoder separately owns bounded raw-byte preflight, duplicate fields, topology/authorization, wire-range negotiation and persistence. Runtime receive retains topology, routes, configured depth/capacity, GVT, known-delivery conflict and duplicate semantics. A reconstructed message provides no durable admission, source authentication, restart fencing, receipt or distributed GVT guarantee. Native cancellation identity omits authority epoch: a transport must not merely strip an outer epoch and admit colliding old/new epochs into this native namespace. Track48/49 must separately review epoch integration or a proven globally non-reused native identity with restart/migration fencing before production admission.

## Compatibility and evidence

This is an additive opt-in alpha Rust API in `crates/kairo-ecs-pdes`; no ABI, Arrow, binding, shared fixture or dependency change. Existing constructors/getters remain unchanged. Update the PDES preview contract and append API compatibility review with the actual reviewer dispositions before acceptance. Add crate integration conformance fixtures, including native full-range and structural roundtrip cases. No stable/publication approval is inferred.

Behavioral acceptance: root and multi-emitter Output roundtrip including positive/anti and all bytes; nested parent/ordinal inspection; root source mismatch rejection; equal/backward Output ticks rejected; depth128 accepted/129 rejected; full u128 tick and u64 sequence/incarnation preserved; ordering unchanged by incarnation/kind; reconstruct and actually receive valid messages; duplicate/conflicting metadata rejection leaves report/state unchanged. Existing local runtime regression lane must pass.

Architecture role inspected current source and accepted this narrow bridge over a public DTO, requesting complete bounded ancestry validation. Independent hostile-input role accepts the bridge and requires the explicit native epoch namespace blocker retained above. Neither reviewer authorizes Track49 production admission. Implementation requires a separately reviewed exact-base packet, isolated writer claim, native toolchain binding, source/command/log receipts and independent review.
