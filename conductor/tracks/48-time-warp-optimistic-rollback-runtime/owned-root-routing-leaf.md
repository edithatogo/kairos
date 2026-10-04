# Owned root routing and native admission join

Status: proposed exact contract for joint review,4 October2026. Constructor source integration is a prerequisite and is not yet accepted; bind actual source only after its independent GREEN review. Architecture proposal readback2d0e1ee informs this draft. No source writer is dispatched by this document. Track48 remains In Progress and the whole native/distributed objective remains mandatory.

## Scope and actual behavior

Enable owned schedule_initial only after healthy/open inputs, complete sealed live peers, global routes/authority/GVT/ancestry, local bounds and checked identity/revision preflight. Source must be owned. A local destination receives exactly one scoped positive root in its actual local queue. A remote destination receives no local queue or mirrored model: the source retains one positive root obligation in its outbox. Public returned OptimisticMessage is immutable metadata and cannot authenticate admission. Handler execution remains explicitly OwnedRuntimeJoinIncomplete, so this leaf cannot claim execution/rollback parity. Raw receive and raw fossil stay guarded. Closing initial inputs blocks new roots but permits already-retained native admission/ACK.

Private delivery identity/storage keys preserve actual source, complete authority, complete logical ID and incarnation throughout queues, known deliveries, replay markers, history/outputs, conflict indexes and tombstones. Private map ordering is mechanical storage ordering; execution keys still exclude authority/incarnation/kind. Source reserves every initial (source LP,root logical ID) cohort across both local and remote routes. Acknowledgement cannot permit rescheduling that root with changed destination/payload. Current-authority configuration is immutable; epoch rotation/recovery/migration are later joins.

## Exact proposed public API

All are feature-gated time-warp exports. Opaque structs have private fields, Clone+Debug, no public constructor/deserialization or conversion from OptimisticMessage. OptimisticSendKey additionally implements Eq+PartialEq for exact identity, with no public Ord/Hash.

```rust
outbound_pending(&self) -> Result<Vec<OptimisticOutboundView>, OptimisticError>;
ready_native_sends(&self) -> Result<Vec<NativeOutboundSend>, OptimisticError>;
admit_native(&mut self, send: &NativeOutboundSend)
    -> Result<NativeAdmissionCapability, OptimisticError>;
acknowledge_native_admission(&mut self, cap: NativeAdmissionCapability)
    -> Result<(), OptimisticError>;
accounting_snapshot(&self) -> Result<OptimisticAccountingSnapshot, OptimisticError>;

pub enum OptimisticOutboundStatus {
    Ready,
    BlockedReplacement,
    RetiredPredecessorAwaitingAccounting,
}
pub enum NativeAdmissionMembership { Pending, Executed, Tombstoned }
```

Both enums are Copy+Debug+Eq+PartialEq. This join issues only Ready positive-root sends and Pending admission. Unsupported anti/blocked/retirement states are not fabricated; later joins implement them before final acceptance.

Immutable getters:

- OptimisticSendKey: source_lp()->LpId, authority()->OptimisticAuthority, logical_id()->&LogicalEventId, incarnation()->u64, kind()->OptimisticMessageKind.
- OptimisticOutboundView: key()->&OptimisticSendKey, message()->&OptimisticMessage, issuer()->&NativeAccountingAuthority, status()->OptimisticOutboundStatus.
- NativeOutboundSend: key()->&OptimisticSendKey, message()->&OptimisticMessage, issuer()->&NativeAccountingAuthority.
- NativeAdmissionCapability: key()->&OptimisticSendKey, message()->&OptimisticMessage, receiver()->&NativeAccountingAuthority, sender()->&NativeAccountingAuthority, recorded_revision()->u64, recorded_membership()->NativeAdmissionMembership.
- OptimisticAccountingSnapshot: revision()->u64; local_positive_count(),local_anti_count(),local_replay_count(),ready_positive_count(),ready_anti_count(),blocked_count(),retirement_count(),reserved_receipt_count(),retained_receipt_count() all ->usize; local_minimum(),outbound_minimum(),minimum_obligation_tick() all ->Option<Tick>; frontiers()->&BTreeMap<LpId,Tick> for actual owned LPs only. Snapshot Clone+Debug+Eq+PartialEq; count/minima observers certify no global cut or completion.

Polling returns exact retained immutable work without consuming it or allocating revision. Ordering of inspection is deterministic by private exact send keys; it is not a selection rule among competing executable cohorts. Receipt keys include kind and full authority; full envelope metadata is also retained and checked.

## Actual admission and accounting

Admission validates source issuer is exactly the registered live source owner/generation; complete immutable global config and configured source authority; destination is owned by this receiver; complete metadata/route/GVT/ancestry; logical cohort/delivery conflicts; pending/receipt capacity; token epoch and accounting revision before any mutation. Never advance another source's emission allocator. Metadata-only raw envelope, as_anti conversion or copied observer data cannot mint a ticket. Unknown registered issuer, unowned endpoint, wrong authority or unrelated receipt fails unchanged.

Remote scheduling reserves one sender completion/ACK record slot as well as one active outbox slot before allocation/publication. The sender receipt budget is shared across outstanding reservations and retained completed receipts. ACK consumes its pre-reserved slot, frees only that exact positive outbox obligation and retains exact completed root/cohort state for duplicate/conflict protection. Receiver admission reserves one stable receipt record plus pending capacity before insertion. The receiver receipt budget counts its stable records together with sender reservations/completions if it performs both roles. An exact repeated admission returns the original accounted capability without new capacity, token change or revision, even when full. Exact repeated ACK is similarly unchanged. First admission changes only owned destination epoch and accounting revision once; first remote scheduling changes accounting but no unrelated destination token. First ACK increments once; each local enqueue increments its destination epoch and accounting revision once.

The admission capability privately binds the exact admitted ticket source issuer, including runtime ID, recovery generation, weak-witness identity and complete immutable descriptor. ACK checks cap.sender against SELF before consulting an active-send or completed-receipt cache; foreign actual source rejects NativeSendIssuerMismatch{source_lp} unchanged. Receiver exact retry validates actual ticket issuer against the registered source owner before receipt-cache lookup.

ACK then verifies actual registered receiver/generation/destination, exact send key/kind and complete message bytes. It cannot acknowledge any other positive or anti. Unknown/conflicting ACK rejects unchanged. Closing inputs does not invalidate outstanding capabilities. Root reservations and receipts remain retained until the later complete-group fossil policy; this join does no independent pruning and makes no durability claim.

## Concurrent issuer Drop

Replace the private witness representation with an admission gate using standard-library synchronization. Each native ownership-dependent mutation—including register_native_peer (with the incoming issuer too), seal_native_peers and close_initial_inputs as well as root scheduling, admission and ACK—upgrades weak witnesses, acquires shared read guards in runtime-ID order, checks active flags and holds guards through validation/preflight/publication. No guards escape through capabilities or public API. A guard may not keep a dead issuer valid: runtime Drop takes its own exclusive write guard, marks inactive and releases it before model fields/destructors are dropped. Use explicit runtime Drop/invalidation before automatic field destruction; owned-state Drop after process fields would be insufficient. Lock failure returns NativeAccountingUnavailable before mutation; Drop invalidates even if the lock is poisoned. is_live remains point-in-time.

No handler, snapshot, restore or model destructor runs while these guards are held. Constructor snapshots occur before issuer publication. Native root routing/admission/ACK mutate only runtime-owned data. The later enabled-handler/rollback join must separately address lifecycle synchronization around model callbacks, rather than holding a lock that a reentrant destructor could deadlock. Complete native cuts later exclusively borrow all actual runtimes; flag sampling cannot authorize fossil collection.

## Exact proposed new failures

```rust
UnregisteredNativeIssuer { runtime_id: u64, recovery_generation: u64 },
NativeSendIssuerMismatch { source_lp: LpId },
NativeAuthorityMismatch {
    source_lp: LpId,
    expected: OptimisticAuthority,
    actual: OptimisticAuthority,
},
UnknownNativeSend,
ConflictingNativeReceipt,
OutboxLimitExceeded { limit: usize },
ReceiptLimitExceeded { limit: usize },
NativeAccountingUnavailable { runtime_id: u64 },
```

Reuse existing endpoint/route/GVT/logical/delivery/pending/epoch/revision/stale issuer errors. Preserve Poisoned precedence. These public enum additions retain the alpha exhaustive-match source compatibility limitation.

## Required independent evidence and later joins

Two genuine disjoint runtimes A owns0/B owns1: remote scheduling changes only A outbox, actual admission changes only B queue, local scheduling queues once, remote models/tokens never appear. Inspect/poll/admit/ACK exact bytes and full-width tick/namespace/epoch; compare counts/revisions/tokens/model/RNG snapshots and root reservations. Input closure permits retained admission/ACK. Changed root payload/destination after ACK remains a conflict. Capacity failure preflights allocator/token/revision and exact retries succeed at full capacity. Raw forged metadata and anti cannot become tickets; wrong configured issuer/destination/unrelated receipt rejects. Drop-before/after registration plus concurrent Drop/admission proves either complete publication before invalidation or unchanged rejection afterward. Add the actual-issuer collision oracle: A and fresh A′ own the same LP under identical configured epoch and emit identical root key/incarnation/metadata. B registers A and admits A. A′ cannot acknowledge its identical outbox with A's capability; B must reject A′'s matching ticket before consulting A's cached receipt; it cannot return A's cached capability for A′. A's genuine ACK and duplicate ACK still succeed. Native run remains guarded with unchanged model/RNG.

Use actual matching Rust1.98.1 and1.76.0 tools/targets/receipts; independently authored RED and GREEN remain distinct. Full combined CI/hosted/normal merge are separate gates. Successful root routing does not replace handler outputs, stragglers, suffix replay, retained antis, P→N1→N2 retirement chains, complete native group GVT and Track49 durable real process/rank joins. Ownership/actualcommands/base/source hashes are bound only after prerequisite acceptance and joint contract review.


Qualified architecture accepted the bounded direction atb690390 and confirmed this source-issuer receipt correction after the coordinator traced the A/A′ collision. Exact-document independent fixture review and accepted constructor integration remain prerequisites to dispatch.

Independent fixture review ofb061c3a accepted exact sender/cache binding and requested explicit gate coverage for registration/seal/close. Those ownership-dependent operations are named above. The first constructor contract remains immutable and point-in-time; this later join upgrades its private synchronization before enabling admission.
