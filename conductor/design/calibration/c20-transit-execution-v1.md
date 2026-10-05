# C2.0 transactional transit execution v1

Status: test-interface contract accepted after independent owner readback and
coordinator resolution of the explicit-start correction; runtime remains absent.
Track03 owns production geometry/actor route execution and the DES planned hook.
Track21 owns calibration purpose streams and cryptographic receipt adapter.
This contract freezes future tests; it does not assert these APIs already exist.

## Opt-in transaction hook; legacy contracts preserved

```rust
impl FlowRuntime {
    pub fn register_domain_plan_hook<C:'static>(&mut self,
        registration:&str, kind:EventKind,
        planner:for<'a> fn(&'a C,&'a FlowCallbackSnapshot,FlowWorldView<'a>,
                          &'a mut FlowCommandSink)->Result<C,FlowError>)
        -> Result<(),FlowError>;
}
```

Registration follows existing context-type/domain-kind/duplicate rules. Planner
reads current C, emits bounded commands and returns a staged replacement C. Its
Err wins over buffered command errors and maps to the existing rejected receipt
with failed_ticket=None. On Ok, existing sink-poison/whole-batch preflight rules
apply. Commit commands and replace C only after complete preflight succeeds.
Rejected plan/batch leaves actual C, claims and queued commands unchanged.
No new FlowBatchReceipt variant or generic retry-token API is needed.

Delivered source event is consumed and time, dispatch budget and callback batch
identity advance even on rejection. This is not rollback of the whole dispatch.
The callback must be pure apart from its sink and returned owned C: interior
mutability/global side effects are not transactional. Existing mutable hooks keep
retained-context semantics. Public API adoption needs ADR/conformance/compatibility.

## Generic routing signatures and units

The reusable geometry/route primitives belong in existing kairo-ecs-abm, not DES
scheduling or a site policy. Geometry has no SHA dependency: emit canonical bytes,
then optional calibration adapter hashes them using existing sha2 at its1.88 floor.
Default DES/ABM1.76 is unchanged. No cryptographic identity supplied by an untrusted
caller may replace comparing actual graph/profile state.

```rust
pub struct TransitGraphV1 { /* immutable validated topology */ }
pub struct TransitEdge {
    pub id: EdgeId, pub from: NodeId, pub to: NodeId,
    pub length_mm:u64, pub allowed_modes:Vec<MovementModeId>,
}
pub struct MovementProfile { /* checked mode and NonZeroU64 speed */ }
pub struct RoutePlan { /* actual graph/profile identity and segments */ }
impl TransitGraphV1 {
    pub fn new(version:u32,nodes:Vec<NodeId>,edges:Vec<TransitEdge>)
        -> Result<Self,TransitError>;
    pub fn canonical_bytes(&self)->Vec<u8>;
    pub fn route(&self,origin:NodeId,destination:NodeId,
                 profile:&MovementProfile,ticks_per_second:u64)
        -> Result<RoutePlan,TransitError>;
}
impl MovementProfile {
    pub fn new(mode:&str,speed_mm_per_second:u64)->Result<Self,TransitError>;
}
```

NodeId/EdgeId are stable u64 newtypes (zero allowed); MovementModeId is an exact
canonical UTF8 string. Exact constructors/getters are listed below; never infer them from display IDs. Errors include
UnsupportedVersion, InvalidGraph, InvalidMovementMode, InvalidSpeed,
InvalidTickRate, UnknownNode, Unreachable, Overflow, InvalidProgress. Reject
duplicate nodes/edges/modes, dangling endpoints, invalid mode IDs, empty allowed
modes; validate all graph data before constructing immutable topology.

Edges have nonnegative integer millimetres and allowed modes; profile speed is
positive and uniform across one route. Route ordering is lexicographic
(distance_mm,hop_count,full_edge_id_sequence), with simple paths. This refinement
prevents zero-cost cycles from winning. Origin=destination is a valid empty route.
Checked u128 distance sum and ceil(distance*ticks_per_second/speed) determine total
ticks. Compute each segment's tick increment from cumulative ceiling differences;
sum exactly equals total. Zero increments consume segments without extra events.
Check multiplication/addition overflow; no float or hidden unit conversion.

Canonical bytes encode a schema/domain tag, graph version, units mm, sorted node
IDs and edges sorted by stable edge ID, endpoints/length and sorted allowed-mode
bytes with length framing. Route receipt also binds chosen profile/mode/speed and
ticks_per_second; they are not secretly part of topology hash. Calibration adapter
computes SHA256 from actual canonical bytes and checks graph/profile identity on
continuation. Permuted inputs yield identical graph bytes/hash/route timing.
RoutePlan supplies read-only origin/destination, edges, cumulative tick positions,
distance, duration, profile, tick rate and actual graph identity getters; no mutable
route fields. Accessor names below are frozen for test authoring; public adoption still requires review.

## Separate actor carrier and actual task

Transit progress is a separate actor-domain context, not service restart template.
Track03 extends create_actor_domain_context's role checks to permit Plan alongside
View, preserving one carrier per actor and bound event-kind validation. Runtime
registers hooks before work starts. Carrier creation retains a cloneable context
in the adapter and passes a clone because creation consumes C even on Err.

A TransitContext owns active actual service WorkId, immutable RoutePlan, segment
index/elapsed/remaining ticks, phase and DES FlowAcquireCommand (no ABM -> calibration dependency). Its actor carrier
may be a model-selected staff actor; AcquireIntent.owner remains actual task owner.
Carrier ownership and selected task/resource relationship must be validated by
model adapter, never inferred from colliding entity IDs. Separate carrier avoids
Restart rebuilding completed transit. Existing carrier may be reused only after
its previous route/claim is terminal; overlapping movement fails explicitly.

Nonzero Micro creates/reuses a Ready carrier and schedules an initial ordinary
domain start event at AcquireIntent.at. Only its accepted planned callback changes
Ready to Moving and schedules the first positive progress event at actual start
plus segment duration. The planned callback then emits each next progress event,
or an actual timed Acquire for the service task at arrival. A rejected start leaves
Ready unchanged; explicit retry targets that same start on the retained carrier.
No useful travel is inferred before the accepted start.
Only accepted arrival batch changes context phase and WorkSpec.request together.
Macro and explicit Zero Micro schedule no transit events or RNG draws. Deterministic
graph travel uses no RNG; later empirical transit sampling uses Transit purpose
only. Behavior purpose and intrinsic Service stream remain separate.

Bound adapter stores carrier/runtime identity, exact in-flight EventId, kind and
priority. On a matching rejected dispatch it verifies lineage first and explicit
source event/receipt/work/request state, then may retry same carrier/kind at now.
Replace pending EventId only after scheduling succeeds; repeated retry of consumed
attempt rejects. Accepted arrival reconciles actual request/admission receipt with
stored intent and closes pending movement. Lost bound/dispatch state is unavailable
recovery, not a reason to manufacture a new event. No automatic retry loop.

## Required behavioral fixtures

Planner Err and poisoned/invalid batch leave plain context equal and commit no
commands; source event/time/budget/batch still advance. Accepted planner commits
both C and actual claim. Legacy mutable-hook rejection still retains context.

Actor carrier Plan registration works; duplicate carrier, wrong kind/role and
foreign runtime fail. Real route interruption/retry keeps progress, emits one
arrival, and never duplicates actual service claim. Restart of service leaves
carrier route unchanged. Zero-route Macro/Micro pairs have equal intrinsic work,
completion/outcome/draw positions. Nonzero route adds only observed transit and
reconciles travel+queue+work. Route ties/zero cycles/permuted input and overflow
fixtures use actual graph/Flow execution, not independent accounting counters.

Full runtime checkpoint/restore remains Track22: provider snapshots and retained
in-memory carrier are insufficient. Complete scheduler/resource/work/policy/route/
stream continuation, public API review and native hosted gates remain mandatory.


## Exact routing/carrier accessors and control ingress

```rust
impl NodeId { pub const fn new(value:u64)->Self; pub const fn value(self)->u64; }
impl EdgeId { pub const fn new(value:u64)->Self; pub const fn value(self)->u64; }
impl MovementModeId {
    pub fn new(value:&str)->Result<Self,TransitError>;
    pub fn as_str(&self)->&str;
}
impl MovementProfile {
    pub fn mode(&self)->&MovementModeId;
    pub fn speed_mm_per_second(&self)->std::num::NonZeroU64;
}
impl RoutePlan {
    pub fn origin(&self)->NodeId; pub fn destination(&self)->NodeId;
    pub fn segments(&self)->&[RouteSegment];
    pub fn distance_mm(&self)->u128; pub fn duration(&self)->SimDuration;
    pub fn profile(&self)->&MovementProfile; pub fn ticks_per_second(&self)->u64;
    pub fn graph_version(&self)->u32; pub fn graph_canonical_bytes(&self)->&[u8];
}
impl RouteSegment {
    pub fn edge_id(&self)->EdgeId;
    pub fn from(&self)->NodeId; pub fn to(&self)->NodeId;
    pub fn length_mm(&self)->u64;
    pub fn start_offset(&self)->SimDuration; pub fn end_offset(&self)->SimDuration;
}
pub struct TransitContext { /* private cloneable route/progress/command state */ }
pub enum TransitPhase { Ready,Moving,Paused,Arrived }
pub struct TransitProgress {
    pub segment_index:usize,pub useful_elapsed:SimDuration,
    pub remaining:SimDuration,pub phase:TransitPhase,
}
impl TransitContext {
    pub fn new(flow:&FlowRuntime,route:RoutePlan,acquire:FlowAcquireCommand,
        start:SimTime)->Result<Self,TransitError>;
    pub fn service_work(&self)->WorkId;
    pub fn phase(&self)->TransitPhase;
    pub fn progress_at(&self,at:SimTime)->Result<TransitProgress,TransitError>;
    pub fn arrival_ticket(&self)->Option<FlowCommandTicket>;
    pub fn next_progress_ticket(&self)->Option<FlowCommandTicket>;
    pub fn plan<'a>(current:&'a Self,snapshot:&'a FlowCallbackSnapshot,
        view:FlowWorldView<'a>,sink:&'a mut FlowCommandSink)->Result<Self,FlowError>;
}
pub enum FlowDomainControl { Pause,Resume }
impl FlowRuntime {
    pub fn schedule_domain_control(&mut self,work:WorkId,kind:EventKind,
        action:FlowDomainControl,at:SimTime,priority:i32)->Result<EventId,FlowError>;
}
// Additive cause, retaining existing Domain { kind } unchanged:
// FlowCallbackCause::DomainControl { kind:EventKind, action:FlowDomainControl }
```

Node/edge/mode/profile/route immutable data may Clone; handles with live mutable
stream ownership may not. TransitPhase/control/progress derive Debug/Eq/PartialEq;
control and phase Copy. TransitContext Clone contains no RNG owner and is used
for pure planned replacement and failed-creation retry. new validates actual
service work owner/timed command/work link/Pending state, route and future start;
InvalidProgress covers invalid work/command/start without inventing IDs. It
retains no calibration type. Route getters expose validated actual state only.

DomainControl targets the same bound kind on a Plan carrier. Registration/role,
past time, budget/counter and lineage checks precede event allocation. Legacy
mutable hooks cannot receive controls through this new API. Controls are real
scheduler events, not synchronous arbitrary context mutation. Pause/Resume enum
is the minimal generic domain protocol; future directives need their own review.

At Pause delivery, compute checked movement elapsed up to that actual tick, retain
current segment/remaining ticks and enter Paused. Existing progress events stay
queued. While paused they perform no movement/claim; matching consumed event clears
outstanding status. Resume recomputes current due from actual now+remaining, then
reuses an outstanding event only if its due is unchanged. Otherwise emit one new
progress event and treat older events as stale. Expected due/phase and accepted
receipt ticket track current progress; stale events never advance another segment.

If remaining is zero on Resume, process contiguous zero-duration segments and
emit one arrival claim in that control's accepted transaction. Pause/Resume before
planned route start preserve zero useful elapsed; invalid repeated control yields
planner rejection with unchanged C. Ties at the same tick follow existing scheduler
priority/sequence. No automatic cancellation/reordering is introduced.

Required actual-runtime interruption oracle: Pause before the planned arrival,
consume old arrival while paused with no request, Resume, then observe exactly one
accepted arrival/acquire and intrinsic completion. Assert useful movement plus
paused time plus queue plus useful service separately, remaining ticks unchanged
while paused, and no duplicate on repeated controls/stale events. Source events
remain consumed on rejection; retain sample and context and explicitly retry.


## Reviewed explicit-start and pre-start pause correction

TransitContext retains original_start_at, paused_from (Ready or Moving), current
segment elapsed/remaining, and pending start/progress due plus outstanding status.
Pause from Ready records zero useful movement. A delivered start while Paused is
a no-op that clears its outstanding status. Resume from Ready reuses a still
outstanding future start event; if it was consumed, emit a new start event at
max(actual now, original_start_at). The accepted start then begins the full leg.
Resume from Moving retains remaining ticks and follows the due/reuse rules above.

progress_at before accepted start returns zero useful movement and full remaining
route duration, even if planned start time passed while paused/rejected. Repeated
start/control events cannot trigger a second movement or arrival. Contract fixture
must pause before a future start, consume the stale start while paused, resume,
and prove full movement begins at the accepted new start and one actual claim.
Existing post-start pause/stale-arrival fixtures remain mandatory.

Independent review identified and resolved this correction without weakening
Macro/Zero no-transit, actual Flow scheduling or interruption requirements. This
closes interface definition only; red fixture authoring and runtime/native gates
are still required for C2.0/C2.1.
