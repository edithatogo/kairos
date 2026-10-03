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
