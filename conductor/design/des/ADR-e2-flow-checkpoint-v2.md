# ADR: explicit Flow checkpoint V2 for grant eligibility

Status: internally reviewed contract preparation (root and GPT-6.1-sol in
Track03/Track22 owner-review roles); no implementation authority or acceptance. Rust/Cargo 1.99.0 only. Base architecture63cd1cd;
current grant-gate implementation is still under native-oracle review.
A reviewed runtime commit and separately bound packets precede implementation.

## Problem and scope

V1 cannot encode resource grant flags or pending gate commands. Its interim
writer rejects that state rather than losing it. E2 needs lossless continuation,
including a pending same-tick boundary and suspended work. Keep one Flow clock.
Track03 owns DES checkpoint semantics; calibration/Track22 owns downstream joins.
No core scheduler, FFI, Arrow schema, dependency or stable API promotion.

## Exact additive API proposal

All new types and methods are experimental/doc-hidden like existing checkpoint
APIs. Existing methods and V1 DTOs retain signatures and behavior.

```rust
pub const FLOW_CHECKPOINT_VERSION_V2: u32 = 2;

impl FlowRuntime {
    pub fn capture_checkpoint_v2(
        &self, codecs: &FlowCheckpointCodecs, limits: FlowCheckpointLimits,
    ) -> Result<FlowCheckpointV2, FlowCheckpointError>;
    pub fn restore_checkpoint_v2(
        image: FlowCheckpointV2, codecs: &FlowCheckpointCodecs,
        limits: FlowCheckpointLimits,
    ) -> Result<Self, FlowCheckpointError>;
    pub fn restore_checkpoint_v2_with_rebind(
        image: FlowCheckpointV2, codecs: &FlowCheckpointCodecs,
        limits: FlowCheckpointLimits,
    ) -> Result<(Self, FlowCheckpointRebindV1), FlowCheckpointError>;
}
impl FlowCheckpointV2 {
    pub fn encode_wire_v2(&self, limits: FlowCheckpointWireLimits)
        -> Result<Vec<u8>, FlowCheckpointWireError>;
    pub fn decode_wire_v2(bytes: &[u8], limits: FlowCheckpointWireLimits)
        -> Result<Self, FlowCheckpointWireError>;
}
```

FlowCheckpointV2 derives Clone, Debug, Eq, PartialEq. It contains every field
listed in FlowCheckpointV1 at63cd1cd with the same name/type/order, except
version must be2 and commands becomes Vec<(EventId, FlowCommandV2)>. Append:

```rust
pub resource_grant_flags: Vec<(ResourceId, bool)>,
pub started_frontier: Option<SimTime>,
```

FlowCommandV2 derives Clone, Copy, Debug, Eq, PartialEq and repeats every V1
command variant unchanged, plus GrantEnabled(ResourceId, bool). Reusing legacy
field types does not make the entire V2 image a V1 image. No V1 DTO field or
variant is added, deleted or repurposed. FlowError and V1 nested dispatch/error
wire tags stay unchanged. Existing checkpoint error categories suffice.

Runtime storage is a sparse grant_enabled map (missing means true), a private
pending (ResourceId,SimTime)->EventId index and GrantEnabled command. Frontier
is equivalent to None when dispatched_events==0, otherwise Some(scheduler.now).
V2 capture enumerates all live resources in canonical ResourceId order and emits
exactly one flag each; missing V2 rows never default true. Restore may populate
the private map densely; its representation is not a public state promise.
Existing built-in component stores remain unchanged, so FlowBuiltInStoresV1 is
reused. FlowCheckpointRebindV1 is an ID/runtime-identity view, not a Flow image;
its meaning and context/template decoder signatures remain unchanged.

## Pending gate index and atomic validation

Do not serialize redundant pending rows. EventId is the command row key; tick
and priority are in the corresponding live scheduler entry; resource and enabled
are command payload. Derive the index from those complete data. Capture must
independently derive it and require exact equality with the live runtime cache.
Before format routing, validate every sparse grant_enabled key names a live
generational resource; a stale true or false entry must not be silently omitted
when enumerating live resources. Validate the pending index/command join before
choosing V1 or V2, so routing cannot hide inconsistent cached state.
Restore validates the join, then builds the cache in a detached staged runtime.

Validate unique EventIds, unique resource/tick boundaries, live generational
resources and one-to-one complete command/live scheduler joins. Validate the
existing Flow event-kind/owner conventions for the new command exactly as for
other commands. No gate may coexist with a pending Remove for its resource.
Flags exactly cover the live resource set, sorted and unique. Frontier must
equal the value derived from scheduler dispatch count and now.

Predeclared pending boundaries at the CURRENT started tick are valid: completion
at T may dispatch before a gate at T. Restore must not reuse fresh-admission
frontier rejection to reject these persisted commands. Commands before now are
invalid under existing scheduler validation. Semantic rejection advances the
frontier; precommit budget halt does not consume the pending command/index.

Apply all existing request/work/context/handler/component/scheduler invariants
without weakening them. No stripped V1 projection, discarded gate event, fake
legacy command or restore-then-reinsert workaround. Shared private validators
and read-only common views must retain every command and state field.

Decode yields detached owned data. Structural preflight and cross-field joins
run before invoking caller codecs or publishing a runtime/rebind. On any failure
no runtime escapes and caller-owned source state is unchanged. Trusted codec
side effects/panics retain existing limitations; no new rollback promise for them.

## Wire contract

Use distinct eight-byte magic b"KFLOWV2\0", wire schema u16=1 and image version
u32=2. Version and wire schema are separate discriminators. Little-endian scalar
widths and common-field encodings follow V1. Encode common fields in V1 field
order using complete V2 commands, then flags as count + (ResourceId,bool) rows,
then frontier as the existing option tag + SimTime encoding.

V2 command tags0–11 retain the V1 meanings and payloads. Tag12 encodes
GrantEnabled(ResourceId,bool), in that order. Boolean encoding accepts only0/1.
V1 magic, schema, command tags, nested FlowDispatch bytes and golden fixtures
are unchanged; V1 readers reject V2 magic. V2 readers reject V1 magic. A separate
version-aware dispatcher chooses the correct decoder from magic. Reject unknown
magic/schema/image version/tags, malformed booleans, truncation and trailing bytes.
Encoding validates canonical order and aggregate limits before allocation;
V2 decoding performs a separate complete scan/preflight pass before allocating
owned DTO collections. Scan tags, booleans, UTF-8, counts, aggregate limits and
trailing bytes across the entire payload, then run the owning decode. This is a
new V2 requirement: existing V1 decode constructs bounded collections while
decoding and is not claimed to supply this full separate pass. Cross-field
reference joins may run on bounded detached DTOs before codecs/publication.

Reuse FlowCheckpointLimits and FlowCheckpointWireLimits without new fields or
increased defaults. Flag rows count against max_resources. Gate command and
derived-index counts count against max_commands. Pending release+despawn+gate
counts combined must respect max_pending_operations. New rows count in aggregate
max_total_records and wire/payload byte limits with checked arithmetic. The index
is derived data; cap it before allocation, without double charging serialized
rows that do not exist. All existing per-collection limits still apply.

## Lossless consumer routing proposal

Add public doc-hidden types deriving Clone, Debug, Eq, PartialEq:

```rust
pub enum FlowCheckpointImage { V1(FlowCheckpointV1), V2(FlowCheckpointV2) }
#[derive(Clone, Copy)]
pub enum FlowCheckpointRef<'a> {
    V1(&'a FlowCheckpointV1), V2(&'a FlowCheckpointV2),
}
impl FlowCheckpointImage {
    pub fn as_ref(&self) -> FlowCheckpointRef<'_>;
    pub fn encode_wire(&self, limits: FlowCheckpointWireLimits)
        -> Result<Vec<u8>, FlowCheckpointWireError>;
    pub fn decode_wire(bytes: &[u8], limits: FlowCheckpointWireLimits)
        -> Result<Self, FlowCheckpointWireError>;
    pub fn restore_with_rebind(
        self, codecs: &FlowCheckpointCodecs, limits: FlowCheckpointLimits,
    ) -> Result<(FlowRuntime, FlowCheckpointRebindV1), FlowCheckpointError>;
}
impl<'a> FlowCheckpointRef<'a> {
    pub fn scheduler(self) -> &'a SchedulerCheckpointV1;
    pub fn work_registrations(self) -> &'a [FlowWorkRegistrationV1];
    pub fn context_stores(self) -> &'a [FlowContextStoreV1];
    pub fn commands(self)
        -> impl Iterator<Item = (EventId, FlowCommandV2)> + 'a;
}
impl FlowRuntime {
    pub fn capture_checkpoint_image(
        &self, codecs: &FlowCheckpointCodecs, limits: FlowCheckpointLimits,
    ) -> Result<FlowCheckpointImage, FlowCheckpointError>;
}
```

The borrowed enum retains the entire original native image; common accessors do
not replace it with a reduced DTO. The commands iterator copies complete command
values without allocating or dropping variants, upgrading V1 commands losslessly
to V2 variants. Never project V2 to V1. Version-specific image encode/restore
dispatches to the corresponding owner implementation. Decode dispatches only by
known magic; unknown input fails before allocation.

Capture routing selects V2 when a disabled resource or pending gate command/index
requires it, otherwise V1 to preserve existing no-gate C2 bytes. Do not treat an
arbitrary V1 capture error as a signal to retry V2; structural errors remain
errors. Both formats enforce capture-cache consistency.

Baseline63cd1cd consumer inventory:

- c2_portable_checkpoint.rs captures/encodes V1 at268–271, decodes/restores at
  420–468, and invokes coherent-cut validation at330/545. Route complete images
  through the new owner methods; retain detached cross-owner validation.
- flow_bridge/checkpoint_wire.rs validate_coherent_cut takes a V1 image at94–99.
  It reads work registrations, context stores, live scheduler entries and full
  commands. Accept FlowCheckpointRef and match complete FlowCommandV2 variants.
- flow_bridge.rs V1-returning helper2734–2763 and image-mutation tests2850–2878
  require version-aware test fixtures, preserving missing-command negatives.
- c5_native.rs369–379 and shadow_native.rs63–75 capture an image to inspect the
  next live scheduler tick. Use capture_checkpoint_image plus scheduler view.
  Shadow byte checkpoints delegate to model methods166–192; keep clock checks.

The outer C2 envelope can remain unchanged, but old readers cannot read an inner
V2 section. ABM transit, bound/submitted work and coherent-cut checks remain
mandatory. No gate or command is discarded to preserve a legacy helper type.

## Oracles and bounded sequencing

Before implementation bind explicit source paths and exact1.99commands. Runtime
writer releases flow.rs/checkpoint.rs before checkpoint writers edit them.
Split native DTO/capture/restore, V2 wire, then consumers with explicit joins;
no worker implements a speculative API before its prerequisite source freezes.
Read-only consumer inventory and test-shape preparation may run in parallel.

Required native oracles: legacyV1 exactbytes/read and no-gate lifecycle parity;
V1 rejects disabled/pendingstate; V2 disabled and enabled/future-toggle captures;
checkpoint before/after gate dispatch; actual completion-first same-tick pending
gate restore; precommit budgethalt capture/restore; uninterrupted versus restored
full lifecycle/workprogress history, including Suspend identity and sampling.
Malformed flags, duplicate boundaries, orphan/mismatched commands/events, stale
resources, Remove conflicts, inconsistent frontier and all wire/limit errors must
reject before runtime publication. Include exactlimit/limit+1 checks and trusted
codec-call counters to prove preflight timing. No lossy acceptance fallback.

After V2 and consumer integration, qualify combined DES/core/ABM/calibration on
the exact integrated Rust1.99commit, then update the parent pin separately.
Parent single-runtime binding retains E1 parity and all88 accepted E2 hand
fixtures. Focused native passes do not close E2/MVP/empirical/clinical/platform/
hosted/release gates. No remote push or publication is authorized by this ADR.

## Red-team review

- Duplicate state can disagree: derive the index, validate the live capture cache
  and all restore joins; flags/frontier are validated against authoritative state.
- Shared V1 validators can erase gates: require full variant-aware inputs and
  prohibit projected V1 restore. Unchanged V1 goldens remain independent checks.
- Mid-tick state may look late: persisted admission is not fresh ingress; allow
  current-tick gates with valid scheduler identities.
- Decoder allocation or codec abuse: bounded full preflight, checked counts and
  detached construction before callbacks or publication; no new unsafe code.
