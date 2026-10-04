# Owned runtime and native accounting API contract

Status: proposed for joint review, 4 October 2026. Native source baseline is authority-envelope integration5f1df83, not an enabled owned implementation. This is a native prerequisite to faithful distributed execution, not a transport substitute. The full Track48/49 objective and dependency gates remain. Authority-envelope metadata and all-local rejection have a separately accepted leaf; this document does not broaden it before joint acceptance.

## Static configuration

Proposed public owned constructor:

```rust
pub struct OptimisticOwnedOptions {
    pub simulation_namespace: u128,
    pub current_authorities: BTreeMap<LpId, OptimisticAuthority>,
    pub emission_epochs: BTreeMap<LpId, u64>,
    pub local_limits: OptimisticLimits,
    pub max_global_lps: usize,
    pub max_outbox_entries: usize,
    pub max_transition_entries: usize,
    pub max_receipt_entries: usize,
}

OptimisticRuntime::new_owned(
    global_partition: PartitionPlan,
    global_topology: BTreeMap<LpId, Vec<LpId>>,
    owned_processes: BTreeMap<LpId, P>,
    options: OptimisticOwnedOptions,
) -> Result<Self, OptimisticError>;
```

All maps/configuration are immutable in the first owned implementation. Global partition LPs are bounded and every directed route is validated globally. Owned processes are a nonempty exact subset. current_authorities covers every global LP, uses Scoped with exactly the configured namespace, and emission_epochs keys exactly equal owned LP keys and match current authorities. Existing LocalPreview constructors/limits remain unchanged. Only owned LPs have process state, validity tokens and emitters. Remote LPs have no local mirror/model. Local source scheduling may target remote destinations but unowned source scheduling rejects unchanged. Epoch rotation, restart ownership activation and migration are disabled until separately implemented and accepted.

## Actual native issuer and peer coverage

```rust
native_accounting_authority(&self) -> NativeAccountingAuthority;
register_native_peer(&mut self, peer: NativeAccountingAuthority)
    -> Result<(), OptimisticError>;
seal_native_peers(&mut self) -> Result<(), OptimisticError>;
```

A NativeAccountingAuthority is privately issued by an actual owned runtime. It anchors that live runtime ID/generation, namespace, exact owned LP scope and global partition/topology/current-authority configuration. No public constructor, deserialization, mutable fields or caller verifier callback may mint one. Registration compares the complete global configuration, forbids overlapping LP ownership, and binds each peer to its configured epochs. Exact repeated registration is idempotent; conflicting peer/generation/configuration rejects unchanged. Sealing requires complete global ownership coverage including self, exactly once per LP, and prevents later mutations. Scheduling/execution/admission requires sealing; missing coverage cannot be inferred from topology alone. Local limits may differ across participants and do not authorize a remote send beyond its receiver's bounds.

Capabilities below establish volatile in-memory native facts only. They cannot be persisted/serialized across process restart and cannot be advertised as authenticated external transport or durable admission. Future Track49 verifier/persistence/recovery APIs need a separate freeze. Public raw Scoped envelope construction never authenticates fresh native admission: plain receive rejects it with a typed verified-admission-required error in owned mode. Existing all-local Scoped rejection remains unchanged.

## Ready sends and receiver admission

```rust
outbound_pending(&self) -> Vec<OptimisticOutboundView>;
ready_native_sends(&self) -> Vec<NativeOutboundSend>;
admit_native(&mut self, send: &NativeOutboundSend)
    -> Result<NativeAdmissionCapability, OptimisticError>;
acknowledge_native_admission(&mut self, cap: NativeAdmissionCapability)
    -> Result<(), OptimisticError>;
accounting_snapshot(&self) -> OptimisticAccountingSnapshot;
```

Inspection exposes exact immutable envelopes and ready/blocked state; polling never consumes work. A NativeOutboundSend has private fields and is issued only for actual ready retained routing work. A metadata-only OptimisticMessage cannot be converted into one. A blocked replacement exposes inspection data but cannot issue a ready ticket. Ticket issuer must be the registered source owner/generation; source authority, owned destination and full ancestry/routes/GVT/capacity all validate before mutation. Incoming observations never advance another authority's emitter counter.

Exact delivery storage identity carries source, full authority, complete logical ID and incarnation through queues, history/outputs, known-delivery/conflict indexes, replay markers and tombstones. Send identity additionally distinguishes positive from anti and binds original tick/destination/payload. No authority/incarnation ordering selects the active version of a logical occurrence. Scoped receive/admission permits only one executable unretired version per(source LP,completeLogicalID) cohort, excluding tick/destination. Conflicting unlinked positives reject unchanged/unacknowledged; transport cannot silently replace them or manufacture a successor.

Admission capability is issued only by the actual receiver after exact native pending/executed/tombstoned membership is established. Duplicate retry obtains exact accounted readback, not an invented acknowledgement from DuplicatePositive alone. Sender verifies receiver handle/generation/destination, exact identity/kind and full metadata. It discharges only that obligation and retains original history for future rollback. Exact repeated receipt is idempotent; unknown/conflicting receipts reject unchanged. Acknowledged receipt records are bounded and their retention/fossil policy must preserve duplicate-proof semantics before GVT; exhaustion rejects rather than dropping evidence. Anti admission is distinct from cancellation applied and cannot release replacement.

## Cross-owner retirement barrier

```rust
receive_native_retirement(&mut self, request: &NativeRetirementRequest)
    -> Result<(), OptimisticError>;
applied_native_retirements(&self) -> Vec<NativeRetirementCapability>;
release_native_replacement(&mut self, cap: NativeRetirementCapability)
    -> Result<(), OptimisticError>;
```

Retirement requests and capabilities have private fields, immutable inspection and no public reconstruction. Runtime rollback retains original send records and ties a replay successor to its exact predecessor; model handlers cannot invent that binding. Request binds exact predecessor anti AND successor metadata/cohort/transition, source owner and old receiver. Replayed replacement remains retained/accounted/unpublishable at source until the old destination applies cancellation. An atomic pair at new destination is insufficient when old/new destinations belong to different owners.

Receiver processing produces retirement capability only after old local effects are removed or were never applied, a tombstone prevents delayed reapplication, and all induced downstream antis are retained in accounted state. Queued anti admission, socket completion, timeout and positive admission capability cannot satisfy this transition. Retirement capability binds both exact envelopes, the actual old receiver/generation and accounting revision. Native capability is volatile; no descendant graph drain or crash-safe claim follows.

Sender release verifies the complete bound transition and registered old receiver before making the successor publishable. Exact duplicate transition/receipt is idempotent; forks, cycles, unknown/skipped predecessor chains and mismatched metadata reject unchanged or remain explicitly bounded/accounted staging. Sender shutdown/timeout never releases replacement. Historical retries/antis require exact retained authorized records, not merely an epoch value. Unrecognized old-authority work is rejected. The immutable first implementation cannot rotate to a new epoch, reactivate recovered handles or accept reconstructed external receipts.

## Atomic bounds and local GVT

Execution outputs and rollback antis use one preflight/routing operation: local destination enqueues once, remote destination retains once. Preflight local pending, remote outbox, transition/receipt/tombstone capacities, routes and accounting-revision overflow before publication. A healthy rollback-capacity failure preserves model/RNG, queues/history/tokens/GVT and accounting. Existing post-handler invalid-output/restore failure poison rules remain explicit. Counters/identity reservations never wrap or silently unblock.

Snapshots report checked revision, local pending minima, ready/blocked remote obligations, per-LP frontier information and unresolved retirement work. Outbox positives, antis, blocked successors and retirement transitions independently constrain fossil/GVT to the minimum relevant tick, including predecessor/successor minima. Equality remains reversible. ACK handoff may move an obligation into receiver accounting but cannot erase both sides before proof; independently sampled minima are not a global consistent cut. Native snapshots do not authorize distributed fossil collection. Track49 separately proves durable accounting, recoverable receiver retirement, durable sender release, fencing, root-sequence reservations and actual process/rank crash/migration.

## Required proof and bounded implementation joins

1. Owned constructor/issuer/peer sealing + exact authority indexes: two genuinely disjoint runtimes, no unowned process/token, metadata-only/fresh unauthorized admission rejected. No transport/fencing claim.
2. Routing and retained ready sends + native exact admission capabilities: local output once, remote output only outbox, repeated inspection unchanged; wrong peer/identity/kind/metadata rejects; positive ACK preserves history and does not clear anti.
3. Retirement barrier: three disjoint runtimes(source,old destination,new destination), delay actual anti processing, reorder/loss/duplicate receipts, replacement never ready on admission-only ACK, exact applied cancellation releases it and late old anti cannot cancel successor.
4. Bounded replay/capacity/local GVT: budget-one resumes, forks/cycles/missing predecessors, mixed local/remote atomic failures, model/RNG and committed trace match an independent serial oracle. Local control oracle may own all LPs; participant implementations may not mirror remote LPs.
5. Track49 real process/rank persistence/recovery/fencing and consistent cuts are mandatory later joins, not optional improvements or a local-capability serialization exercise.

Before dispatch, joint review must settle accessor/status enum and exact typed errors, ticket/receipt capacity/revision reservations and duplicate/fossil retention policy. Current names are proposed. No executable owned writer packet is authorized by this draft.
