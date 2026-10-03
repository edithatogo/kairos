# Q1 concrete experimental Rust surface — coordinator proposal

Add `flow` module re-exported from kairo-ecs-des. Legacy DESContext/Resource unchanged.
FlowRuntime owns one private Scheduler, World and ComponentRegistry. ECS stores
resource capacities, queues, active allocations and requests. Public queries return
cloned canonical snapshots, never mutable world/registry references.

Identifiers ResourceId/RequestId wrap generational EntityId; LeaseId contains
RequestId plus checked monotonic revision. FlowError: InvalidEntity,
InvalidResource, InvalidRequest, TerminalRequest, InvalidLease, CapacityInUse,
ResourceInUse, PastCommand, CounterOverflow, InvalidState.

FlowRuntime::new(); spawn_actor()->Result<EntityId,FlowError>;
create_resource(total:u32)->Result<ResourceId,FlowError>;
submit(resource,owner,at:SimTime)->Result<RequestId,FlowError> validates before
allocating and schedules admission (never promises synchronous grant).
step()->Result<Option<FlowDispatch>,FlowError> dispatches one internal event;
FlowDispatch { event:EventId, at:SimTime, records:Vec<LifecycleRecord>, error:Option<FlowError> }
returns every causal transition, each LifecycleRecord includes dispatch EventId and
checked contiguous transition ordinal. Rejected stale buffered commands emit an
error outcome and change no authoritative ECS state.
run_for(max_events:u64)->Result<FlowRun,FlowError> with FlowRun containing
dispatches and budget_exhausted; bounded execution retains all outcomes and pending work.
request(id)->Result<ResourceRequest,FlowError>;
resource(id)->Result<ResourceSnapshot,FlowError> (sorted handles).
release(lease,at)->Result<(),FlowError> validates and schedules release; duplicate
pending release rejected. Dispatch releases once and fills FIFO waiting claims.
set_capacity(resource,total)->Result<(),FlowError> buffers control command at now; reject shrink below active on submission and
revalidate on dispatch; no hidden eviction.
remove_resource(resource)->Result<(),FlowError> reject nonempty active/queue or
pending requests; otherwise buffer removal at now; revalidate dispatch then remove every owned
component before world despawn.
despawn_actor(owner)->Result<(),FlowError> buffers at now; dispatch cleans actor requests/allocations,
invalidates leases; marks pending events stale; no old-generation mutation.
now()->SimTime. LifecycleRecord includes request, resource, at, state, optional lease.
RequestState Pending/Queued/Active/Released/Cancelled. Timed work and priority,
deadlines, preemption are future Q2/Q3, not represented as completed.

Operation cap u32::MAX before World spawn/despawn and Scheduler schedule, and
lease counter checked before grants; supported facade prevents raw counter wrap.
No public mutable core exposure. Overflow checks occur before state writes.
Counter test seam uses internal unit tests, no public counter mutation API.
Internal COMMAND_DISPATCH code 4000 only; no caller eventkind ingress.
Before each dispatch commit preflight all derived grants, records, counter/entity
operations; if overflow would occur consume rejected command with error outcome
and otherwise leave authoritative state unchanged. Resource create/spawn are
setup entity creation, not runtime resource availability transitions.
No serializers, portable checkpoint or binding exposure. Experimental API release hold.

Cleanup cancels all owner pending/queued/active requests in canonical request
order, retains terminal request entities, then arbitrates affected resources in
canonical resource order. Admission sequence assigned at Submit dispatch, not
submit call. None means scheduler empty only; stale events return a dispatch.
Consumed/rejected release always clears its reservation. No RNG/core ordering/
Arrow or language binding changes; release held. Independent Track03/01/25 role
review found no remaining architecture blocker, subject to these required tests.

# Remaining Q1.2 types — proposal

Within experimental Flow module, add opaque WorkId(EntityId), WorkSpec storing
owner, original duration SimDuration, continuation registration key String and
optional request association. WorkContext<C> owns typed C inside ComponentRegistry.
create_work<C:'static>(owner,duration,registration:&str,context:C)->Result<WorkId,FlowError>;
validate live actor, nonempty stable key, same key consistently registered to same
Rust TypeId (runtime-owned lookup only; no portable schema promise), and creation
cap before writes. work(id)->Result<WorkSpec,FlowError> returns clone;
work_context<C:'static>(id)->Result<&C,FlowError> returns read-only typed context.
submit_work(resource,owner,work,at)->Result<RequestId,FlowError> validates work
owner and unused association, existing resource/actor/time/counters first. Work
association is admission bookkeeping; grant remains asynchronous. Duplicate
work association rejected transactionally. submit() remains manual lease-only.
Actor despawn cancels all requests first, removes every owned WorkContext via a
registered typed cleanup function and WorkSpec, then despawns work and actor with
complete counter preflight. Terminal requests remain queryable but old WorkIds
cannot resolve recycled entities. No timed completion or portable codecs yet.

PriorityKey { level:i32, enqueue_sequence:u64, request:RequestId } derives Ord.
ClaimQueue<K=PriorityKey> stores ordered BTreeSet<K> index. Current submit assigns
level0 at committed admission. RequestState authoritative; queue indexes handles
only. Q2 owns public priority/repriority/deadline admission and conformance.
Legacy Resource unchanged. Public API Rust-only experimental, release-held;
no core/RNG/Arrow/binding changes. Same-tick budget remains explicit Q4 followup.

Independent review refinements: ResourceRequest stores priority_level=0 and
work:Option<WorkId>, so the ordered index is derivable from authoritative state.
Request/work association is written only after all admission validation and
counters succeed, and retained after termination (no implicit work reuse). Actor
cleanup preflights every work despawn plus actor in canonical work-ID order.
Internal typed cleanup functions remove components only, never model callbacks.
Arbitrary context destructors are trusted in-process code; panic rollback is not
promised. Keys validate context typing only, not executable handler registration.
Wrong context type and stale WorkId return InvalidWork without mutation; &C does
not prohibit user context interior mutability. Exact RNG/order/Arrow/bindings
unchanged. Timed work and handler dispatch remain Q3/Q4 qualification.

Q1 allocation records store lease, request, owner, optional work, priority,
granted_at, segment_started_at, and completion_at=None for manual leases. The
active ECS component is indexed by LeaseId; resource inspection returns canonical
allocation records as well as existing lease IDs. Q3 owns timed completion dates.
