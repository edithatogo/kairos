# Explicit native Flow checkpoint V3 contract

Status: Owner contract reviewed by root and GPT-6.1-sol. Preparation only;
runtime acceptance and separately bound implementation dispatch remain required.
Authoring base: 2fb908cae59f2e21285fb0d5ace437a9e2cb11f6. Rust/Cargo 1.99.0 only.
Track03 owns Flow and checkpoint invariants; Track22 owns downstream composite
joins. The reviewed deferred-allocation ADR remains the behavioral authority.

## Prerequisites and staged ownership

Deferred runtime is still under qualification. Bind its accepted commit, exact
API/cache names and read-only input hashes before preparing an implementation
packet. The current reviewed prototype uses `allocation_modes` and
`pending_allocation_flushes`; these names are context, not accepted source proof.
Future commands, worktree, toolchain and inputs resolve at dispatch.

The explicit native implementation owns four inseparable Rust paths:

- `crates/kairo-ecs-des/src/flow/checkpoint.rs`
- `crates/kairo-ecs-des/src/flow/checkpoint/v2.rs`
- new `crates/kairo-ecs-des/src/flow/checkpoint/v3.rs`
- new `crates/kairo-ecs-des/tests/flow_checkpoint_v3.rs`

Result and retained logs make six owned paths. This requires an explicit bounded
size exception. `flow.rs`, wire.rs, existing tests, calibration, manifests and
lockfiles remain read-only. No duplicate runtime or allocator is introduced.

Stage order is explicit native V3, explicit V3 wire, then routed image/ref and
coherent compilation join. This native leaf does not widen FlowCheckpointImage,
FlowCheckpointRef or their command iterator, and adds no wire methods or stubs.
Old routed capture continues to reject Deferred state until the routing join.
Later widening requires ownership of every exhaustive match, including wire
encoding and the calibration coherent validator; it must never filter Flush out
of a supposedly complete view. Qualify actual V3 consumer behavior separately.

## Exact native API

All new APIs are experimental/doc-hidden, matching existing checkpoint APIs.

```rust
pub const FLOW_CHECKPOINT_VERSION_V3: u32 = 3;

impl FlowRuntime {
    pub fn capture_checkpoint_v3(
        &self, codecs: &FlowCheckpointCodecs, limits: FlowCheckpointLimits,
    ) -> Result<FlowCheckpointV3, FlowCheckpointError>;
    pub fn restore_checkpoint_v3(
        image: FlowCheckpointV3, codecs: &FlowCheckpointCodecs,
        limits: FlowCheckpointLimits,
    ) -> Result<Self, FlowCheckpointError>;
    pub fn restore_checkpoint_v3_with_rebind(
        image: FlowCheckpointV3, codecs: &FlowCheckpointCodecs,
        limits: FlowCheckpointLimits,
    ) -> Result<(Self, FlowCheckpointRebindV1), FlowCheckpointError>;
}
```

Export V3 types through checkpoint.rs. Existing signatures and public DTO fields
remain unchanged. Reuse FlowCheckpointRebindV1 as the validated runtime/entity
identity mapping; it is not an image version. Context/template codec signatures
and trusted callback identifiers remain unchanged. Existing checkpoint error
categories suffice; do not add FlowError or old error wire variants.

FlowCommandV3 derives Clone, Copy, Debug, Eq, PartialEq. It repeats every
FlowCommandV2 variant with identical payload types and adds
`AllocationFlush(ResourceId)`. Lossless From conversions from V1/V2 commands
are allowed; any conversion back is private/fallible and rejects Flush.

FlowCheckpointV3 derives Clone, Debug, Eq, PartialEq. It contains every named
FlowCheckpointV2 field in the same declaration order and type, except version is
3 and commands is `Vec<(EventId, FlowCommandV3)>`. Append:

```rust
pub allocation_modes: Vec<(ResourceId, FlowAllocationMode)>,
```

For clarity, the complete common fields are config, callback_config,
next_batch_identity, budget_tick, budget_consumed, budget_halt, scheduler, world,
resources, requests, actors, actor_domains, work_registrations, contexts,
handlers, continuations, domains, builtins, context_stores,
restart_store_manifest, restart_stores, commands, notifications,
pending_releases, pending_despawns, created, destroyed, scheduled,
next_admission, next_lease, resource_grant_flags and started_frontier.
No pending flush index rows are serialized: derive the cache from the complete
command and live scheduler join. Native DTO persistence is not wire acceptance.

## Complete common pipeline and legacy behavior

The private Parts representation becomes a complete V3 superset, retaining all
commands, modes, flags, frontier, builtins, contexts and restart data. Keep
capture/restore validation shared; do not copy a second large restore engine.
No V3-to-V2 stripped image, fake old command, restore-then-reinsert workaround or
lossy Deref projection is permitted. Full command traversal handles Flush
explicitly wherever command identity, ownership, liveness or resource causality
is validated. Legacy format emission is private and fallible.

V1 and V2 inputs upgrade losslessly into all-Eager modes only after an
allocation-free borrowed normalization preflight. Preserve an origin/version
marker where validation or limit behavior differs. V1 native restore continues
to accept legal unsorted resource vectors; synthesized mode/flag rows may be
sorted without changing other legacy input order or acceptance. V2 upgrade must
retain every gate, flag, frontier and command. Do not impose V3-only canonical
resource-vector rejection on accepted V1/V2 inputs. Exact mode coverage uses the
validated resource set, independently of legacy resource-vector ordering.

Explicit V1, explicit V2 and current routed capture fail before codecs when any
Deferred mode or pending Flush cannot be represented. Preserve accepted error
precedence and successful Eager V1/V2 behavior. Do not implement old capture as
capture-V3-and-retry, and do not invoke codecs before deciding old-format
eligibility. Explicit V3 capture uses the shared full-state path and supports
Deferred state without bypassing its cache/bounds checks. V1/V2 restoration
still produces Eager resources and an empty pending flush cache.

## Bounds before normalization, allocation and codecs

Use existing FlowCheckpointLimits; no new limit field is necessary. Borrowed
preflight for BOTH V1 and V2 must precede synthesized vectors, command
conversion, scheduler snapshots, BTreeMap/BTreeSet indexes and reference joins.
V3 restore likewise bounds its complete input before allocating validation
structures. Counting loops and checked integer arithmetic may inspect borrowed
rows; no Vec sized by sparse_slots or supposedly harmless sorting allocation is
allowed in this phase.

Bound world slots/free/live IDs, scheduler entries, resources, requests, actors,
actor domains, works, commands and notifications by their existing limits.
Independently bound resource_grant_flags and allocation_modes by max_resources.
Check aggregate registrations including nested callback IDs, component rows and
sparse slots including context/restart stores, key bytes including factory and
registration keys, and payload bytes including context/restart rows. Preserve
existing codec-specific budgets. Aggregate pending operations for V3 is checked
release count + despawn count + GrantEnabled command count + AllocationFlush
command count, bounded by max_pending_operations. Preserve accepted V1 and V2
limit semantics and error precedence rather than importing stricter V3 behavior
into their normalization wrappers. All sums use checked arithmetic.

Capture validates resource/command/pending bounds before one reusable bounded
scheduler snapshot and lookup structures. Reuse that snapshot throughout cache
validation and DTO construction. Precompute pending Remove membership once;
avoid scanning all commands for each gate/flush. Joins should be bounded linear
or ordered-map work, never O(flushes * commands). Final native allocation and
codec payload ownership remain bounded by the existing pipeline.

## Complete structural joins before codecs

Before capture codecs, validate dense/sparse allocation-mode cache identity:
every cache key, including Eager entries, must refer to a live generational
resource. The exact representation follows the accepted runtime; never default
away malformed cache rows. Enumerate exactly one canonical mode row per live
resource. Verify pending flush cache equals the index independently derived
from commands and scheduler, and retain all accepted grant-cache checks.

Before restore codecs or publishing runtime/rebind:

- Version is 3. Mode rows are sorted/unique and exactly cover live resources,
  including matching generations; neither missing nor extra rows default Eager.
- Every Flush targets a live Deferred resource. It has one matching live
  scheduler entry of `EventKind::custom(FLOW_COMMAND_DISPATCH_EVENT_KIND)`
  (current accepted convention 4000) and entity None. Bind the actual constant
  from accepted runtime source; do not invent a new event kind in this leaf.
- Event IDs and complete command/live scheduler membership obey existing Flow
  cardinality and ownership rules. Unique (resource,tick) flush boundaries are
  required. Flush cannot coexist with pending Remove for that resource.
- Derive pending flush index from event key, scheduler tick and command resource.
  Cache rows are not a second authority. Validate grant and flush joins together
  without dropping either command family.
- Flush at the current started tick is legal; fresh admission permits it.
  Pending same-tick calendar boundaries retain accepted V2 restoration rules.
  Events before scheduler.now are invalid under existing scheduler validation.
- started_frontier equals None at zero dispatched events, otherwise scheduler.now.
  Mode, flag, command, request/work/queue, lease, registry, callback, context and
  restart joins all retain existing validation. New state cannot weaken them.

Build a detached staged runtime and its cache only after structural validation.
No runtime/rebind escapes on error. Trusted codec side effects/panics retain
existing limitations; do not promise codec rollback. Failures before codecs are
proved with counters and a real valid-codec positive control.

## Native acceptance matrix

All cases are actual runtime cuts and explicit V3 capture/restore; no DTO-only
positive or unsupported legacy image substituted for Deferred state.

| Family | Required proof |
| --- | --- |
| Complete state | Nonempty contexts/restart data, Eager and Deferred resources, flags, pending gates/Flush and actual work/queues survive capture/restore and recapture equality; complete uninterrupted versus restored dispatch/state suffix. |
| Interior drain | Capture after one real tick-6 completion while another completion/readiness/Flush is pending; restore and close ingress then flush. Both completion insertion orders preserve low[1,4), urgent[4,6), B[6,7), low[7,14), no zero-length low resume/preempt at6. |
| Preflush halt | Capture real precommit budget halt, restore full pending command/event/index/work/progress/clock/counter state and equal repeated halt. Budget-halt metadata is retained; no impossible progress under unchanged budget. |
| Bounds | Exact cap succeeds and cap+1 input fails before codecs for resources, modes and combined gate+Flush+release+despawn pending counts. Include existing aggregate registration/key/payload/component/sparse budgets and a valid positive control. |
| Malformed modes | Missing, extra, duplicate, unsorted and stale-generational rows reject before codecs; wrong target mode does not silently become Eager. |
| Malformed Flush | Wrong resource/mode/event ID/kind/entity/time, duplicate resource/tick, absent/nonlive scheduler entry and pending Remove conflict reject before codecs. Include full command membership, not just matching one event. |
| Capture corruption | Private tests independently corrupt mode keys and pending Flush cache event/resource/tick/non-Flush command; reject before codecs without mutating source. Valid snapshot/control proves nonvacuity. |
| Legacy compatibility | Legal unsorted V1 resources restore with the same native tail; nontrivial V2 gates/contexts upgrade and recapture identically. Explicit Eager V1/V2 and routed selection/bytes remain unchanged, existing native/wire regressions retained. Deferred old captures reject before codecs. |

Compare actual command/cwd/base/toolchain/seed/input and source hashes in retained
receipts; planned checks are not passes. Qualifying commands and exact resource
cache representation resolve in the separately reviewed implementation packet.
Tests may use owned private unit modules for full cache witnesses; do not add
public unchecked mutation APIs or unsafe allocation instrumentation.

## Remaining work and acceptance boundary

This contract does not qualify runtime, native V3, wire V3, complete routed V3
views, calibration consumers, parent pin, ED recovery or the 88 fixture bindings.
Explicit wire later owns full nonallocating scan/measure/encode/decode and keeps
V1/V2 goldens. The routing join later owns capture selection, image/ref and
complete command view, wire dispatcher and coherent compilation changes. Parent
logical admission/selection state must be included in a separately reviewed
composite recovery image; native Flow state alone does not certify ED recovery.
No placeholder method or hidden uncheckpointable window closes these tasks.
