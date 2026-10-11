# Flow V4 native checkpoint contract

**Status:** Owner design reviewed by root and independent Sol reviewer on 12 October 2026. The native queue-order and arrival-handoff join is qualified at `ef20ae54b73dc47d59250c9b073c063311704413`; final result SHA-256 `ae8285021fa7c9ee3adc3e23b587048d58f946db66a21f214f5b391b927b42c6`. This contract freezes native DTO/API semantics. Implementation requires a separately bound, reviewed worker packet; wire/routing/consumers and parent pin remain later gates.

## Native DTO

Keep `FlowCheckpointV3` and all V1/V2/V3 layouts unchanged. Define `FlowCheckpointV4` as a lossless V3 payload plus two canonical vectors:

```rust
pub struct FlowCheckpointV4 {
    pub version: u32, // exactly 4
    pub config: FlowConfig,
    pub callback_config: FlowCallbackConfig,
    pub next_batch_identity: u64,
    pub budget_tick: Option<SimTime>,
    pub budget_consumed: u64,
    pub budget_halt: Option<FlowBudgetHalt>,
    pub scheduler: SchedulerCheckpointV1,
    pub world: WorldCheckpointV1,
    pub resources: Vec<ResourceId>,
    pub requests: Vec<RequestId>,
    pub actors: Vec<EntityId>,
    pub actor_domains: Vec<(EntityId, EntityId)>,
    pub work_registrations: Vec<FlowWorkRegistrationV1>,
    pub contexts: Vec<FlowContextRegistrationV1>,
    pub handlers: Vec<FlowCallbackRegistrationV1>,
    pub continuations: Vec<FlowCallbackRegistrationV1>,
    pub domains: Vec<FlowCallbackRegistrationV1>,
    pub builtins: FlowBuiltInStoresV1,
    pub context_stores: Vec<FlowContextStoreV1>,
    pub restart_store_manifest: Vec<(String, u32)>,
    pub restart_stores: Vec<FlowRestartStoreV1>,
    pub commands: Vec<(EventId, FlowCommandV3)>,
    pub notifications: Vec<(EventId, FlowNotificationV1)>,
    pub pending_releases: Vec<LeaseId>,
    pub pending_despawns: Vec<EntityId>,
    pub created: u64,
    pub destroyed: u64,
    pub scheduled: u64,
    pub next_admission: u64,
    pub next_lease: u64,
    pub resource_grant_flags: Vec<(ResourceId, bool)>,
    pub started_frontier: Option<SimTime>,
    pub allocation_modes: Vec<(ResourceId, FlowAllocationMode)>,
    pub queue_order_modes: Vec<(ResourceId, FlowQueueOrdering)>,
    pub request_queue_orders: Vec<(RequestId, FlowRequestQueueOrderV4)>,
}

pub enum FlowRequestQueueOrderV4 {
    AdmissionOrder,
    // Untrusted raw DTO bytes permit structural rejection before key construction.
    StableKey(Vec<u8>),
}
```

The V3 inventory to retain verbatim is visible in `crates/kairo-ecs-des/src/flow/checkpoint/v3.rs` at `FlowCheckpointV3`: config/callback config, batch/budget/halt fields, scheduler/world, resource/request/actor identities and bindings, registrations/codecs, stores, commands/notifications, pending operations, counters, grant flags/frontier, and allocation modes. The names above are the native DTO contract; wire representation is a separate review.

Ordering rules:

- `queue_order_modes` is strictly ascending by full `ResourceId`, has exactly one row for each live `resources` entry, and no other rows. It records `AdmissionOrder` or `StableKey` independently of allocation mode (`allocation_modes`).
- `request_queue_orders` is strictly ascending by full `RequestId` and has exactly one row for every retained `requests` identity, including Pending, Queued, Active, Suspended and retained terminal requests. `AdmissionOrder` means no key; `StableKey` carries the exact immutable 1–128 opaque bytes. Duplicate key bytes across different requests are valid; duplicate request identities are not.
- For a request whose referenced resource is live, its tagged request order must match that resource's mode. For a removed resource generation, every retained request row in that generation's group must be uniform. Such a group must also satisfy the pre-existing terminal/missing-resource validation; V4 adds no permission to retain a live request against a missing resource.
- Do not add a historical mode row for a removed resource with no retained requests. A fully erased set of keys and mode after removal has no independent witness in current native state; validation must not claim it can recover this history. Resource generation is part of identity, so a reused slot cannot inherit an old generation's mode/key.
- Keep physical `ClaimQueue`/`PriorityKey` and deadline membership data as V3 currently serializes it. These structures preserve old queue/deadline membership; the effective allocation order is derived from mode, tagged key, primary priority, enqueue sequence and RequestId. Do not serialize the sidecar index as authority.

## Methods and behavior

Additive native API:

```rust
impl FlowRuntime {
    pub fn capture_checkpoint_v4(
        &self,
        codecs: &FlowCheckpointCodecs,
        limits: FlowCheckpointLimits,
    ) -> Result<FlowCheckpointV4, FlowCheckpointError>;

    pub fn restore_checkpoint_v4(
        image: FlowCheckpointV4,
        codecs: &FlowCheckpointCodecs,
        limits: FlowCheckpointLimits,
    ) -> Result<Self, FlowCheckpointError>;

    pub fn restore_checkpoint_v4_with_rebind(
        image: FlowCheckpointV4,
        codecs: &FlowCheckpointCodecs,
        limits: FlowCheckpointLimits,
    ) -> Result<(Self, FlowCheckpointRebindV1), FlowCheckpointError>;
}
```

Avoid adding V4 to the public image/ref or changing automatic version routing in this packet unless the accepted design explicitly includes that seam. A direct native API keeps this contract bounded. If names collide with a later reviewed image/ref plan, stop and rebind rather than widening signatures here. Do not add a public V4-to-old-version downconversion API; old public conversion methods stay untouched and V4 is not silently converted to a lossy legacy form.

Capture preflights source counts and checked key-byte totals before metadata/image copies, validates sidecar parity by streaming, and preserves V4 metadata separately from V3 base parts. It must reject corruption instead of rebuilding a valid index and masking a broken live cache. `capture_checkpoint_parts` currently invokes the old-version queue guard unconditionally, so direct V4 cannot call it unchanged. Add a private reviewed capture policy/helper: old entrypoints retain the legacy-only guard; direct V4 validates the V4 metadata and source parity, then enters common base capture using a private `StableMetadataValidated` policy. Only private V4 code can supply that policy, and the validated metadata stays alongside base parts. Do not add a public V3 conversion or let V1/V2/V3 callers opt into this policy.

Restore validates V4 counts, key lengths and full metadata cross-joins before caller codecs. A private validated-restore metadata parameter/seam must carry the already checked mode/key maps into common restoration. Restore base stores/codecs into a temporary runtime, install validated modes and typed keys before invoking a generalized effective-index reconstruction helper, then check sidecar parity and all original validations before publication. Do not restore through a public V3 image and overwrite ordering afterward. Existing legacy restore continues with no keys, all resources `AdmissionOrder`, and the legacy-only reconstruction path. Keep the existing `rebuild_admission_queue_order_indexes` distinct or wrap it with an explicit policy; it currently rejects retained keys.

## Validation order and bounds

An incorrect version returns `FlowCheckpointError::UnsupportedVersion(version)`, matching V1/V2/V3 restore. Before length-driven allocation, check untrusted vector counts, each key length, and checked aggregate key bytes. Then perform full bounded structural cross-join validation (temporary validator maps are bounded by `max_resources`/`max_requests`) before caller context/template codecs. Next validate existing scheduler-command identities, resource/request/work lineage, queue/deadline/active claims, pending operations, frontier/budget/counters and V3 limits. Do not use a codec callback as a validator. Capture similarly preflights source counts/key totals before copies and streams sidecar parity rather than allocating a duplicate full index.

Use existing `FlowCheckpointLimits` fields rather than adding fields to a public struct with external struct-literal consumers:

- mode rows `<= max_resources`; request-order rows `<= max_requests`.
- Per StableKey payload is nonempty and `<= MAX_FLOW_QUEUE_ORDER_KEY_BYTES` (128), consistent with `FlowQueueOrderKey::new` in `flow/queue_order.rs`. The raw `Vec<u8>` DTO allows malformed zero- and 129-byte values to be rejected before conversion to runtime key type.
- Sum opaque key bytes with checked addition and charge them to the same `max_key_bytes` aggregate as existing registration/key strings. After computing opaque key total, pass only the remaining key-byte budget to common V3 base validation/capture; do not grant two independent full budgets. Preserve current `max_payload_bytes` meaning (component/codec payload); do not silently redefine it to include V4 structural rows. Bound structural allocations by existing row/count limits plus the shared key-byte budget. Exact wire `max_total_bytes` belongs to a later wire contract. Do not relax any current V3 limit or add an unbounded historical table.
- Existing defaults in `checkpoint.rs::FlowCheckpointLimits::default` are 100,000 resources, 1,000,000 requests, 16 MiB key bytes and 256 MiB payload bytes; mode/key row counts therefore fit existing ceilings.

Use the accepted code's existing `check_count`/preflight machinery and error taxonomy. Exact first-error precedence is a schema review item. Structural invalidity maps to `InvalidState`, count/key/codec-payload overrun to `LimitExceeded` with stable labels, and caller codec errors retain existing codec mapping. V4 structural counts and checked byte totals need explicit preflight; they are not a new meaning for `max_payload_bytes`.

## Restore/index seam

The qualified native implementation's runtime authority is in `FlowRuntime`: `queue_orderings: BTreeMap<ResourceId, FlowQueueOrdering>`, `queue_order_keys: BTreeMap<RequestId, FlowQueueOrderKey>`, and derived `queue_order_indexes: BTreeMap<ResourceId, ResourceQueueOrderIndexes>`; `FlowRuntime::validate_queue_order_indexes` checks both effective queue/preempting indexes against legacy `ClaimQueue`, `PreemptingWaiters`, request rows and key/mode relationships. The existing `rebuild_admission_queue_order_indexes` is specifically legacy-only and rejects nonempty keys; V4 needs a separate checked reconstruction seam that accepts the validated modes/keys and rebuilds the effective cache. Do not weaken or repurpose the legacy-only guard without reviewing all callers.

Checkpoint validation/restoration is concentrated in `crates/kairo-ecs-des/src/flow/checkpoint.rs`; V3 DTO/conversion is in `checkpoint/v3.rs`; root re-exports are in the checkpoint and flow module surfaces. V4 changes will likely touch `checkpoint.rs`, a new `checkpoint/v4.rs`, and `flow.rs` for the reconstruction seam. One owner should control these shared paths. Keep wire codec and FlowCheckpointImage/Ref routing in later separately accepted packets.

## Minimum native proof set

Use actual Flow captures/restores and continuations, not hand-authored “valid” images derived from the candidate:

1. AdmissionOrder capture/restore with exact equality/continuation and unchanged existing V1/V2/V3 behavior. No new native literal golden is proposed; joined qualification separately retains existing outer fixture coverage.
2. StableKey live resource with retained Pending and Queued requests; assert exact key bytes/order, effective selection, and post-restore same next grant.
3. Active and Suspended request continuation (including reprioritize/Resume) with immutable key and original enqueue sequence retained.
4. Removed resource with terminal retained keyed requests; capture/restore preserves tags and request accessor behavior. Include multiple same-generation rows to prove partial key/mode disagreement rejection; do not claim detection after all history is erased.
5. Mixed live AdmissionOrder and StableKey resources, exact one-row-per-live-resource and one-row-per-retained-request coverage.
6. Malformed native DTO matrix: wrong version; noncanonical/duplicate/missing/unknown mode row; noncanonical/duplicate/missing request row; empty and 129-byte key; live mode/request-tag mismatch; inconsistent removed-generation group; invalid nonterminal reference. For each, use codec-call counter to prove rejection before caller codec and ensure positive valid baseline.
7. Corrupted source sidecar capture fails before codecs without mutation; valid capture and restored sidecar verify parity with unchanged queue/preempting/deadline membership.
8. Limit boundary: exact 128-byte key and aggregate `max_key_bytes` at cap succeed; one byte/count over cap and arithmetic overflow reject before decode allocation. Verify existing max_requests/max_resources also bound metadata rows. Preserve `max_payload_bytes` semantics.
9. Run representative capture/restore for existing admission-only payload to demonstrate default ordering reconstructed once. Joined qualification retains its existing outer fixture proof independently.

Native V4 tests do not qualify a wire V4 codec, automatic Ref/Image selection, consumer compatibility, parent pin, or full E2 acceptance. Those remain later barriers after this native contract and source are accepted.
