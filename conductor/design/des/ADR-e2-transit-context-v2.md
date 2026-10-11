# ADR: TransitContext native checkpoint V2

**Status:** Owner design reviewed by root and independent Sol reviewer on 12 October 2026. Native join `ef20ae54b73dc47d59250c9b073c063311704413` is qualified; final result SHA-256 `ae8285021fa7c9ee3adc3e23b587048d58f946db66a21f214f5b391b927b42c6`.

**Scope:** Freeze the V2 owner image contract and serial native-to-wire implementation boundary. Source execution requires a separately bound, reviewed packet.

## Context and source basis

Native arrival handoff is implemented at Kairos HEAD `0b004141b5a727ae601d2315cebb6c3732e6fac4`; source and joined native qualification have root/Sol acceptance. Current native state is in `crates/kairo-ecs-abm/src/flow_adapter.rs:253-347`; V1 DTO/capture/restore are at `:119-160`, `:353-442`, `:444-620`; current state checks are at `:630-679`. V1 byte transport and manifest-1 codec are in `crates/kairo-ecs-abm/src/flow_adapter/transit_checkpoint_wire.rs:5-95,98-279,630+`. The existing V1 regression source is `crates/kairo-ecs-abm/tests/transit_context_checkpoint_v1.rs`.

The accepted native design `conductor/design/des/ADR-e2-transit-arrival-handoff.md:46-52,79-97` already requires a separate V2 boundary, KTCXCPV2, a version-2 context codec helper, no V1 fallback, historical issued-event validation, and V2-only restored-continuation/malformed-proof families. This contract specifies and freezes the native typed DTO fields and API signatures; wire framing remains a later packet.

## Decision: preserve V1, add a mode-aware V2 DTO

1. Do not change `TransitContextCheckpointV1`, `TransitCommandPurposeCheckpointV1`, the existing `register_transit_context_checkpoint_codec` version-1 registration, KTCXCPV1 bytes, or V1 behavior. The current `checkpoint_v1` rejects Handoff with existing `TransitContextCheckpointError::InvalidState` before route encoding (`flow_adapter.rs:353-363`). V1 restore normalizes to the legacy Acquire mode only.
2. Add a distinct typed `TransitContextCheckpointV2` and `TransitArrivalActionCheckpointV2`. The action must be explicit; Handoff must never be encoded using a dummy `ResourceId`, Acquire request, deadline, or priority-level surrogate.
3. Store source entity/event IDs as IDs/parts only. Never store `FlowRuntimeIdentity`, pointers, closures, or live runtime objects.
4. Preserve the common route image/cursor, service work, carrier and event kind, start/frontier/phase/control state, and event/ticket continuation data. Preserve the historical arrival witness after service admission.

### Typed fields

The following names and fields are the native API contract:

```rust
pub enum TransitArrivalActionCheckpointV2 {
    Acquire {
        resource: EntityId,
        owner: EntityId,
        work: Option<EntityId>,
        at_ticks: u128,
        priority_level: i32,
        deadline_ticks: Option<u128>,
        scheduler_priority: i32,
        timed: bool,
        can_preempt: bool,
        preemptible: Option<PreemptionStrategy>,
    },
    Handoff {
        progress_scheduler_priority: i32,
    },
}

pub struct TransitArrivalWitnessCheckpointV2 {
    pub event: (u64, u32),
    pub at_ticks: u128,
    pub cause: TransitArrivalCauseCheckpointV2,
}

pub enum TransitArrivalCauseCheckpointV2 {
    Domain,
    Resume,
}

pub struct TransitContextCheckpointV2 {
    pub schema_version: u32, // exactly 2
    pub route: RoutePlanImageV1,
    pub segment_index: usize,
    pub elapsed_in_segment_ticks: u128,
    pub service_work: EntityId,
    pub service_owner: EntityId,
    pub action: TransitArrivalActionCheckpointV2,
    pub arrival_witness: Option<TransitArrivalWitnessCheckpointV2>,
    pub carrier: Option<EntityId>,
    pub kind: Option<EventKind>,
    pub start_at_ticks: u128,
    pub phase: TransitPhase,
    pub paused_from: Option<TransitPhase>,
    pub last_advanced_at_ticks: u128,
    pub initial_start_pending: bool,
    pub expected_event: Option<(u64, u32)>,
    pub expected_due_ticks: Option<u128>,
    pub command_ticket: Option<(u64, usize, TransitCommandPurposeCheckpointV1)>,
    pub arrival_ticket: Option<(u64, usize)>,
    pub next_progress_ticket: Option<(u64, usize)>,
}
```

`service_owner` is explicit common state so Handoff can retain the current private `TransitContext.service_owner` without inventing Acquire fields. For Handoff restore, require a current `resolve_work_binding(service_work)` row and require its owner to equal `service_owner`; historical `resolve_work`/`resolve_work_owner` is insufficient. For Acquire, preserve V1's historical-resolution behavior: `action.owner` must equal `service_owner`, and its work must equal `service_work`, but do not silently narrow legacy semantics. Reuse `TransitCommandPurposeCheckpointV1` because Start/Progress/Arrival meanings already match the private runtime purpose. Cause `Domain` versus `Resume` preserves witness provenance; it is not a second event source or independently authenticated dispatch cause.

The V2 action discriminator and witness representation are additive. No V1 field is removed, renamed, or reinterpreted.

## Native API (Packet A only)

Mirror existing V1 methods without introducing wire functionality:

```rust
impl TransitContext {
    pub fn checkpoint_v2(
        &self,
        source_identity: &FlowRuntimeIdentity,
        limits: TransitContextCheckpointLimitsV1,
    ) -> Result<TransitContextCheckpointV2, TransitContextCheckpointError>;
}

impl TransitContextCheckpointV2 {
    pub fn restore(
        &self,
        graph: &TransitGraphV1,
        rebind: &FlowCheckpointRebindV1,
        limits: TransitContextCheckpointLimitsV1,
    ) -> Result<TransitContext, TransitContextCheckpointError>;

    pub fn restore_for_owner(
        &self,
        graph: &TransitGraphV1,
        rebind: &FlowCheckpointRebindV1,
        row_owner: EntityId,
        limits: TransitContextCheckpointLimitsV1,
    ) -> Result<TransitContext, TransitContextCheckpointError>;
}
```

Names are the existing ADR’s `checkpoint_v2` plus direct typed equivalents of `TransitContextCheckpointV1::restore` and `restore_for_owner`. The accepted ADR also proposes `checkpoint_bytes_v2`, `encode_bytes_v2`, `decode_bytes_v2`, `restore_bytes_v2` and `register_transit_context_checkpoint_codec_v2`; defer all of those to Packet B. Packet A must not edit `transit_checkpoint_wire.rs` or claim byte-level/codec support.

## Restore validation and mapping order

Perform all checks before constructing/returning the restored `TransitContext`. Map failures to existing native errors; add no error variant in this contract:

1. Require image schema version 2; unsupported image/route versions map to `UnsupportedVersion(version)`.
2. Check route segment, graph-byte, mode-byte and total-image budgets with checked arithmetic before route/image clones; overflow or any configured bound maps to `LimitExceeded`. Validate route image version and route structure; map to existing `InvalidRoute` or `InvalidProgress` using current route-error mapping (`flow_adapter.rs:681-686`).
3. Check action-specific structural invariants and phase/cursor combinations. Contradictions map to `InvalidState`.
4. Resolve the serialized service identity. For Acquire, use `resolve_work`, `resolve_work_owner`, and `resolve_actor` as the current V1 path does; these APIs accept validated historical references, so do not call this a liveness check. For Handoff, require `resolve_work_binding(service_work)` to return a current WorkSpec binding and require its owner == serialized `service_owner`. This current-row check establishes the supported live service binding; do not infer stronger role/kind restrictions. Unknown/mismatched references map to `InvalidReference`.
5. For Acquire, resolve its resource/owner/work using `resolve_resource`, `resolve_actor`, and `resolve_work`; require action owner == service owner, action work == service work, timed=true, and Acquire time == `start_at_ticks`, preserving V1 historical-reference behavior. For Handoff, resolve no resource/request fields and do not build a placeholder `FlowAcquireCommand`. The owner rebind exposes no WorkProgress or WorkSpec.request getter, so this DTO cannot prove that the service is pending/unadmitted at the outer Flow cut; make no such claim. Outer Flow/coherent-cut policy is responsible for any cross-state admission check, and readback suppression is evaluated against the actual restored WorkSpec.
6. For a bound carrier, resolve work and `resolve_work_binding(carrier)`; `restore_for_owner` additionally requires carrier ID == `row_owner`, row is actor-domain, and row `EventKind` equals saved kind. `resolve_work_binding` returns owner/registration/optional domain kind (`kairo-ecs-des/src/flow/checkpoint.rs:395-403`). `restore_for_owner` already has this owner-row pattern (`flow_adapter.rs:590-620`). Invalid joins map to `InvalidReference`. Carrier and kind must be both absent only for an allowed Ready shape; Moving, Paused, and Arrived require their valid bound carrier/kind according to the state table below.
7. Resolve `expected_event` and Handoff witness event via `resolve_issued_event`, not `resolve_event`: current helper accepts allocator-issued historical EventIds, including dispatched/cancelled events (`kairo-ecs-des/src/flow/checkpoint.rs:438-449`); `resolve_event` only accepts current pending scheduler entries (`:430-436`). Invalid event identity maps to `InvalidReference`.
8. Resolve each serialized ticket with `resolve_ticket(batch,index)`; failure maps to `InvalidTicket` (`checkpoint.rs:452+`). Preserve Start/Progress ticket purposes and pending event/due joins exactly; do not synthesize fresh scheduler IDs. This contract captures only a public committed cut after successful command acceptance, when `command_ticket` has been consumed and is therefore `None`; reject a DTO with a non-None command ticket. `next_progress_ticket` is retained accepted native state and is distinct from the consumed staging ticket.
9. Restore route against caller-approved `TransitGraphV1`, restore cursor, run action-aware phase validation, and only then construct the context with the new `rebind.identity()`. The owner decoder returns a detached context; failed restore publishes no Flow runtime/context and leaves source/caller state unchanged.

### Action and phase rules

- `Acquire`: `arrival_witness` is None. Preserve legacy V1 parity, including the real acquire tuple and Arrival ticket behavior.
- `Handoff/Ready`: no witness and initial cursor. Before the first callback, carrier/kind may both be None; unscheduled Ready has no expected event/due, while a scheduled Start has both. A resumed Ready can have carrier/kind bound, `initial_start_pending == false`, no `next_progress_ticket`, and a newly accepted Start event/due at `max(now, start_at)`; do not require due == original start. `command_ticket` remains None at the committed cut.
- `Handoff/Moving`: no witness; cursor incomplete; carrier/kind bound; a scheduled Progress event and due are paired and `next_progress_ticket` is present. Preserve exact IDs/times. `command_ticket` remains None at the committed cut.
- `Handoff/Paused`: no witness; `paused_from` only Ready or Moving. Paused-from-Ready has initial cursor. Paused-from-Moving may have a complete cursor when Pause reaches the route frontier; it remains unarrived until accepted Resume. A paused cut may retain its old pending event/due and next-progress ticket, or may have consumed that stale event and cleared all three; both forms must be supported. Never infer a witness from an expected event.
- `Handoff/Arrived`: witness required, cursor complete, `witness.at_ticks == last_advanced_at_ticks`, carrier/kind bound, context expected event/due absent, `next_progress_ticket` absent, `command_ticket` absent, and no Acquire/arrival ticket. Do not reject an old LIVE Domain Progress event that still targets the carrier: when Pause reaches the frontier and higher-priority Resume arrives before that event dispatches, arrival clears context-owned pending state, while the old Flow event remains live and later dispatches as a no-op. Preserve this exact Resume-witness cut and prove its restored no-op suffix. A restored Arrived witness persists when service work is already admitted; observers suppress handoff readback against the actual restored WorkSpec.
- All modes: expected event and expected due are either both Some or both None. `paused_from` is present iff phase is Paused; last-advanced time cannot precede start. Captures are committed accepted-plan cuts, not in-flight command staging states. Reuse existing route/cursor checks while splitting away V1-only `acquire_timed`/Acquire-time requirements (`flow_adapter.rs:630-679`).

## Provenance limit and outer trust

Mechanical joins available to the owner codec differ by purpose: `resolve_work`, `resolve_work_owner`, `resolve_actor`, and `resolve_resource` can validate current or accepted historical references; `resolve_work_binding` is the current WorkSpec-row join used to establish a live Handoff service binding or carrier binding; `resolve_event` checks a currently pending event while `resolve_issued_event` checks only allocator-issued identity; and `resolve_ticket` checks allocator/index membership. `FlowRuntime::actor_domain_context(actor)` validates the live actor-to-carrier relation and context presence (`kairo-ecs-des/src/flow.rs:1905-1919`), but `FlowCheckpointRebindV1` does not expose a historical dispatch/cause ledger or WorkProgress/WorkSpec.request state.

Therefore a correctly resealed V2 owner payload can be structurally checked for an issued event, phase, cursor and matching witness time, but the owner codec cannot prove from these APIs alone that this historical event actually dispatched or caused the recorded arrival. `resolve_issued_event` is not authenticity evidence. Follow the accepted ADR’s boundary: trusted outer capture/coherent-cut handling is responsible for witness provenance; the ABM payload decoder performs structural validation. The contract must name that trust assumption and must not describe a checksum, inner byte parser, or allocator-range check as a historical authenticity proof. Also state that Flow sees ABM payload bytes as opaque until it invokes the registered context decoder; “malformed precodec” means before constructing/owning the ABM TransitContext, not before calling that opaque decoder.

## Bounds and error taxonomy

Reuse the configured `TransitContextCheckpointLimitsV1` fields `max_segments`, `max_graph_bytes`, `max_mode_bytes`, `max_total_bytes` (`flow_adapter.rs:27-84`); do not silently increase them. The eventual V2 wire encoder must count the complete payload, including mode/action/witness and all route data, against `max_total_bytes`. Current native `check_context_size` estimates a fixed 512-byte envelope; exact V2 serialized-size proof belongs in Packet B. Packet A still bounds route arrays/graph/mode before allocation/copy and uses checked arithmetic for typed-image estimates.

Keep native error variants `UnsupportedVersion`, `LimitExceeded`, `LineageMismatch`, `InvalidState`, `InvalidReference`, `InvalidRoute`, `InvalidProgress`, `InvalidTicket` (`flow_adapter.rs:86-98`). Packet B preserves `TransitContextWireError` classes (`UnsupportedVersion`, `LimitExceeded`, `InvalidData`, `InvalidUtf8`, `Truncated`, `TrailingBytes`, `AllocationFailure`, `Native`; `transit_checkpoint_wire.rs:9-48`). Do not add public error variants for V2 mode incompatibility.

## Serial ownership and qualification

**Packet A (native DTO only)** owns `crates/kairo-ecs-abm/src/flow_adapter.rs`, `crates/kairo-ecs-abm/src/lib.rs`, and new `crates/kairo-ecs-abm/tests/transit_context_checkpoint_v2_native.rs`. No wire file, byte methods, codec helper, or KTCXCPV2 claim. Prove typed DTO capture/restore for unscheduled and scheduled Ready, resumed Ready with due later than original start, Moving, both retained-event and consumed-stale-event Paused cuts, Arrived-unadmitted, Arrived-admitted suppression, and the frontier Resume arrival with its still-LIVE old Domain event followed by restored no-op dispatch. Include Handoff restore with a historical-only service-work reference as an `InvalidReference` control; retain Acquire historical-reference behavior and unchanged literal V1 behavior. Prove uninterrupted/restored suffix equality and service binding once. Do not claim capture universally rejects an orphaned Handoff: `checkpoint_v2` receives identity only and cannot inspect whether the service WorkSpec is currently live; such a DTO may be structurally captured, but Handoff restore must fail `InvalidReference` without a current `resolve_work_binding(service_work)`. The absent WorkProgress/request cross-check remains an outer-policy responsibility. Freeze/review before Packet B.

**Packet B (wire/codec, after Packet A is accepted)** owns `crates/kairo-ecs-abm/src/flow_adapter/transit_checkpoint_wire.rs`, additive API bridge/re-exports in `flow_adapter.rs` and `lib.rs`, and new `crates/kairo-ecs-abm/tests/transit_context_checkpoint_v2_wire.rs`. The codec helper uses a distinct manifest version 2 and exact KTCXCPV2 magic/schema 2; its decoder accepts only V2. Existing version-1 helper remains V1-only; no V1 fallback or same-key dual-manifest assumption. Prove canonical bytes, full-payload bounds, malformed/truncated/trailing/preflight cases, registered full-Flow restoration and outer-sealed malformed inner payload behavior. No Flow/DES source ownership.

Both are separate reviewed claims/gates; no parallel dispatch because both stage through `flow_adapter.rs`/`lib.rs`. Only after V2 wire/combined qualification should calibration coherent/portable consumers bind to it. No parent pin or ED claim follows from either packet alone.
