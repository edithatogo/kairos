# ADR: native transit arrival handoff

Reviewed native design; runtime acceptance pending. Exact packet review and a fresh claim are required before dispatch. Root acts as Track03 scoped owner under user delivery authority with independent Sol source review. Source baseline Kairos 5c57dc807cf6a6524d174adbb88502a92cbf5673; Rust 1.99.0 only. Native handoff is the next bounded implementation; V2 persistence details below are a dependency design, not authority to implement it in the native packet. Source basis: `flow_adapter.rs` TransitContext::new at685, plan at779, emit_domain at960, emit_arrival at980, apply_receipt at990; V1 checkpoint DTO at132 and state validation at568; wire magic KTCXCPV1 in transit_checkpoint_wire.rs. Root is defining DES stable ordering separately. No dummy resource, premature fixed-staff Acquire, second clock or private calibration export.

## Proposed signatures

Keep existing constructor unchanged:
`TransitContext::new(&FlowRuntime, RoutePlan, FlowAcquireCommand, SimTime) -> Result<TransitContext, TransitError>`.

Add:
```rust
pub fn new_handoff(
    flow: &FlowRuntime,
    route: RoutePlan,
    service_work: WorkId,
    start: SimTime,
    progress_scheduler_priority: i32,
) -> Result<TransitContext, TransitError>;

pub fn observe_transit_arrival_handoff(
    flow: &FlowRuntime,
    carrier: WorkId,
    dispatch: &FlowDispatch,
) -> Result<Option<TransitArrivalHandoff>, FlowError>;

pub fn pending_transit_arrival_handoff(
    flow: &FlowRuntime,
    carrier: WorkId,
) -> Result<Option<TransitArrivalHandoff>, FlowError>;
```

`TransitArrivalHandoff` has private fields with copied getters: `carrier()->WorkId`, `service_work()->WorkId`, `owner()->EntityId`, `arrived_at()->SimTime`, `arrival_event()->EventId`. Include a private FlowRuntimeIdentity for live validation; never serialize that identity. Observations are idempotent, not claims. No arrival receipt constructor is public. The runtime's typed context, not an arbitrary caller-provided dispatch, is authoritative. Parent owns logical admission deduplication; native service binding also prevents duplicate work submission.

Use existing register_transit_context, create_actor_domain_context, schedule_transit_start and schedule_transit_control on the same Flow. Handoff constructor validates owner alive, same Flow identity, service work Pending/unbound, start>=flow.now and valid route. It does not require or manufacture ResourceId, request, deadline, can_preempt or priority_level. Queue urgency/ordering/selected staff are decided by parent at real admission after arrival.

## Internal state without legacy resource leakage

Refactor private acquire field into `TransitArrivalAction::Acquire(FlowAcquireCommand) | Handoff { progress_scheduler_priority:i32 }`. Keep service_work explicit. Progress scheduling reads its priority from the action. Legacy mode's validation still checks resource/owner/timed/acquire.at==start, exactly as before; Handoff validates only service work/owner/start and route. Do not build a fake Acquire with placeholder resource merely to reuse V1 validation.

Add a private persisted arrival witness `Option<{ event:EventId, at:SimTime }>` for Handoff only. During the candidate final Domain plan, when route remaining becomes0, record snapshot.delivery.id/time, clear expected_event/expected_due/next_progress_ticket and enter Arrived. Emit NO Acquire, no Notify and no follow-up arrival event. Candidate state is committed only through the existing accepted Flow domain-plan transaction. A rejected/budget-halted final plan leaves no committed arrival witness. Empty accepted domain batches must be covered by actual tests; do not assume the receipt application depends on an emitted command ticket.

`observe...` reads the actual context via flow.work_context::<TransitContext>(carrier), validates context.runtime==flow.identity, carrier/service owner/phase/witness and dispatch.event/at exactly, and returns None for non-arrival dispatches. It rejects wrong-runtime/wrong-carrier or matching-event malformed/error receipt rather than inventing arrival. `pending...` reads the same committed witness without needing an old in-memory dispatch, enabling an arrived-but-not-admitted restored cut. Both return None once service WorkSpec.request is Some; successful public submit binds the work before execution, preventing replayed handoff from creating a second claim. Preserve the historic witness for audit even after service admission.

Before constructing the first handoff claim, parent still enforces its logical ready/skill/calendar/space eligibility key. Native work remains the SAME WorkId through route and later service; Suspend does not create a replacement WorkId. Arrival does not start service or consume sampled work.

## Explicit checkpoint V2 boundary

Do not alter TransitContextCheckpointV1 fields or KTCXCPV1 bytes. Add `TransitContextCheckpointV2` and `TransitArrivalActionCheckpointV2::{Acquire{legacy_fields}, Handoff{progress_scheduler_priority}}`, plus optional handoff witness. Preserve existing route image/cursor, service_work, carrier/kind, start/frontier/phase/control state, pending event/ticket fields. V2 stores source EntityIds/EventId parts, never runtime identity.

Proposed methods: `TransitContext::checkpoint_v2(&FlowRuntimeIdentity, TransitContextCheckpointLimitsV1)->Result<TransitContextCheckpointV2, TransitContextCheckpointError>`; `checkpoint_bytes_v2`; DTO `encode_bytes_v2/decode_bytes_v2/restore_bytes_v2/restore_for_owner`. Use the explicit V2 codec helper frozen below. Existing V1 capture/decode and manifest version1 remain unchanged; no magic fallback or automatic cross-manifest migration is claimed. V1 capture rejects Handoff with the existing InvalidState error.

Future V2 payload validation before constructing owned TransitContext state (inside the registered ABM decoder; Flow cannot parse opaque ABM payloads before invoking that decoder): mode tag and exact schema; bounded route/key/payload sizes; legal cursor/phase; owner/service/carrier/kind joins; source identity on capture; no arrival Acquire or arrival_ticket in Handoff; legitimate Start/Progress command_ticket and next_progress_ticket remain valid while moving; witness only in committed Arrived with complete cursor and time==last_advanced; Handoff Pending/Moving/Paused has no witness; historical arrival EventId uses allocator-range validation, not live resolve_event (completed arrival is no longer pending). FlowCheckpointRebindV1::resolve_issued_event permits structurally valid historical events; it is not authenticity proof. Trusted outer capture/coherent frontier must establish witness provenance. Current Flow context codecs can store this V2 payload independently of the proposed DES FlowV4 ordering schema.

## Programme proof families and packet boundaries

- `transit_handoff_arrival_without_acquire`: actual positive route; no resource/request exists for service before or after arrival; WorkId unchanged; arrived at exact Flow clock; one receipt; no service progress consumed.
- `transit_handoff_select_after_arrival`: arrive while staff busy/unavailable, remain Pending/unbound; parent later selects real staff and submits same work exactly once; duplicate pending receipt/admission cannot create second request. No dummy-resource timer.
- `transit_handoff_dispatch_identity_rejects`: valid baseline then wrong Flow/carrier/event/time/error receipt controls; no context/work/request mutation.
- `transit_handoff_pause_resume_same_clock`: real controls before/mid-route and at frontier; actual route cursor/state retained; no extra service draw.
- `transit_handoff_budget_halt_no_arrival`: exhausted budget before final Domain; exact repeated halt, complete pending state, no witness/admission. Restored-cut repetition is a later V2 persistence proof.
- **Later V2 persistence packet only:** `transit_handoff_v2_interiors_continue`: Ready, Moving, Paused and Arrived-but-unadmitted captures/restores; source/fresh restored suffix and witness identity equivalent; later one real service admission.
- `transit_handoff_v1_legacy_parity`: old constructor/Acquire behavior and literal V1 bytes unchanged; Handoff V1 capture rejects before caller codecs.
- **Later V2 persistence packet only:** `transit_handoff_v2_malformed_precodec`: invalid action tag, witness phase/time/history bounds, owner/carrier/work joins, arrival-ticket/Acquire contamination and size limits fail before context ownership inside the ABM payload decoder; restoration atomicity and valid positive control.

## Ownership, sizing and trigger

ABM owner paths are flow_adapter.rs, new TransitV2 native/wire support within ABM, bounded ABM integration tests and owner documentation. DES core/ordering/V4 and private calibration stay protected. Explicit API ADR and compatibility template precede dispatch. Separate packets: native Handoff state/observer tests; V2 native+wire/registration; combined owner qualification and downstream coherent/portable consumer review. ABM native Handoff can target existing public Flow and run independently of DES stable-ordering implementation after shared service/receipt/codec contract freeze. Parent full adapter binding waits for accepted joined APIs and separate pin update. Preserve all failed receipts; do not claim V3 consumers automatically accept new TransitV2 payloads.

## Revised arrival-trigger and receipt decisions

Thread the actual `snapshot.delivery.id` and `snapshot.delivery.at` through BOTH planner paths. Domain progression reaches `schedule_next_or_arrive` then `emit_arrival`; DomainControl Resume from Paused/Moving with zero remaining calls `emit_arrival` directly around flow_adapter.rs:912. The private signature should become `emit_arrival(&mut self, delivery: &FlowCallbackSnapshot, at: SimTime, sink: &mut FlowCommandSink)` (or exact EventId/time arguments from that same snapshot), and both call paths must pass that delivery. Never reconstruct the witness from expected_event, because Pause may consume/clear that prior event and Resume itself can be the accepted arrival event.

Handoff witness includes actual triggering EventId/time and domain cause: Domain(kind) or DomainControl(kind, Resume). Pause alone does not finalize handoff even when it reaches the route frontier; a later accepted Resume can finalize it. Existing legacy Acquire behavior remains unchanged. The observer accepts matching committed Domain OR Resume control arrival dispatch. A wrong Flow/carrier is InvalidWork. An unrelated event returns Ok(None), regardless of unrelated receipt error. A matching witness event with mismatched time, dispatch.error or rejected callback batch returns Err(FlowError::InvalidWork) without state mutation. A caller can construct FlowDispatch publicly, so the observer does not claim cryptographic receipt authenticity: it verifies the real authoritative typed context witness and structural receipt. `pending...` intentionally obtains the same committed arrival from state without an old dispatch.

Source confirms the empty Accepted callback batch applies and accepts staged context in Flow (flow.rs:2910-2945). Handoff final arrival therefore requires no emitted command ticket. Assert actual `Accepted(empty)` on both Domain and Resume paths; budget or planner rejection must not commit candidate phase/witness. Existing apply_receipt can remain a no-op for Handoff's absent command_ticket.

After restoring an Arrived/unadmitted V2 context, `pending...` returns a new live-runtime receipt whose private identity equals the restored Flow. Service WorkSpec remains Pending with requestNone. Parent selects actual named staff and uses the same WorkId with a direct keyed acquire builder. Successful builder submission binds WorkSpec.request immediately; subsequent pending observation returnsNone and a duplicate direct work submission rejects. No mutation of transit history/witness is needed to acknowledge service admission. Restored Arrived/admitted state retains the historical witness but pending accessor returnsNone.

## Frozen error and codec-version choices

Handoff `checkpoint_v1` returns the EXISTING `TransitContextCheckpointError::InvalidState` before route encoding or caller codec work. `checkpoint_bytes_v1` maps that through its existing wire/native error wrapper. Add no public error variant for this incompatibility; documentation explains that V1 cannot encode Handoff. Invalid new_handoff service/start/route state uses existing TransitError::InvalidProgress consistently with the existing constructor. Observer validation uses existing FlowError::InvalidWork.

Propose this exact new helper, with codec manifest version2 and explicit V2 wire magic:
```rust
pub fn register_transit_context_checkpoint_codec_v2(
    codecs: &mut FlowCheckpointCodecs,
    codec_key: impl Into<String>,
    source_identity: FlowRuntimeIdentity,
    trusted_graph: Arc<TransitGraphV1>,
    limits: TransitContextCheckpointLimitsV1,
) -> Result<(), FlowCheckpointError>;
```

It registers TransitContext with owner under version2; capture emits KTCXCPV2 and decoder accepts only V2, without fallback. Existing bounds fields apply to FULL V2 encoded size, including witness/action payload. Existing helper remains manifest version1 and exact V1-only capture/decode. Legacy callers need no edits; explicit V1 restore normalizes Acquire. A version-neutral payload dispatcher may be added later only under a reviewed consumer contract. Recognizing V1 magic in a version2 decoder would not solve an old Flow manifest's version1 registration check, so do not claim automatic old-image migration. New callers use a distinct reviewed codec key for V2 or choose the matching helper for their manifest; no dual same-key registration is assumed.

New native DTO restore/restore_for_owner validates all references against the actual rebind. Historical witness event uses `resolve_issued_event`, not `resolve_event`; the latter requires a live pending row. Validate source owner/context coherent frontier via trusted outer state, because issued-event validation alone is structural. V2 mode-specific validation does not read or fabricate Acquire resource fields for Handoff.

Add native proof variants for Resume-at-arrival frontier, matching errored forged receipt rejection and unrelated errored receipt returningNone. The later V2 persistence packet adds arrived/unadmitted restored service binding and arrived/admitted restored no-second-handoff. These are explicit prospective oracles, not executed evidence.

## API governance and native boundary

Protected Rust root crates/kairo-ecs-abm, rust_api, experimental alpha. Existing TransitContext fields are private; preserve constructor signatures, TransitContextCheckpointV1 layout, TransitContextCheckpointError variants, V1 wire bytes and existing codec registration. Additive native constructor, readback receipt type and functions do not require legacy caller edits. C ABI include/kairo_ecs.h, Arrow schemas/arrow/event_log_v1.schema.json and host APIs are unchanged. No root rename, binding promotion or beta/stable promise. Returned receipt owns copied IDs/time plus a private live-runtime identity; state access borrows Flow immutably, no new shared mutation or concurrency promise.

Native-stage acceptance includes actual route and control dispatches, immutable service WorkId, zero premature Acquire, atomic handoff witness, one later service admission, wrong identity/error controls, budget halt and legacy parity. Existing V1 capture rejects Handoff with InvalidState before encoding. V2 DTO/wire/registration is a separately reviewed packet; no restored Handoff acceptance can be claimed at native stage. The eight proof families above are programme-level: native source packet binds native cases; V2 interior/malformed/restore cases bind only with V2 implementation.

A reviewed source packet may own flow_adapter.rs, lib.rs re-exports if needed, one new native integration test and a behavior note. Coordinator owns ADR/handoff freeze. At most three failed development executions, all receipts preserved; stop for required out-of-scope changes or any final qualification failure. Exact commands and protected hashes bind at dispatch. Parent integration and pin wait for accepted native/persistence/consumer joins. No clinical, empirical, hardware or release acceptance follows.

V2 preownership validation does not claim to run before Flow invokes the opaque registered context decoder. The decoder owns ABM structural validation. The separate native V1 capture incompatibility guard runs before route encoding; outer precodec guarantees are claimed only where the actual Flow capture path supplies them.
