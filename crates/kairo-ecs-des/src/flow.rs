//! Experimental, single-world Flow facade. No portable checkpoint promise.
use kairo_ecs_core::Scheduler;
use kairo_ecs_state::{ComponentRegistry, World};
use kairo_ecs_types::{
    EntityId, EventId, EventKind, ScheduleRequest, SimDuration, SimTime, StepOutcome,
};
use std::any::TypeId;
use std::collections::{BTreeMap, BTreeSet};

/// Generational resource identity.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ResourceId(EntityId);
/// Capacity is ECS-owned; available capacity is always derived.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceCapacity {
    pub total: u32,
}
/// Experimental Flow errors; messages are not a stable compatibility key.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum FlowError {
    #[error("invalid entity")]
    InvalidEntity,
    #[error("invalid resource")]
    InvalidResource,
    #[error("invalid request")]
    InvalidRequest,
    #[error("terminal request")]
    TerminalRequest,
    #[error("invalid lease")]
    InvalidLease,
    #[error("capacity in use")]
    CapacityInUse,
    #[error("resource in use")]
    ResourceInUse,
    #[error("past command")]
    PastCommand,
    #[error("counter overflow")]
    CounterOverflow,
    #[error("invalid state")]
    InvalidState,
    #[error("invalid work")]
    InvalidWork,
}

/// Generational request identity retained after termination.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RequestId(EntityId);
/// Allocation identity; an old lease cannot release its replacement.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct LeaseId {
    request: RequestId,
    revision: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequestState {
    Pending,
    Queued,
    Active,
    Released,
    Cancelled,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceRequest {
    pub resource: ResourceId,
    pub owner: EntityId,
    pub state: RequestState,
    pub admission_sequence: Option<u64>,
    pub lease: Option<LeaseId>,
    pub priority_level: i32,
    pub work: Option<WorkId>,
}
/// Ordering index derived from authoritative request fields.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PriorityKey {
    pub level: i32,
    pub enqueue_sequence: u64,
    pub request: RequestId,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimQueue<K: Ord = PriorityKey> {
    requests: BTreeSet<K>,
}
impl<K: Ord> Default for ClaimQueue<K> {
    fn default() -> Self {
        Self {
            requests: BTreeSet::new(),
        }
    }
}
/// Opaque work identity; owned context is live in-process state only.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct WorkId(EntityId);
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkSpec {
    pub owner: EntityId,
    pub original_duration: SimDuration,
    pub context_type_key: String,
    pub request: Option<RequestId>,
}
struct WorkContext<C>(C);
fn cleanup_context<C: 'static>(registry: &mut ComponentRegistry, id: EntityId) {
    registry.remove::<WorkContext<C>>(id);
}
type ContextCleanup = fn(&mut ComponentRegistry, EntityId);
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ActiveAllocations {
    leases: BTreeMap<LeaseId, Allocation>,
}
/// Authoritative manual allocation; timed work is introduced in Q3.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Allocation {
    pub lease: LeaseId,
    pub request: RequestId,
    pub owner: EntityId,
    pub work: Option<WorkId>,
    pub priority_level: i32,
    pub granted_at: SimTime,
    pub segment_started_at: SimTime,
    pub completion_at: Option<SimTime>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceSnapshot {
    pub total: u32,
    pub available: u32,
    pub queued: Vec<RequestId>,
    pub active: Vec<LeaseId>,
    pub allocations: Vec<Allocation>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LifecycleRecord {
    pub request: RequestId,
    pub resource: ResourceId,
    pub at: SimTime,
    pub state: RequestState,
    pub lease: Option<LeaseId>,
    pub causal_event_id: EventId,
    pub transition_ordinal: u32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowDispatch {
    pub event: EventId,
    pub at: SimTime,
    pub records: Vec<LifecycleRecord>,
    pub error: Option<FlowError>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowRun {
    pub dispatches: Vec<FlowDispatch>,
    pub budget_exhausted: bool,
}
#[derive(Clone, Copy, Debug)]
enum Command {
    Submit(RequestId),
    Release(LeaseId),
    Capacity(ResourceId, u32),
    Remove(ResourceId),
    Despawn(EntityId),
}
#[derive(Clone)]
struct ResourceStage {
    capacity: ResourceCapacity,
    queue: ClaimQueue,
    active: ActiveAllocations,
}
/// Declarative admission; fields do not mutate runtime state until submit.
pub struct AcquireBuilder<'a> {
    runtime: &'a mut FlowRuntime,
    resource: ResourceId,
    owner: Option<EntityId>,
    work: Option<WorkId>,
    at: SimTime,
}
impl AcquireBuilder<'_> {
    pub fn owner(mut self, owner: EntityId) -> Self {
        self.owner = Some(owner);
        self
    }
    pub fn at(mut self, at: SimTime) -> Self {
        self.at = at;
        self
    }
    pub fn for_work(mut self, work: WorkId) -> Self {
        self.work = Some(work);
        self
    }
    pub fn submit(self) -> Result<RequestId, FlowError> {
        let owner = self.owner.ok_or(FlowError::InvalidState)?;
        self.runtime
            .submit_inner(self.resource, owner, self.work, self.at)
    }
}
/// Private shared scheduler/world/registry. Single process, experimental Rust API.
/// All runtime resource changes occur only at command dispatch boundaries.
pub struct FlowRuntime {
    scheduler: Scheduler,
    world: World,
    registry: ComponentRegistry,
    resources: BTreeSet<ResourceId>,
    requests: BTreeSet<RequestId>,
    actors: BTreeSet<EntityId>,
    works: BTreeMap<WorkId, ContextCleanup>,
    context_types: BTreeMap<String, TypeId>,
    commands: BTreeMap<EventId, Command>,
    pending_releases: BTreeSet<LeaseId>,
    created: u64,
    destroyed: u64,
    scheduled: u64,
    next_admission: u64,
    next_lease: u64,
}
const OPERATION_CAP: u64 = u32::MAX as u64;
impl Default for FlowRuntime {
    fn default() -> Self {
        Self::new()
    }
}
impl FlowRuntime {
    pub fn new() -> Self {
        Self {
            scheduler: Scheduler::new(),
            world: World::new(),
            registry: ComponentRegistry::new(),
            resources: BTreeSet::new(),
            requests: BTreeSet::new(),
            actors: BTreeSet::new(),
            works: BTreeMap::new(),
            context_types: BTreeMap::new(),
            commands: BTreeMap::new(),
            pending_releases: BTreeSet::new(),
            created: 0,
            destroyed: 0,
            scheduled: 0,
            next_admission: 0,
            next_lease: 0,
        }
    }
    pub fn now(&self) -> SimTime {
        self.scheduler.now()
    }
    fn spawn(&mut self) -> Result<EntityId, FlowError> {
        if self.created >= OPERATION_CAP {
            return Err(FlowError::CounterOverflow);
        }
        let id = self.world.spawn();
        self.created += 1;
        Ok(id)
    }
    pub fn spawn_actor(&mut self) -> Result<EntityId, FlowError> {
        let id = self.spawn()?;
        self.actors.insert(id);
        Ok(id)
    }
    pub fn create_resource(&mut self, total: u32) -> Result<ResourceId, FlowError> {
        let id = ResourceId(self.spawn()?);
        let _ = self.registry.insert(id.0, ResourceCapacity { total });
        let _ = self
            .registry
            .insert(id.0, ClaimQueue::<PriorityKey>::default());
        let _ = self.registry.insert(id.0, ActiveAllocations::default());
        self.resources.insert(id);
        Ok(id)
    }
    fn actor(&self, id: EntityId) -> Result<(), FlowError> {
        if self.actors.contains(&id) && self.world.is_alive(id) {
            Ok(())
        } else {
            Err(FlowError::InvalidEntity)
        }
    }
    fn check_schedule(&self, at: SimTime) -> Result<(), FlowError> {
        if at < self.now() {
            return Err(FlowError::PastCommand);
        }
        if self.scheduled >= OPERATION_CAP {
            return Err(FlowError::CounterOverflow);
        }
        Ok(())
    }
    fn schedule(&mut self, command: Command, at: SimTime) -> Result<(), FlowError> {
        self.check_schedule(at)?;
        let event = self.scheduler.schedule(ScheduleRequest {
            at,
            priority: 0,
            entity: None,
            kind: EventKind::custom(4000),
        });
        self.scheduled += 1;
        self.commands.insert(event, command);
        Ok(())
    }
    pub fn acquire(&mut self, resource: ResourceId) -> AcquireBuilder<'_> {
        let at = self.now();
        AcquireBuilder {
            runtime: self,
            resource,
            owner: None,
            work: None,
            at,
        }
    }
    pub fn create_work<C: 'static>(
        &mut self,
        owner: EntityId,
        duration: SimDuration,
        registration: &str,
        context: C,
    ) -> Result<WorkId, FlowError> {
        self.actor(owner)?;
        if registration.trim().is_empty()
            || self
                .context_types
                .get(registration)
                .is_some_and(|kind| *kind != TypeId::of::<C>())
        {
            return Err(FlowError::InvalidWork);
        }
        let id = WorkId(self.spawn()?);
        let _ = self.registry.insert(
            id.0,
            WorkSpec {
                owner,
                original_duration: duration,
                context_type_key: registration.to_owned(),
                request: None,
            },
        );
        let _ = self.registry.insert(id.0, WorkContext(context));
        self.context_types
            .insert(registration.to_owned(), TypeId::of::<C>());
        self.works.insert(id, cleanup_context::<C>);
        Ok(id)
    }
    pub fn work(&self, id: WorkId) -> Result<WorkSpec, FlowError> {
        if !self.works.contains_key(&id) || !self.world.is_alive(id.0) {
            return Err(FlowError::InvalidWork);
        }
        self.registry
            .get::<WorkSpec>(id.0)
            .cloned()
            .ok_or(FlowError::InvalidWork)
    }
    pub fn work_context<C: 'static>(&self, id: WorkId) -> Result<&C, FlowError> {
        self.work(id)?;
        self.registry
            .get::<WorkContext<C>>(id.0)
            .map(|c| &c.0)
            .ok_or(FlowError::InvalidWork)
    }
    pub fn submit(
        &mut self,
        resource: ResourceId,
        owner: EntityId,
        at: SimTime,
    ) -> Result<RequestId, FlowError> {
        self.submit_inner(resource, owner, None, at)
    }
    pub fn submit_work(
        &mut self,
        resource: ResourceId,
        owner: EntityId,
        work: WorkId,
        at: SimTime,
    ) -> Result<RequestId, FlowError> {
        self.submit_inner(resource, owner, Some(work), at)
    }
    fn submit_inner(
        &mut self,
        resource: ResourceId,
        owner: EntityId,
        work: Option<WorkId>,
        at: SimTime,
    ) -> Result<RequestId, FlowError> {
        self.actor(owner)?;
        self.resource(resource)?;
        self.check_schedule(at)?;
        let mut spec = match work {
            Some(id) => {
                let spec = self.work(id)?;
                if spec.owner != owner || spec.request.is_some() {
                    return Err(FlowError::InvalidWork);
                };
                Some(spec)
            }
            None => None,
        };
        let request = RequestId(self.spawn()?);
        let _ = self.registry.insert(
            request.0,
            ResourceRequest {
                resource,
                owner,
                state: RequestState::Pending,
                admission_sequence: None,
                lease: None,
                priority_level: 0,
                work,
            },
        );
        self.requests.insert(request);
        self.schedule(Command::Submit(request), at)?;
        if let (Some(id), Some(spec)) = (work, spec.as_mut()) {
            spec.request = Some(request);
            let _ = self.registry.insert(id.0, spec.clone());
        }
        Ok(request)
    }
    pub fn request(&self, id: RequestId) -> Result<ResourceRequest, FlowError> {
        if !self.requests.contains(&id) || !self.world.is_alive(id.0) {
            return Err(FlowError::InvalidRequest);
        }
        self.registry
            .get::<ResourceRequest>(id.0)
            .cloned()
            .ok_or(FlowError::InvalidRequest)
    }
    pub fn resource(&self, id: ResourceId) -> Result<ResourceSnapshot, FlowError> {
        if !self.resources.contains(&id) || !self.world.is_alive(id.0) {
            return Err(FlowError::InvalidResource);
        }
        let capacity = self
            .registry
            .get::<ResourceCapacity>(id.0)
            .ok_or(FlowError::InvalidResource)?;
        let queue = self
            .registry
            .get::<ClaimQueue>(id.0)
            .ok_or(FlowError::InvalidResource)?;
        let active = self
            .registry
            .get::<ActiveAllocations>(id.0)
            .ok_or(FlowError::InvalidResource)?;
        let used = u32::try_from(active.leases.len()).map_err(|_| FlowError::InvalidState)?;
        Ok(ResourceSnapshot {
            total: capacity.total,
            available: capacity
                .total
                .checked_sub(used)
                .ok_or(FlowError::InvalidState)?,
            queued: queue.requests.iter().map(|key| key.request).collect(),
            active: active.leases.keys().copied().collect(),
            allocations: active.leases.values().cloned().collect(),
        })
    }
    pub fn release(&mut self, lease: LeaseId, at: SimTime) -> Result<(), FlowError> {
        self.check_schedule(at)?;
        let request = self.request(lease.request)?;
        if request.state != RequestState::Active
            || request.lease != Some(lease)
            || self.pending_releases.contains(&lease)
        {
            return Err(FlowError::InvalidLease);
        }
        self.schedule(Command::Release(lease), at)?;
        self.pending_releases.insert(lease);
        Ok(())
    }
    pub fn set_capacity(&mut self, id: ResourceId, total: u32) -> Result<(), FlowError> {
        if self.resource(id)?.active.len() > total as usize {
            return Err(FlowError::CapacityInUse);
        }
        self.schedule(Command::Capacity(id, total), self.now())
    }
    fn in_use(&self, id: ResourceId) -> bool {
        self.requests.iter().any(|q| {
            self.registry.get::<ResourceRequest>(q.0).is_some_and(|r| {
                r.resource == id
                    && matches!(
                        r.state,
                        RequestState::Pending | RequestState::Queued | RequestState::Active
                    )
            })
        })
    }
    pub fn remove_resource(&mut self, id: ResourceId) -> Result<(), FlowError> {
        self.resource(id)?;
        if self.in_use(id) {
            return Err(FlowError::ResourceInUse);
        }
        self.schedule(Command::Remove(id), self.now())
    }
    pub fn despawn_actor(&mut self, id: EntityId) -> Result<(), FlowError> {
        self.actor(id)?;
        self.schedule(Command::Despawn(id), self.now())
    }
    pub fn run_for(&mut self, max_events: u64) -> Result<FlowRun, FlowError> {
        let mut dispatches = Vec::new();
        for _ in 0..max_events {
            match self.step()? {
                Some(outcome) => dispatches.push(outcome),
                None => break,
            }
        }
        Ok(FlowRun {
            dispatches,
            budget_exhausted: self.scheduler.pending_events() > 0,
        })
    }
    pub fn step(&mut self) -> Result<Option<FlowDispatch>, FlowError> {
        let event = match self.scheduler.step() {
            StepOutcome::Empty => return Ok(None),
            StepOutcome::LimitReached => return Err(FlowError::InvalidState),
            StepOutcome::Dispatched(event) => event,
        };
        let command = self
            .commands
            .remove(&event.id)
            .ok_or(FlowError::InvalidState)?;
        // Reservations are admission metadata, not authoritative allocation state.
        if let Command::Release(lease) = command {
            self.pending_releases.remove(&lease);
        }
        let mut outcome = FlowDispatch {
            event: event.id,
            at: event.at,
            records: Vec::new(),
            error: None,
        };
        if let Err(error) = self.dispatch(command, &mut outcome) {
            outcome.error = Some(error);
            outcome.records.clear();
        }
        Ok(Some(outcome))
    }
    fn dispatch(&mut self, command: Command, outcome: &mut FlowDispatch) -> Result<(), FlowError> {
        // Stage all affected ECS values. No writes until every derived transition
        // and counter/despawn reservation succeeds. Q5 measures this baseline cost.
        let mut resources: BTreeMap<ResourceId, ResourceStage> = self
            .resources
            .iter()
            .map(|id| {
                (
                    *id,
                    ResourceStage {
                        capacity: self.registry.get::<ResourceCapacity>(id.0).unwrap().clone(),
                        queue: self.registry.get::<ClaimQueue>(id.0).unwrap().clone(),
                        active: self
                            .registry
                            .get::<ActiveAllocations>(id.0)
                            .unwrap()
                            .clone(),
                    },
                )
            })
            .collect();
        let mut requests: BTreeMap<RequestId, ResourceRequest> = self
            .requests
            .iter()
            .map(|id| {
                (
                    *id,
                    self.registry.get::<ResourceRequest>(id.0).unwrap().clone(),
                )
            })
            .collect();
        let mut affected = BTreeSet::new();
        let mut remove_resource = None;
        let mut remove_actor = None;
        let mut admission = self.next_admission;
        let mut lease_revision = self.next_lease;
        match command {
            Command::Submit(id) => {
                let request = requests.get_mut(&id).ok_or(FlowError::InvalidRequest)?;
                if request.state != RequestState::Pending {
                    return Err(FlowError::TerminalRequest);
                }
                self.actor(request.owner)?;
                let resource = resources
                    .get_mut(&request.resource)
                    .ok_or(FlowError::InvalidResource)?;
                let next = admission.checked_add(1).ok_or(FlowError::CounterOverflow)?;
                request.admission_sequence = Some(admission);
                admission = next;
                request.state = RequestState::Queued;
                resource.queue.requests.insert(PriorityKey {
                    level: request.priority_level,
                    enqueue_sequence: request.admission_sequence.unwrap(),
                    request: id,
                });
                affected.insert(request.resource);
                record(outcome, id, request)?;
            }
            Command::Release(lease) => {
                let request = requests
                    .get_mut(&lease.request)
                    .ok_or(FlowError::InvalidLease)?;
                if request.state != RequestState::Active || request.lease != Some(lease) {
                    return Err(FlowError::InvalidLease);
                }
                let resource = resources
                    .get_mut(&request.resource)
                    .ok_or(FlowError::InvalidResource)?;
                if resource.active.leases.remove(&lease).is_none() {
                    return Err(FlowError::InvalidState);
                }
                request.state = RequestState::Released;
                request.lease = None;
                affected.insert(request.resource);
                record(outcome, lease.request, request)?;
            }
            Command::Capacity(id, total) => {
                let resource = resources.get_mut(&id).ok_or(FlowError::InvalidResource)?;
                if resource.active.leases.len() > total as usize {
                    return Err(FlowError::CapacityInUse);
                }
                resource.capacity.total = total;
                affected.insert(id);
            }
            Command::Remove(id) => {
                if !resources.contains_key(&id) {
                    return Err(FlowError::InvalidResource);
                }
                if requests.values().any(|r| {
                    r.resource == id
                        && matches!(
                            r.state,
                            RequestState::Pending | RequestState::Queued | RequestState::Active
                        )
                }) {
                    return Err(FlowError::ResourceInUse);
                }
                resources.remove(&id);
                remove_resource = Some(id);
            }
            Command::Despawn(owner) => {
                self.actor(owner)?;
                for (id, request) in &mut requests {
                    if request.owner != owner
                        || matches!(
                            request.state,
                            RequestState::Released | RequestState::Cancelled
                        )
                    {
                        continue;
                    }
                    let resource = resources
                        .get_mut(&request.resource)
                        .ok_or(FlowError::InvalidResource)?;
                    resource.queue.requests.retain(|q| q.request != *id);
                    if let Some(lease) = request.lease {
                        resource.active.leases.remove(&lease);
                    }
                    request.state = RequestState::Cancelled;
                    request.lease = None;
                    affected.insert(request.resource);
                    record(outcome, *id, request)?;
                }
                remove_actor = Some(owner);
            }
        }
        for id in affected {
            let resource = resources.get_mut(&id).ok_or(FlowError::InvalidState)?;
            while resource.active.leases.len() < (resource.capacity.total as usize) {
                let Some(key) = resource.queue.requests.pop_first() else {
                    break;
                };
                let request_id = key.request;
                let request = requests
                    .get_mut(&request_id)
                    .ok_or(FlowError::InvalidState)?;
                if request.state != RequestState::Queued {
                    return Err(FlowError::InvalidState);
                }
                self.actor(request.owner)?;
                let next = lease_revision
                    .checked_add(1)
                    .ok_or(FlowError::CounterOverflow)?;
                let lease = LeaseId {
                    request: request_id,
                    revision: lease_revision,
                };
                lease_revision = next;
                resource.active.leases.insert(
                    lease,
                    Allocation {
                        lease,
                        request: request_id,
                        owner: request.owner,
                        work: request.work,
                        priority_level: request.priority_level,
                        granted_at: outcome.at,
                        segment_started_at: outcome.at,
                        completion_at: None,
                    },
                );
                request.lease = Some(lease);
                request.state = RequestState::Active;
                record(outcome, request_id, request)?;
            }
        }
        let removed_works: Vec<_> = self
            .works
            .iter()
            .filter(|(id, _)| {
                remove_actor.is_some_and(|owner| {
                    self.registry
                        .get::<WorkSpec>(id.0)
                        .is_some_and(|spec| spec.owner == owner)
                })
            })
            .map(|(id, cleanup)| (*id, *cleanup))
            .collect();
        let work_despawns =
            u64::try_from(removed_works.len()).map_err(|_| FlowError::CounterOverflow)?;
        let despawns = u64::from(remove_resource.is_some())
            + u64::from(remove_actor.is_some())
            + work_despawns;
        let destroyed = self
            .destroyed
            .checked_add(despawns)
            .filter(|n| *n <= OPERATION_CAP)
            .ok_or(FlowError::CounterOverflow)?;
        // Commit after the complete plan validates.
        for (id, resource) in resources {
            let _ = self.registry.insert(id.0, resource.capacity);
            let _ = self.registry.insert(id.0, resource.queue);
            let _ = self.registry.insert(id.0, resource.active);
        }
        for (id, request) in requests {
            let _ = self.registry.insert(id.0, request);
        }
        if let Some(id) = remove_resource {
            self.registry.remove::<ResourceCapacity>(id.0);
            self.registry.remove::<ClaimQueue>(id.0);
            self.registry.remove::<ActiveAllocations>(id.0);
            self.resources.remove(&id);
            self.world.despawn(id.0);
        }
        for (work, cleanup) in removed_works {
            self.works.remove(&work);
            cleanup(&mut self.registry, work.0);
            self.registry.remove::<WorkSpec>(work.0);
            self.world.despawn(work.0);
        }
        if let Some(id) = remove_actor {
            self.actors.remove(&id);
            self.world.despawn(id);
            self.pending_releases.retain(|lease| {
                self.registry
                    .get::<ResourceRequest>(lease.request.0)
                    .is_some_and(|r| r.lease == Some(*lease))
            });
        }
        self.destroyed = destroyed;
        self.next_admission = admission;
        self.next_lease = lease_revision;
        Ok(())
    }
}
fn checked_ordinal(length: usize) -> Result<u32, FlowError> {
    u32::try_from(length).map_err(|_| FlowError::CounterOverflow)
}

fn record(
    outcome: &mut FlowDispatch,
    id: RequestId,
    request: &ResourceRequest,
) -> Result<(), FlowError> {
    let ordinal = checked_ordinal(outcome.records.len())?;
    outcome.records.push(LifecycleRecord {
        request: id,
        resource: request.resource,
        at: outcome.at,
        state: request.state,
        lease: request.lease,
        causal_event_id: outcome.event,
        transition_ordinal: ordinal,
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cleanup_preflights_all_owned_work_before_mutation() {
        let mut f = FlowRuntime::new();
        let a = f.spawn_actor().unwrap();
        let r = f.create_resource(1).unwrap();
        let w = f
            .create_work(a, SimDuration::from_ticks(1), "test.v1", 42u32)
            .unwrap();
        let q = f.submit_work(r, a, w, t()).unwrap();
        f.step().unwrap();
        let before = f.resource(r).unwrap();
        f.destroyed = OPERATION_CAP - 1;
        f.despawn_actor(a).unwrap();
        assert_eq!(
            f.step().unwrap().unwrap().error,
            Some(FlowError::CounterOverflow)
        );
        assert_eq!(f.resource(r).unwrap(), before);
        assert_eq!(f.work_context::<u32>(w), Ok(&42));
        assert!(f.actor(a).is_ok());
        assert_eq!(f.request(q).unwrap().state, RequestState::Active);
    }
    #[test]
    fn work_admission_counter_failure_preserves_both_associations() {
        let mut f = FlowRuntime::new();
        let a = f.spawn_actor().unwrap();
        let r = f.create_resource(1).unwrap();
        let w = f
            .create_work(a, SimDuration::from_ticks(1), "test.v1", 42u32)
            .unwrap();
        f.scheduled = OPERATION_CAP;
        let before = f.world.snapshot();
        assert_eq!(f.submit_work(r, a, w, t()), Err(FlowError::CounterOverflow));
        assert_eq!(f.world.snapshot(), before);
        assert!(f.work(w).unwrap().request.is_none());
    }
    #[test]
    fn bounded_generated_operations_preserve_capacity_and_membership() {
        for seed in 0..32u64 {
            let mut f = FlowRuntime::new();
            let actor = f.spawn_actor().unwrap();
            let r = f.create_resource(2).unwrap();
            let mut random = seed + 1;
            for _ in 0..100 {
                random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
                match random % 3 {
                    0 => {
                        f.submit(r, actor, t()).unwrap();
                    }
                    1 => {
                        if let Some(lease) = f.resource(r).unwrap().active.first().copied() {
                            f.release(lease, t()).unwrap();
                        }
                    }
                    _ => {
                        let _ = f.set_capacity(r, ((random >> 32) % 4) as u32);
                    }
                }
                f.run_for(1).unwrap();
                let snapshot = f.resource(r).unwrap();
                assert_eq!(
                    snapshot.available as usize + snapshot.active.len(),
                    snapshot.total as usize
                );
                for key in &f.registry.get::<ClaimQueue>(r.0).unwrap().requests {
                    let request = f.request(key.request).unwrap();
                    assert_eq!(request.state, RequestState::Queued);
                    assert!(request.lease.is_none());
                    assert_eq!(key.level, request.priority_level);
                    assert_eq!(Some(key.enqueue_sequence), request.admission_sequence);
                }
                for lease in &snapshot.active {
                    let request = f.request(lease.request).unwrap();
                    assert_eq!(request.state, RequestState::Active);
                    assert_eq!(request.lease, Some(*lease));
                    assert!(!snapshot.queued.contains(&lease.request));
                }
            }
        }
    }
    #[test]
    fn transition_ordinal_is_checked_uint32() {
        assert_eq!(checked_ordinal(u32::MAX as usize), Ok(u32::MAX));
        if usize::BITS > 32 {
            assert_eq!(
                checked_ordinal(u32::MAX as usize + 1),
                Err(FlowError::CounterOverflow)
            );
        }
    }
    fn t() -> SimTime {
        SimTime::from_ticks(0)
    }
    #[test]
    fn counter_admission_rejection_has_no_partial_spawn() {
        let mut f = FlowRuntime::new();
        let a = f.spawn_actor().unwrap();
        let r = f.create_resource(1).unwrap();
        f.scheduled = OPERATION_CAP;
        let before = f.world.snapshot();
        assert_eq!(f.submit(r, a, t()), Err(FlowError::CounterOverflow));
        assert_eq!(f.world.snapshot(), before);
        f.scheduled = 0;
        f.created = OPERATION_CAP;
        assert_eq!(f.submit(r, a, t()), Err(FlowError::CounterOverflow));
        assert_eq!(f.world.snapshot(), before);
    }
    #[test]
    fn grant_overflow_rejects_the_entire_capacity_command() {
        let mut f = FlowRuntime::new();
        let a = f.spawn_actor().unwrap();
        let r = f.create_resource(0).unwrap();
        let q = f.submit(r, a, t()).unwrap();
        f.step().unwrap();
        let before = f.resource(r).unwrap();
        f.next_lease = u64::MAX;
        f.set_capacity(r, 1).unwrap();
        let dispatch = f.step().unwrap().unwrap();
        assert_eq!(dispatch.error, Some(FlowError::CounterOverflow));
        assert!(dispatch.records.is_empty());
        assert_eq!(f.resource(r).unwrap(), before);
        assert_eq!(f.request(q).unwrap().state, RequestState::Queued);
    }
    #[test]
    fn failed_release_preflight_preserves_lease_and_clears_reservation() {
        let mut f = FlowRuntime::new();
        let a = f.spawn_actor().unwrap();
        let r = f.create_resource(1).unwrap();
        let q = f.submit(r, a, t()).unwrap();
        f.step().unwrap();
        f.submit(r, a, t()).unwrap();
        f.step().unwrap();
        let lease = f.request(q).unwrap().lease.unwrap();
        let before = f.resource(r).unwrap();
        f.next_lease = u64::MAX;
        f.release(lease, t()).unwrap();
        assert_eq!(
            f.step().unwrap().unwrap().error,
            Some(FlowError::CounterOverflow)
        );
        assert_eq!(f.resource(r).unwrap(), before);
        f.next_lease = 10;
        assert!(f.release(lease, t()).is_ok());
        f.step().unwrap();
        assert_eq!(f.request(q).unwrap().state, RequestState::Released);
    }
    #[test]
    fn admission_counter_overflow_preserves_pending_request() {
        let mut f = FlowRuntime::new();
        let a = f.spawn_actor().unwrap();
        let r = f.create_resource(1).unwrap();
        let q = f.submit(r, a, t()).unwrap();
        f.next_admission = u64::MAX;
        assert_eq!(
            f.step().unwrap().unwrap().error,
            Some(FlowError::CounterOverflow)
        );
        assert_eq!(f.request(q).unwrap().state, RequestState::Pending);
        assert_eq!(f.resource(r).unwrap().available, 1);
    }
}
