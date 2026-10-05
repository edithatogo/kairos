# C2.0 transactional transit execution v1

Status: reviewed architecture; exact document awaits final independent readback.
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
canonical UTF8 string. Constructors/getters must be completed in the API review
packet before implementation, never inferred from display IDs. Errors include
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
route fields. Exact accessor names remain part of final API review.

## Separate actor carrier and actual task

Transit progress is a separate actor-domain context, not service restart template.
Track03 extends create_actor_domain_context's role checks to permit Plan alongside
View, preserving one carrier per actor and bound event-kind validation. Runtime
registers hooks before work starts. Carrier creation retains a cloneable context
in the adapter and passes a clone because creation consumes C even on Err.

A TransitContext owns active actual service WorkId, immutable RoutePlan, segment
index/elapsed/remaining ticks, phase and exact AcquireIntent. Its actor carrier
may be a model-selected staff actor; AcquireIntent.owner remains actual task owner.
Carrier ownership and selected task/resource relationship must be validated by
model adapter, never inferred from colliding entity IDs. Separate carrier avoids
Restart rebuilding completed transit. Existing carrier may be reused only after
its previous route/claim is terminal; overlapping movement fails explicitly.

Nonzero Micro starts by scheduling the carrier's domain event at now plus first
positive segment duration. The planned callback returns next progress and emits
next domain event, or an actual timed Acquire for the service task at arrival.
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
