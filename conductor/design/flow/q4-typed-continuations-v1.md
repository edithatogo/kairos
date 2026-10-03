# Q4 typed continuations and domain hooks — experimental contract v1

Status: accepted by bounded internal Track 03/25 coordinator review for fixture preparation. This is not external maintainer approval, release acceptance or runtime implementation authority; implementation remains separately gated. Child base 358bb156c638e3bc5a67129e306ee0ae65d6f61d. No source implementation, acceptance, publication or ABI promotion. Track 03 owns DES integration; Track 25 reviews experimental source compatibility. Track 04 telemetry and shared ABM adapters are separate joins.

## Exact proposed surface

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FlowCallbackConfig {
    pub max_callback_commands: std::num::NonZeroUsize,
}
pub struct FlowContinuations<C> {
    pub on_resume: Option<fn(&mut C, &FlowCallbackSnapshot, &mut FlowCommandSink)>,
    pub on_restart: Option<fn(&mut C, &FlowCallbackSnapshot, &mut FlowCommandSink)>,
    pub on_abort: Option<fn(&mut C, &FlowCallbackSnapshot, &mut FlowCommandSink)>,
    pub on_cancel: Option<fn(&mut C, &FlowCallbackSnapshot, &mut FlowCommandSink)>,
    pub on_complete: Option<fn(&mut C, &FlowCallbackSnapshot, &mut FlowCommandSink)>,
}
pub struct FlowCallbackSnapshot {
    pub delivery: ScheduledEventPreview,
    pub origin: EventId,
    pub origin_ordinal: Option<u32>,
    pub work: WorkId,
    pub cause: FlowCallbackCause,
}
pub enum FlowCallbackCause {
    Work { transition: LifecycleTransition, progress: WorkProgress },
    Domain { kind: EventKind },
}
// Opaque, Copy, Eq tickets; no public constructor or integer conversion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FlowCommandTicket { /* private batch identity + emission index */ }
pub enum FlowRequestRef { Existing(RequestId), Submitted(FlowCommandTicket) }
pub struct FlowAcquireCommand {
    pub resource: ResourceId, pub owner: EntityId, pub work: Option<WorkId>,
    pub at: SimTime, pub priority_level: i32, pub deadline: Option<SimTime>,
    pub scheduler_priority: i32, pub timed: bool,
    pub can_preempt: bool, pub preemptible: Option<PreemptionStrategy>,
}
pub enum FlowOwnedCommand {
    Acquire(FlowAcquireCommand),
    Release { lease: LeaseId, at: SimTime },
    Cancel { request: FlowRequestRef, at: SimTime, scheduler_priority: i32 },
    Reprioritize { request: FlowRequestRef, level: i32, at: SimTime,
                   scheduler_priority: i32 },
    Domain { work: WorkId, kind: EventKind, at: SimTime, scheduler_priority: i32 },
}
impl FlowCommandSink {
    pub fn emit(&mut self, command: FlowOwnedCommand) -> Result<FlowCommandTicket, FlowError>;
}
pub struct FlowCommandAdmission {
    pub ticket: FlowCommandTicket,
    pub event: EventId,
    pub request: Option<RequestId>,
    pub deadline_event: Option<EventId>,
}
pub struct FlowBatchRejection { pub failed_ticket: Option<FlowCommandTicket>, pub error: FlowError }
pub enum FlowBatchReceipt {
    Accepted(Vec<FlowCommandAdmission>),
    Rejected(FlowBatchRejection),
}
// New field on FlowDispatch: pub callback_batches: Vec<FlowBatchReceipt>.
impl FlowRuntime {
    pub fn with_configs(flow: FlowConfig, callbacks: FlowCallbackConfig) -> Self;
    pub fn register_work_continuations<C: 'static>(&mut self, registration: &str,
        continuations: FlowContinuations<C>) -> Result<(), FlowError>;
    pub fn register_domain_hook<C: 'static>(&mut self, registration: &str,
        kind: EventKind,
        callback: fn(&mut C, &FlowCallbackSnapshot, &mut FlowCommandSink))
        -> Result<(), FlowError>;
    pub fn schedule_domain(&mut self, work: WorkId, kind: EventKind,
        at: SimTime, scheduler_priority: i32) -> Result<EventId, FlowError>;
}
```

Registration uses existing WorkSpec context_type_key and requires a nonempty registration. Every registration operation checks concrete TypeId against ALL existing legacy handler, continuation and domain descriptors for that registration, regardless of which descriptor kind is being added. The same C may register each descriptor kind before its first work; reject only duplicates of the same descriptor kind/key, or a cross-descriptor type mismatch. Legacy key is registration, continuation key is registration, domain key is (registration, kind). Register before creating that registration's first work; no replacement/unregistration API. Domain key is (registration, kind), so a kind can have different typed handlers for different registrations. Registration itself never creates a world, actor, work or context. Existing create_work/create_restartable_work remains authoritative. A domain event refers to live existing work and borrows its existing owned C; no payload codec or arbitrary registry access. Reserved EventKind custom4000–4003 is rejected at registration, schedule_domain and batch validation. Add precise FlowError variants ReservedEventKind, UnregisteredDomainEvent, InvalidCommandTicket and CallbackBatchLimitExceeded; unknown ordinary domain registration is explicit error rather than ignored event.

FlowContinuations<C> implements Default with all five fields None and no C: Default bound. FlowCommandTicket has no Default, public constructor or integer conversion. FlowCommandSink has no Clone, Default or public constructor; only runtime delivery creates it. These restrictions prevent fabricated identities and retained independent sink copies.

## Delivery and causal rules

Legacy WorkHandlers<C> is unchanged, including its four fields/signatures. New continuations are separate descriptors. If both select the same transition, enqueue two distinct4003 tokens, legacy first then continuation, in deterministic transition/registration order. Each live delivery costs one budget transition; invalidated tokens cost zero and remain visible no-ops. Completed queues only new on_complete, creates no delivery lifecycle row. Original lifecycle origin EventId and contiguous ordinal are captured at the transition, not reconstructed from final state. Domain delivery uses its own scheduled EventId as origin and origin_ordinal=None; no fabricated lifecycle ordinal or row. Its delivery costs one only when actually delivered. Captured work progress is immutable; C is live owned context and may differ from the origin snapshot.

No callback receives runtime, scheduler, registry, world, arbitration or step/run capability. All callbacks are function pointers, safe type-erased bridges through ComponentStore::get_mut, never invented ComponentRegistry::get_mut. Completion retains work/context after terminal state under current lifecycle policy, enabling acquisition for a different already-created work. The terminal completed work itself remains associated and cannot be resubmitted.

A missing/destroyed work/context invalidates a deferred notification as zero-cost empty delivery under existing Q3 guards. A scheduled domain event whose handler was never registered is an explicit semantic dispatch error; no handler executes and no lifecycle rows are fabricated. With immutable validated public registrations, this scheduled state is unreachable through public ingress; its oracle must use private fault injection, never a fabricated public scenario. Registration cannot vanish after admission. Invalid live handle admission rejects before event allocation.

## Whole-batch admission

Sink is an append-only owned vector for this invocation, with inaccessible batch identity. A private u64 batch-identity counter is monotonic and never wraps or reuses an identity. Before consuming the delivery head or invoking any callback, pure delivery preflight checks that the next identity exists using checked_add; exhaustion returns CounterOverflow with the pending head and all counters/context untouched. Only an admitted actual continuation/domain callback consumes that identity; rejected batches retain their consumed identity because the callback was delivered. Stale/legacy deliveries consume none. One new restricted sink is issued per callback invocation, never shared between callbacks. emit allocates only a batch-local ticket, never a request/entity/event. A Submitted reference must identify an earlier Acquire in the same invocation. emit validates no ticket references: it only issues a bounded ticket and appends the owned command. After the callback returns, forward, foreign, non-Acquire or otherwise unresolvable references reject the whole batch with InvalidCommandTicket and failed_ticket=Some(the already-issued ticket of the referencing command). An emission may therefore refer to any ticket value it possesses, without synchronous runtime lookup or partial admission. Empty callback batch returns Accepted(empty), distinguishing actual delivery from stale no-op. Legacy handlers have no batch receipt.

FlowCallbackConfig.max_callback_commands is finite, positive and immutable for the run; proposed default1024 is a safety bound requiring independent default/configuration review. It counts every emitted command kind uniformly. Before allocating/appending an over-cap command, emit returns CallbackBatchLimitExceeded, issues no ticket, and poisons the entire batch even if ignored. The receipt uses failed_ticket=None. Successful emission count and vector length never exceed this cap; no unbounded command-vector growth or runtime ID allocation is permitted. Sink maintains its first poison error deterministically. Ticket-index arithmetic exhaustion follows the same no-ticket poison behavior with CounterOverflow.

After callback returns, validate the whole vector against one staged admission view in emission order. Mirror current Flow ingress checks, including owner/resource/work ownership, restart template, association, terminal/lease validity, pending-release reservation, deadline/time rules and reserved/domain registration. Add staged reservations for every earlier Acquire association and Release, and resolve earlier ticket references to staged request entities. Cancel/Reprioritize of a newly admitted Pending request follows the same ordinary execution-time semantics, not synchronous state changes. Conflicting duplicate work acquisition/release rejects all; repeated cancellation/rekey follows existing ingress permission rather than introducing speculative new prohibitions.

Before any admission writes, check aggregate entity IDs/generations, Flow scheduled and Scheduler scheduled counters, sequence/event IDs and every deadline token (one extra only if deadline>at). Check all arithmetic against exact existing caps, including ticket emission/index exhaustion. No calls to public ingress one-by-one as the transaction algorithm. Once preflight succeeds, commit all request associations/reservations and scheduler events in emission order infallibly; return each actual primary EventId, optional RequestId and optional deadline EventId. Callback-generated events at delivery time join ordinary canonical scheduler order; never recurse or bump time. Each later event has its own lifecycle EventId and independent budget admission.

Batch failure returns Rejected in the delivering FlowDispatch, not step Err: the delivery and earlier lifecycle state remain committed. Context mutations are retained; callback is never replayed. The first failed ticket identifies deterministic emission-order validation failure; aggregate counter failure identifies first emission whose cumulative reservation exceeds cap. No request ID, event ID, association or reservation is consumed by rejection. No arbitrary context rollback or panic recovery promised. All ordinary post-callback validation failures use Some(the referencing/emitting ticket); cap/index poison uses None and takes precedence over subsequent whole-batch validation. It never panics/wraps or admits a partial batch. UnregisteredDomainEvent encountered on ordinary public scheduling is admission Err; execution-time semantic rejection is FlowDispatch.error under normal boundary rules.

## Budget/transaction barrier

Preview-at pure planning, aggregate lifecycle/delivery cost and all fallible staged cleanup/arithmetic checks occur before consuming scheduler head. A callback's commands cannot be known until callback execution: they are separately staged admission after its already-admitted delivery, so their rejection never retroactively rejects or replays that delivery. Future command dispatch transitions are charged then, not double charged at admission. Budget-exhausted delivery never invokes callback, allocates tickets or mutates context. Permanent RunHalted prevents all ingress and registrations; no cap raise, time skip, consumed-event sidechannel or pendingwork loss.

This requires extending pure planner delivery liveness to new descriptor kinds and preflighting every generated notification. Existing two-token scheduling and per-dispatch operation-cap proof must become aggregate for both legacy and continuation deliveries. Nonrollback user effects start only after delivery admission and consume/commit. No source acceptance before private fault-injection joins pass.

## Compatibility / ownership

New FlowDispatch field and public error variants can break struct literals/exhaustive Rust matches; classify experimental-breaking development-only. Existing FIFO/manual methods and handlers preserve behavior absent opt-in. Root internal03/25 review is not external maintainer approval. Retain release hold, no C ABI/FFI/bindings/portable codec promises. One integration writer owns flow.rs; public black-box fixtures can be prepared separately only after coordinator freezes this proposal. No core/ABM/Arrow/C1/C4 edits in this packet.

## Amendment disposition

Coordinator accepted conceptual separate legacy-first two-token delivery and cost per actual delivery, plus dead-context/work stale zero cost versus never-registered defensive error. This amendment concretizes the reviewed gaps; it freezes this bounded concrete API for failing-fixture preparation after coordinator review; runtime implementation still requires a separate reviewed packet. Existing FlowConfig shape and its frozen budget fixtures remain unchanged. FlowCallbackConfig implements Default with positive1024; new() and with_config(existing FlowConfig) retain signatures and use that callback default. Additive with_configs chooses both caps before execution; no setters or per-registration overrides. FlowDispatch/errors still require experimental source compatibility disposition. Original proposal and review matrix remain retained in the predecessor artifact directories.

## Required review and failing-fixture families

| Obligation | Independent negative/positive oracle | Barrier |
|---|---|---|
| Typed ownership | wrong C, missing registration, mismatched work owner; context pointer/state retained | concrete contract before fixture |
| Completion workflow | A completes, hook submits existing B, later B runs; A never rebound | public fixture |
| Legacy coexistence | legacy then continuation distinct tokens; exact costs and origin ordinal | private aggregate scheduling + public trace |
| Ticket isolation | prior Acquire resolves real request/event; foreign/forward/non-Acquire ref rejected | whole-batch planner |
| Atomic reservation | two acquires same work; duplicate release; second invalid command leaves every ID/counter/map unchanged | private staged admission |
| Counter/time joins | entity/scheduler/sequence/deadline overflow at second command; no callback replay | private injected boundaries |
| Reserved/unknown kinds | each4000–4003 rejected at all ingress; missing ordinary hook admission error; impossible scheduled unregistered state private-injected only | public/private ingress |
| Nonrollback receipt | mutate C then emit valid+invalid; context changed once, no commands admitted | public fixture |
| Budget | limit blocks notification before callback; exactlimit delivers once; subsequent zero-duration command independently halted/pending | frozen budget regression |
| Staleness | cleanup before notification/domain token; empty/no-cost invalidated delivery, no dereference | private liveness |
| Causality | emitted event actual ID; completion captured ordinal; domain None; no delivery lifecycle row | canonical trace |
| Compatibility | manual/FIFO and old handler tests unchanged, new enum/struct risk held | owner03/25 review |
| Bounded sink | default1024 independently reviewed; each command kind at cap passes, next emit poisons even if ignored, vector/ticket counts stay capped | concrete config/default fixture |
| Identity exhaustion | private next batch identity u64MAX checked before consume/callback; unchanged head/context; no reuse after rejected batch | private counter injection |
| Reference timing | issued referencing ticket retained; only whole-batch validation rejects foreign/forward/non-Acquire with InvalidCommandTicket | public sink and private forged-ref oracle |

No executable verification commands are invented here. Followup bound fixture/implementation packets must resolve actual compiler paths, cache ownership, exact test targets and current leases at dispatch. This contract does not close Q4 builders, shared ABM adapters, telemetry or workflow phase review.


## Reviewed extension boundary: shared Flow/ABM v1

The companion [shared Flow/ABM v1 contract](q4-shared-abm-v1.md) defines a proposed
experimental extension to this v1 baseline: a safe read-only World/time view,
one explicitly typed domain behavior carrier/stream per actor generation,
registration and opaque event-kind-bound handles, and a buffered DespawnActor
owned command. Its status records the internal review stage; it does not claim
implementation, hosted qualification or full Q4 acceptance.

The existing five-command fixture and historical semantics above remain the
baseline. The companion extends the owned enum and public errors under an
explicit experimental source-compatibility/release hold; exhaustive matches may
require migration. It is not a universal nonbreaking or stable release claim.

All shared behavior commands, including DespawnActor, use the same callback
cap, opaque ticket identities, whole-batch admission, causal receipts and
persistent same-tick fail-stop rules. Actor cleanup requires atomic dispatch
preflight of all then-live owned requests/work/carrier state and notifications.
Future cleanup counts are not falsely claimed known at earlier scheduling.
Legacy ABM mutable-World callbacks remain separate from the new borrowed API.
No whole Registry reference aliases mutable typed context, no raw mutable
scheduler/world escapes, and no second authoritative simulation clock is owned.

The shared extension explicitly changes duplicate pending actor-despawn ingress:
the live actor remains valid, but a second pending command rejects with the new
`DuplicateActorDespawn` error before scheduling. Only the signature and normal
single-despawn behavior of `despawn_actor` are preserved; this is an experimental
behavior change under the release hold. Future lifecycle capture distinguishes
causal prior leases from resulting active leases and records staged transition
snapshots before cleanup clears state; Track 04 schema review remains separate.
