//! Experimental, single-world Flow facade. No portable checkpoint promise.
use crate::preemption::{select_replacement, HolderCandidate, WaitingCandidate};
use kairo_ecs_core::Scheduler;
use kairo_ecs_state::{ComponentRegistry, World};
use kairo_ecs_types::{
    EntityId, EventId, EventKind, ScheduleRequest, SimDuration, SimTime, StepOutcome,
};
use std::any::{Any, TypeId};
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
    TimedOut,
    Suspended,
    Completed,
    Aborted,
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
    pub submitted_at: SimTime,
    pub deadline: Option<SimTime>,
    pub timed: bool,
    pub can_preempt: bool,
    pub preemptible: Option<PreemptionStrategy>,
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
/// Checked interruption policy for explicitly timed work.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreemptionStrategy {
    Suspend,
    Abort,
    Restart,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkState {
    Pending,
    Active,
    Suspended,
    Completed,
    Aborted,
    Cancelled,
    Released,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecycleTransition {
    Queued,
    Granted,
    Released,
    Cancelled,
    TimedOut,
    Preempted,
    Resumed,
    Restarted,
    Completed,
    Aborted,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkProgress {
    pub original_duration: SimDuration,
    pub useful_elapsed: SimDuration,
    pub remaining: SimDuration,
    pub cumulative_busy: SimDuration,
    pub attempt_revision: u64,
    pub execution_revision: u64,
    pub state: WorkState,
    pub segment_started_at: Option<SimTime>,
    pub completion_at: Option<SimTime>,
    restart_pending: bool,
}
impl WorkProgress {
    fn new(duration: SimDuration) -> Self {
        Self {
            original_duration: duration,
            useful_elapsed: SimDuration::ZERO,
            remaining: duration,
            cumulative_busy: SimDuration::ZERO,
            attempt_revision: 0,
            execution_revision: 0,
            state: WorkState::Pending,
            segment_started_at: None,
            completion_at: None,
            restart_pending: false,
        }
    }
    fn inspected(&self, now: SimTime) -> Result<Self, FlowError> {
        let mut p = self.clone();
        if let Some(start) = p.segment_started_at {
            let end = p.completion_at.map_or(now, |due| now.min(due));
            let elapsed = end
                .ticks()
                .checked_sub(start.ticks())
                .ok_or(FlowError::InvalidState)?;
            p.useful_elapsed = SimDuration::from_ticks(
                p.useful_elapsed
                    .ticks()
                    .checked_add(elapsed)
                    .ok_or(FlowError::CounterOverflow)?,
            );
            p.cumulative_busy = SimDuration::from_ticks(
                p.cumulative_busy
                    .ticks()
                    .checked_add(elapsed)
                    .ok_or(FlowError::CounterOverflow)?,
            );
            p.remaining = SimDuration::from_ticks(
                p.remaining
                    .ticks()
                    .checked_sub(elapsed)
                    .ok_or(FlowError::InvalidState)?,
            );
        }
        Ok(p)
    }
    fn checkpoint(&mut self, now: SimTime) -> Result<(), FlowError> {
        *self = self.inspected(now)?;
        self.segment_started_at = None;
        self.completion_at = None;
        Ok(())
    }
}
pub struct WorkHandlers<C> {
    pub on_resume: Option<fn(&mut C, &WorkProgress)>,
    pub on_restart: Option<fn(&mut C, &WorkProgress)>,
    pub on_abort: Option<fn(&mut C, &WorkProgress)>,
    pub on_cancel: Option<fn(&mut C, &WorkProgress)>,
}
impl<C> Default for WorkHandlers<C> {
    fn default() -> Self {
        Self {
            on_resume: None,
            on_restart: None,
            on_abort: None,
            on_cancel: None,
        }
    }
}
impl<C> WorkHandlers<C> {
    fn select(&self, transition: LifecycleTransition) -> Option<fn(&mut C, &WorkProgress)> {
        match transition {
            LifecycleTransition::Resumed => self.on_resume,
            LifecycleTransition::Restarted => self.on_restart,
            LifecycleTransition::Aborted => self.on_abort,
            LifecycleTransition::Cancelled => self.on_cancel,
            _ => None,
        }
    }
}
type HandlerBridge =
    fn(&mut ComponentRegistry, EntityId, &WorkProgress, LifecycleTransition, &dyn Any);
struct HandlerDescriptor {
    context_type: TypeId,
    present: fn(&dyn Any, LifecycleTransition) -> bool,
    invoke: HandlerBridge,
    handlers: Box<dyn Any>,
}
fn handler_present<C: 'static>(h: &dyn Any, transition: LifecycleTransition) -> bool {
    h.downcast_ref::<WorkHandlers<C>>()
        .expect("validated handler type")
        .select(transition)
        .is_some()
}
fn invoke_handler<C: 'static>(
    registry: &mut ComponentRegistry,
    id: EntityId,
    progress: &WorkProgress,
    transition: LifecycleTransition,
    h: &dyn Any,
) {
    if let Some(callback) = h
        .downcast_ref::<WorkHandlers<C>>()
        .expect("validated handler type")
        .select(transition)
    {
        if let Some(context) = registry
            .store_mut::<WorkContext<C>>()
            .and_then(|s| s.get_mut(id))
        {
            callback(&mut context.0, progress);
        }
    }
}
trait PreparedContext {
    fn install(self: Box<Self>, registry: &mut ComponentRegistry, id: EntityId);
}
struct Prepared<C>(C);
impl<C: 'static> PreparedContext for Prepared<C> {
    fn install(self: Box<Self>, registry: &mut ComponentRegistry, id: EntityId) {
        let _ = registry.insert(id, WorkContext(self.0));
    }
}
struct RestartTemplate<T, C> {
    template: T,
    factory: fn(&T) -> C,
}
fn prepare_restart<T: 'static, C: 'static>(
    registry: &ComponentRegistry,
    id: EntityId,
) -> Box<dyn PreparedContext> {
    let template = registry
        .get::<RestartTemplate<T, C>>(id)
        .expect("validated restart template");
    Box::new(Prepared((template.factory)(&template.template)))
}
fn cleanup_restart<T: 'static, C: 'static>(registry: &mut ComponentRegistry, id: EntityId) {
    registry.remove::<RestartTemplate<T, C>>(id);
    cleanup_context::<C>(registry, id);
}
#[derive(Clone, Copy)]
struct WorkDescriptor {
    cleanup: ContextCleanup,
    prepare: Option<fn(&ComponentRegistry, EntityId) -> Box<dyn PreparedContext>>,
}
#[derive(Clone, Debug)]
struct Notification {
    work: WorkId,
    transition: LifecycleTransition,
    progress: WorkProgress,
    origin: EventId,
    ordinal: u32,
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
    pub transition: LifecycleTransition,
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
    Deadline(RequestId),
    Cancel(RequestId),
    Reprioritize(RequestId, i32),
    Completion(RequestId, LeaseId, u64, SimTime),
    Notify,
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
    priority: i32,
    deadline: Option<SimTime>,
    scheduler_priority: i32,
    timed: bool,
    can_preempt: bool,
    preemptible: Option<PreemptionStrategy>,
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
    pub fn timed_work(mut self, work: WorkId) -> Self {
        self.work = Some(work);
        self.timed = true;
        self
    }
    pub fn can_preempt(mut self, value: bool) -> Self {
        self.can_preempt = value;
        self
    }
    pub fn preemptible(mut self, strategy: PreemptionStrategy) -> Self {
        self.preemptible = Some(strategy);
        self
    }
    pub fn priority(mut self, level: i32) -> Self {
        self.priority = level;
        self
    }
    pub fn deadline(mut self, at: SimTime) -> Self {
        self.deadline = Some(at);
        self
    }
    pub fn scheduler_priority(mut self, priority: i32) -> Self {
        self.scheduler_priority = priority;
        self
    }
    pub fn submit(self) -> Result<RequestId, FlowError> {
        let owner = self.owner.ok_or(FlowError::InvalidState)?;
        self.runtime.submit_configured(
            self.resource,
            owner,
            self.work,
            self.at,
            self.priority,
            self.deadline,
            self.scheduler_priority,
            self.timed,
            self.can_preempt,
            self.preemptible,
        )
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
    works: BTreeMap<WorkId, WorkDescriptor>,
    context_types: BTreeMap<String, TypeId>,
    handlers: BTreeMap<String, HandlerDescriptor>,
    notifications: BTreeMap<EventId, Notification>,
    commands: BTreeMap<EventId, Command>,
    pending_releases: BTreeSet<LeaseId>,
    created: u64,
    destroyed: u64,
    scheduled: u64,
    next_admission: u64,
    next_lease: u64,
}
const FLOW_COMMAND_DISPATCH_EVENT_KIND: u32 = 4000;
const FLOW_WAITING_DEADLINE_EVENT_KIND: u32 = 4002;
const FLOW_TIMED_COMPLETION_EVENT_KIND: u32 = 4001;
const FLOW_WORK_NOTIFICATION_EVENT_KIND: u32 = 4003;
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
            handlers: BTreeMap::new(),
            notifications: BTreeMap::new(),
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
        self.schedule_priority(command, at, 0)
    }
    fn schedule_priority(
        &mut self,
        command: Command,
        at: SimTime,
        priority: i32,
    ) -> Result<(), FlowError> {
        self.check_schedule(at)?;
        let event = self.scheduler.schedule(ScheduleRequest {
            at,
            priority,
            entity: None,
            kind: EventKind::custom(match command {
                Command::Deadline(_) => FLOW_WAITING_DEADLINE_EVENT_KIND,
                Command::Completion(..) => FLOW_TIMED_COMPLETION_EVENT_KIND,
                Command::Notify => FLOW_WORK_NOTIFICATION_EVENT_KIND,
                _ => FLOW_COMMAND_DISPATCH_EVENT_KIND,
            }),
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
            priority: 0,
            deadline: None,
            scheduler_priority: 0,
            timed: false,
            can_preempt: false,
            preemptible: None,
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
        self.validate_context::<C>(registration)?;
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
        let _ = self.registry.insert(id.0, WorkProgress::new(duration));
        self.works.insert(
            id,
            WorkDescriptor {
                cleanup: cleanup_context::<C>,
                prepare: None,
            },
        );
        Ok(id)
    }
    pub fn register_work_handlers<C: 'static>(
        &mut self,
        registration: &str,
        handlers: WorkHandlers<C>,
    ) -> Result<(), FlowError> {
        if registration.trim().is_empty()
            || self.context_types.contains_key(registration)
            || self.handlers.contains_key(registration)
        {
            return Err(FlowError::InvalidWork);
        }
        self.handlers.insert(
            registration.to_owned(),
            HandlerDescriptor {
                context_type: TypeId::of::<C>(),
                present: handler_present::<C>,
                invoke: invoke_handler::<C>,
                handlers: Box::new(handlers),
            },
        );
        Ok(())
    }
    pub fn create_restartable_work<T: 'static, C: 'static>(
        &mut self,
        owner: EntityId,
        duration: SimDuration,
        registration: &str,
        initial_template: T,
        make_context: fn(&T) -> C,
    ) -> Result<WorkId, FlowError> {
        self.actor(owner)?;
        self.validate_context::<C>(registration)?;
        if self.created >= OPERATION_CAP {
            return Err(FlowError::CounterOverflow);
        }
        let context = make_context(&initial_template);
        let work = self.create_work(owner, duration, registration, context)?;
        let _ = self.registry.insert(
            work.0,
            RestartTemplate {
                template: initial_template,
                factory: make_context,
            },
        );
        self.works.insert(
            work,
            WorkDescriptor {
                cleanup: cleanup_restart::<T, C>,
                prepare: Some(prepare_restart::<T, C>),
            },
        );
        Ok(work)
    }
    fn validate_context<C: 'static>(&self, registration: &str) -> Result<(), FlowError> {
        if registration.trim().is_empty()
            || self
                .context_types
                .get(registration)
                .is_some_and(|t| *t != TypeId::of::<C>())
            || self
                .handlers
                .get(registration)
                .is_some_and(|h| h.context_type != TypeId::of::<C>())
        {
            return Err(FlowError::InvalidWork);
        }
        Ok(())
    }
    pub fn work_progress(&self, id: WorkId) -> Result<WorkProgress, FlowError> {
        self.work(id)?;
        self.registry
            .get::<WorkProgress>(id.0)
            .ok_or(FlowError::InvalidWork)?
            .inspected(self.now())
    }
    fn notification(
        &self,
        work: WorkId,
        transition: LifecycleTransition,
        progress: &WorkProgress,
        outcome: &FlowDispatch,
    ) -> Option<Notification> {
        let spec = self.registry.get::<WorkSpec>(work.0)?;
        let h = self.handlers.get(&spec.context_type_key)?;
        (h.present)(h.handlers.as_ref(), transition).then(|| Notification {
            work,
            transition,
            progress: progress.clone(),
            origin: outcome.event,
            ordinal: outcome
                .records
                .last()
                .expect("transition recorded")
                .transition_ordinal,
        })
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
        self.submit_configured(resource, owner, work, at, 0, None, 0, false, false, None)
    }
    #[allow(clippy::too_many_arguments)]
    fn submit_configured(
        &mut self,
        resource: ResourceId,
        owner: EntityId,
        work: Option<WorkId>,
        at: SimTime,
        priority: i32,
        deadline: Option<SimTime>,
        scheduler_priority: i32,
        timed: bool,
        can_preempt: bool,
        preemptible: Option<PreemptionStrategy>,
    ) -> Result<RequestId, FlowError> {
        self.actor(owner)?;
        self.resource(resource)?;
        if (timed && work.is_none()) || (preemptible.is_some() && (!timed || work.is_none())) {
            return Err(FlowError::InvalidWork);
        }
        if preemptible == Some(PreemptionStrategy::Restart)
            && !work.is_some_and(|w| self.works.get(&w).is_some_and(|d| d.prepare.is_some()))
        {
            return Err(FlowError::InvalidWork);
        }
        self.check_schedule(at)?;
        let timeout = deadline.filter(|d| *d > at);
        let needed = 1 + u64::from(timeout.is_some());
        if self
            .scheduled
            .checked_add(needed)
            .filter(|n| *n <= OPERATION_CAP)
            .is_none()
        {
            return Err(FlowError::CounterOverflow);
        }
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
                priority_level: priority,
                work,
                submitted_at: at,
                deadline,
                timed,
                can_preempt,
                preemptible,
            },
        );
        self.requests.insert(request);
        self.schedule_priority(Command::Submit(request), at, scheduler_priority)?;
        if let Some(deadline) = timeout {
            self.schedule(Command::Deadline(request), deadline)?;
        }
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
    pub fn cancel(&mut self, id: RequestId, at: SimTime) -> Result<(), FlowError> {
        self.cancel_with_scheduler_priority(id, at, 0)
    }
    pub fn cancel_with_scheduler_priority(
        &mut self,
        id: RequestId,
        at: SimTime,
        priority: i32,
    ) -> Result<(), FlowError> {
        let r = self.request(id)?;
        if terminal(r.state) {
            return Err(FlowError::TerminalRequest);
        }
        self.schedule_priority(Command::Cancel(id), at, priority)
    }
    pub fn reprioritize(
        &mut self,
        id: RequestId,
        level: i32,
        at: SimTime,
    ) -> Result<(), FlowError> {
        self.reprioritize_with_scheduler_priority(id, level, at, 0)
    }
    pub fn reprioritize_with_scheduler_priority(
        &mut self,
        id: RequestId,
        level: i32,
        at: SimTime,
        priority: i32,
    ) -> Result<(), FlowError> {
        let r = self.request(id)?;
        if terminal(r.state) {
            return Err(FlowError::TerminalRequest);
        }
        self.schedule_priority(Command::Reprioritize(id, level), at, priority)
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
                        RequestState::Pending
                            | RequestState::Queued
                            | RequestState::Active
                            | RequestState::Suspended
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
        if let Command::Notify = command {
            if let Some(n) = self.notifications.remove(&event.id) {
                let _causal_origin = (n.origin, n.ordinal);
                if let Ok(spec) = self.work(n.work) {
                    if self.world.is_alive(spec.owner) {
                        if let Some(h) = self.handlers.get(&spec.context_type_key) {
                            (h.invoke)(
                                &mut self.registry,
                                n.work.0,
                                &n.progress,
                                n.transition,
                                h.handlers.as_ref(),
                            );
                        }
                    }
                }
            }
            return Ok(Some(outcome));
        }
        if let Err(error) = self.dispatch(command, &mut outcome) {
            outcome.error = Some(error);
            outcome.records.clear();
        }
        Ok(Some(outcome))
    }
    fn dispatch(&mut self, command: Command, outcome: &mut FlowDispatch) -> Result<(), FlowError> {
        // Internal stale completion tokens are strict no-ops. In particular,
        // they cannot lend their causal event to another allocation due now.
        // Explicit user commands still process their independent due boundaries.
        if let Command::Completion(id, lease, revision, due) = command {
            let live = self
                .registry
                .get::<ResourceRequest>(id.0)
                .is_some_and(|request| {
                    request.timed
                        && request.state == RequestState::Active
                        && request.lease == Some(lease)
                        && due == outcome.at
                        && request
                            .work
                            .and_then(|work| self.registry.get::<WorkProgress>(work.0))
                            .is_some_and(|progress| {
                                progress.execution_revision == revision
                                    && progress.completion_at == Some(due)
                            })
                });
            if !live {
                return Ok(());
            }
        }
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
        let mut progress: BTreeMap<WorkId, WorkProgress> = self
            .works
            .keys()
            .map(|w| {
                (
                    *w,
                    self.registry
                        .get::<WorkProgress>(w.0)
                        .expect("owned progress")
                        .clone(),
                )
            })
            .collect();
        let mut tokens: Vec<(Command, SimTime, Option<Notification>)> = Vec::new();
        let mut factories: Vec<WorkId> = Vec::new();
        let mut affected = BTreeSet::new();
        let mut remove_resource = None;
        let mut remove_actor = None;
        let mut admission = self.next_admission;
        let mut lease_revision = self.next_lease;
        // Expire only resources causally targeted by this event.
        let boundary_targets: BTreeSet<ResourceId> = match command {
            Command::Submit(id)
            | Command::Deadline(id)
            | Command::Cancel(id)
            | Command::Reprioritize(id, _)
            | Command::Completion(id, ..) => requests
                .get(&id)
                .map(|r| BTreeSet::from([r.resource]))
                .unwrap_or_default(),
            Command::Release(lease) => requests
                .get(&lease.request)
                .map(|r| BTreeSet::from([r.resource]))
                .unwrap_or_default(),
            Command::Capacity(id, _) | Command::Remove(id) => BTreeSet::from([id]),
            Command::Notify => BTreeSet::new(),
            Command::Despawn(owner) => requests
                .values()
                .filter(|r| r.owner == owner && !terminal(r.state))
                .map(|r| r.resource)
                .collect(),
        };
        // Deadline is a waiting boundary independent of token insertion order.
        for (resource_id, resource) in &mut resources {
            if !boundary_targets.contains(resource_id) {
                continue;
            }
            let expired: Vec<_> = resource
                .queue
                .requests
                .iter()
                .filter(|key| {
                    requests
                        .get(&key.request)
                        .is_some_and(|r| r.deadline.is_some_and(|at| at <= outcome.at))
                })
                .copied()
                .collect();
            for key in expired {
                resource.queue.requests.remove(&key);
                let r = requests
                    .get_mut(&key.request)
                    .ok_or(FlowError::InvalidState)?;
                r.state = RequestState::TimedOut;
                r.deadline = None;
                record(outcome, key.request, r)?;
                affected.insert(*resource_id);
            }
        }
        for (id, resource) in &mut resources {
            if !boundary_targets.contains(id) {
                continue;
            }
            let mut due: Vec<_> = resource
                .active
                .leases
                .values()
                .filter_map(|a| {
                    a.completion_at
                        .filter(|at| *at <= outcome.at)
                        .map(|at| (at, a.request, a.lease))
                })
                .collect();
            due.sort();
            for (_, request_id, lease) in due {
                let request = requests
                    .get_mut(&request_id)
                    .ok_or(FlowError::InvalidState)?;
                let work = request.work.ok_or(FlowError::InvalidState)?;
                complete_timed(
                    outcome,
                    resource,
                    request_id,
                    request,
                    progress.get_mut(&work).ok_or(FlowError::InvalidState)?,
                    lease,
                )?;
                affected.insert(*id);
            }
        }
        let boundary_progress = progress.clone();
        let boundary_tokens = tokens.clone();
        let boundary_factories = factories.clone();
        let boundary_resources = resources.clone();
        let boundary_requests = requests.clone();
        let boundary_affected = affected.clone();
        let boundary_records = outcome.records.len();
        let explicit = (|| -> Result<(), FlowError> {
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
                    if request.deadline.is_some_and(|at| at <= outcome.at) {
                        request.state = RequestState::TimedOut;
                        request.deadline = None;
                        record(outcome, id, request)?;
                        return Ok(());
                    }
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
                    if request.timed {
                        let p = progress
                            .get_mut(&request.work.ok_or(FlowError::InvalidState)?)
                            .ok_or(FlowError::InvalidState)?;
                        p.checkpoint(outcome.at)?;
                        p.state = WorkState::Released;
                    }
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
                                RequestState::Pending
                                    | RequestState::Queued
                                    | RequestState::Active
                                    | RequestState::Suspended
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
                        if request.owner != owner || terminal(request.state) {
                            continue;
                        }
                        let resource = resources
                            .get_mut(&request.resource)
                            .ok_or(FlowError::InvalidResource)?;
                        resource.queue.requests.retain(|q| q.request != *id);
                        if let Some(lease) = request.lease {
                            resource.active.leases.remove(&lease);
                        }
                        if let Some(work) = request.work {
                            let p = progress.get_mut(&work).ok_or(FlowError::InvalidState)?;
                            p.checkpoint(outcome.at)?;
                            p.state = WorkState::Cancelled;
                        }
                        request.state = RequestState::Cancelled;
                        request.lease = None;
                        affected.insert(request.resource);
                        record(outcome, *id, request)?;
                        if let Some(work) = request.work {
                            if let Some(n) = self.notification(
                                work,
                                LifecycleTransition::Cancelled,
                                &progress[&work],
                                outcome,
                            ) {
                                tokens.push((Command::Notify, outcome.at, Some(n)));
                            }
                        }
                    }
                    remove_actor = Some(owner);
                }
                Command::Notify => return Err(FlowError::InvalidState),
                Command::Completion(..) => {
                    // Validated before staging; due-boundary processing completed it.
                }
                Command::Deadline(id) => {
                    let _ = requests.get(&id).ok_or(FlowError::InvalidRequest)?;
                }
                Command::Cancel(id) => {
                    let request = requests.get_mut(&id).ok_or(FlowError::InvalidRequest)?;
                    if terminal(request.state) {
                        return Err(FlowError::TerminalRequest);
                    }
                    let resource = resources
                        .get_mut(&request.resource)
                        .ok_or(FlowError::InvalidResource)?;
                    resource.queue.requests.retain(|k| k.request != id);
                    if let Some(lease) = request.lease {
                        resource.active.leases.remove(&lease);
                    }
                    if let Some(work) = request.work {
                        let p = progress.get_mut(&work).ok_or(FlowError::InvalidState)?;
                        p.checkpoint(outcome.at)?;
                        p.state = WorkState::Cancelled;
                    }
                    request.state = RequestState::Cancelled;
                    request.lease = None;
                    request.deadline = None;
                    affected.insert(request.resource);
                    record(outcome, id, request)?;
                    if let Some(work) = request.work {
                        if let Some(n) = self.notification(
                            work,
                            LifecycleTransition::Cancelled,
                            &progress[&work],
                            outcome,
                        ) {
                            tokens.push((Command::Notify, outcome.at, Some(n)));
                        }
                    }
                }
                Command::Reprioritize(id, level) => {
                    let request = requests.get_mut(&id).ok_or(FlowError::InvalidRequest)?;
                    if terminal(request.state) {
                        return Err(FlowError::TerminalRequest);
                    }
                    let resource = resources
                        .get_mut(&request.resource)
                        .ok_or(FlowError::InvalidResource)?;
                    request.priority_level = level;
                    if matches!(
                        request.state,
                        RequestState::Queued | RequestState::Suspended
                    ) {
                        resource.queue.requests.retain(|k| k.request != id);
                        resource.queue.requests.insert(PriorityKey {
                            level,
                            enqueue_sequence: request
                                .admission_sequence
                                .ok_or(FlowError::InvalidState)?,
                            request: id,
                        });
                    }
                    if let Some(lease) = request.lease {
                        resource
                            .active
                            .leases
                            .get_mut(&lease)
                            .ok_or(FlowError::InvalidState)?
                            .priority_level = level;
                    }
                    affected.insert(request.resource);
                }
            };
            Ok(())
        })();
        if let Err(error) = explicit {
            // Rejected explicit operations cannot discard independent boundaries.
            // Counter/preflight failure rejects the complete transaction.
            if error == FlowError::CounterOverflow || error == FlowError::InvalidState {
                return Err(error);
            }
            progress = boundary_progress;
            tokens = boundary_tokens;
            factories = boundary_factories;
            resources = boundary_resources;
            requests = boundary_requests;
            affected = boundary_affected;
            outcome.records.truncate(boundary_records);
            outcome.error = Some(error);
            remove_resource = None;
            remove_actor = None;
            admission = self.next_admission;
        }
        for id in affected {
            let resource = resources.get_mut(&id).ok_or(FlowError::InvalidState)?;
            loop {
                let key = if resource.active.leases.len() < resource.capacity.total as usize {
                    resource.queue.requests.iter().next().copied()
                } else {
                    let waiters: Vec<_> = resource
                        .queue
                        .requests
                        .iter()
                        .map(|key| WaitingCandidate {
                            id: key.request,
                            priority_level: key.level,
                            original_admission_sequence: key.enqueue_sequence,
                            can_preempt: requests[&key.request].can_preempt,
                        })
                        .collect();
                    let holders: Vec<_> = resource
                        .active
                        .leases
                        .values()
                        .map(|a| {
                            let request = &requests[&a.request];
                            HolderCandidate {
                                id: a.request,
                                priority_level: a.priority_level,
                                original_admission_sequence: request
                                    .admission_sequence
                                    .expect("active admitted"),
                                remaining: request
                                    .work
                                    .and_then(|w| progress.get(&w))
                                    .map_or(SimDuration::ZERO, |p| p.remaining),
                                completion_at: a.completion_at,
                                timed: request.timed,
                                preemptible: request.preemptible.is_some(),
                            }
                        })
                        .collect();
                    let Some((incoming, victim)) =
                        select_replacement(outcome.at, &waiters, &holders)
                    else {
                        break;
                    };
                    let victim_request =
                        requests.get_mut(&victim).ok_or(FlowError::InvalidState)?;
                    let work = victim_request.work.ok_or(FlowError::InvalidState)?;
                    let p = progress.get_mut(&work).ok_or(FlowError::InvalidState)?;
                    p.checkpoint(outcome.at)?;
                    let old_lease = victim_request.lease.take().ok_or(FlowError::InvalidState)?;
                    resource
                        .active
                        .leases
                        .remove(&old_lease)
                        .ok_or(FlowError::InvalidState)?;
                    victim_request.state = RequestState::Suspended;
                    p.state = WorkState::Suspended;
                    record_transition(
                        outcome,
                        victim,
                        victim_request,
                        LifecycleTransition::Preempted,
                    )?;
                    match victim_request.preemptible.ok_or(FlowError::InvalidState)? {
                        PreemptionStrategy::Abort => {
                            victim_request.state = RequestState::Aborted;
                            p.state = WorkState::Aborted;
                            record(outcome, victim, victim_request)?;
                            if let Some(n) =
                                self.notification(work, LifecycleTransition::Aborted, p, outcome)
                            {
                                tokens.push((Command::Notify, outcome.at, Some(n)));
                            }
                        }
                        PreemptionStrategy::Restart => {
                            p.attempt_revision = p
                                .attempt_revision
                                .checked_add(1)
                                .ok_or(FlowError::CounterOverflow)?;
                            p.useful_elapsed = SimDuration::ZERO;
                            p.remaining = p.original_duration;
                            p.restart_pending = true;
                            resource.queue.requests.insert(PriorityKey {
                                level: victim_request.priority_level,
                                enqueue_sequence: victim_request
                                    .admission_sequence
                                    .ok_or(FlowError::InvalidState)?,
                                request: victim,
                            });
                        }
                        PreemptionStrategy::Suspend => {
                            resource.queue.requests.insert(PriorityKey {
                                level: victim_request.priority_level,
                                enqueue_sequence: victim_request
                                    .admission_sequence
                                    .ok_or(FlowError::InvalidState)?,
                                request: victim,
                            });
                        }
                    }
                    resource
                        .queue
                        .requests
                        .iter()
                        .find(|key| key.request == incoming)
                        .copied()
                };
                let Some(key) = key else { break };
                resource.queue.requests.remove(&key);
                let request_id = key.request;
                let request = requests
                    .get_mut(&request_id)
                    .ok_or(FlowError::InvalidState)?;
                if !matches!(
                    request.state,
                    RequestState::Queued | RequestState::Suspended
                ) {
                    return Err(FlowError::InvalidState);
                }
                self.actor(request.owner)?;
                let lease = LeaseId {
                    request: request_id,
                    revision: lease_revision,
                };
                lease_revision = lease_revision
                    .checked_add(1)
                    .ok_or(FlowError::CounterOverflow)?;
                let mut completion_at = None;
                let mut transition = LifecycleTransition::Granted;
                if request.timed {
                    let work = request.work.ok_or(FlowError::InvalidState)?;
                    let p = progress.get_mut(&work).ok_or(FlowError::InvalidState)?;
                    if request.state == RequestState::Suspended {
                        if p.restart_pending {
                            transition = LifecycleTransition::Restarted;
                            if self.works.get(&work).and_then(|d| d.prepare).is_none() {
                                return Err(FlowError::InvalidState);
                            }
                            factories.push(work);
                            p.restart_pending = false;
                        } else {
                            transition = LifecycleTransition::Resumed;
                        }
                    }
                    p.execution_revision = p
                        .execution_revision
                        .checked_add(1)
                        .ok_or(FlowError::CounterOverflow)?;
                    p.state = WorkState::Active;
                    p.segment_started_at = Some(outcome.at);
                    let due = outcome
                        .at
                        .checked_add(p.remaining)
                        .ok_or(FlowError::CounterOverflow)?;
                    p.completion_at = Some(due);
                    completion_at = Some(due);
                    tokens.push((
                        Command::Completion(request_id, lease, p.execution_revision, due),
                        due,
                        None,
                    ));
                }
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
                        completion_at,
                    },
                );
                request.lease = Some(lease);
                request.state = RequestState::Active;
                request.deadline = None;
                record_transition(outcome, request_id, request, transition)?;
                if request.timed {
                    let work = request.work.ok_or(FlowError::InvalidState)?;
                    if let Some(n) = self.notification(work, transition, &progress[&work], outcome)
                    {
                        tokens.push((Command::Notify, outcome.at, Some(n)));
                    }
                    if completion_at == Some(outcome.at) {
                        complete_timed(
                            outcome,
                            resource,
                            request_id,
                            request,
                            progress.get_mut(&work).ok_or(FlowError::InvalidState)?,
                            lease,
                        )?;
                    }
                }
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
        let token_count = u64::try_from(tokens.len()).map_err(|_| FlowError::CounterOverflow)?;
        let scheduled = self
            .scheduled
            .checked_add(token_count)
            .filter(|n| *n <= OPERATION_CAP)
            .ok_or(FlowError::CounterOverflow)?;
        // Factory invocations are after every arithmetic/invariant reservation.
        let prepared: Vec<_> = factories
            .into_iter()
            .map(|w| {
                let factory = self.works[&w].prepare.expect("preflighted factory");
                (w, factory(&self.registry, w.0))
            })
            .collect();
        // Commit after the complete plan validates.
        for (id, resource) in resources {
            let _ = self.registry.insert(id.0, resource.capacity);
            let _ = self.registry.insert(id.0, resource.queue);
            let _ = self.registry.insert(id.0, resource.active);
        }
        for (id, request) in requests {
            let _ = self.registry.insert(id.0, request);
        }
        for (id, p) in progress {
            let _ = self.registry.insert(id.0, p);
        }
        for (id, context) in prepared {
            context.install(&mut self.registry, id.0);
        }
        for (command, at, notification) in tokens {
            let event = self.scheduler.schedule(ScheduleRequest {
                at,
                priority: 0,
                entity: None,
                kind: EventKind::custom(if matches!(command, Command::Notify) {
                    FLOW_WORK_NOTIFICATION_EVENT_KIND
                } else {
                    FLOW_TIMED_COMPLETION_EVENT_KIND
                }),
            });
            self.commands.insert(event, command);
            if let Some(n) = notification {
                self.notifications.insert(event, n);
            }
        }
        self.scheduled = scheduled;
        if let Some(id) = remove_resource {
            self.registry.remove::<ResourceCapacity>(id.0);
            self.registry.remove::<ClaimQueue>(id.0);
            self.registry.remove::<ActiveAllocations>(id.0);
            self.resources.remove(&id);
            self.world.despawn(id.0);
        }
        for (work, cleanup) in removed_works {
            self.works.remove(&work);
            (cleanup.cleanup)(&mut self.registry, work.0);
            self.registry.remove::<WorkProgress>(work.0);
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
fn terminal(state: RequestState) -> bool {
    matches!(
        state,
        RequestState::Released
            | RequestState::Cancelled
            | RequestState::TimedOut
            | RequestState::Completed
            | RequestState::Aborted
    )
}

fn checked_ordinal(length: usize) -> Result<u32, FlowError> {
    u32::try_from(length).map_err(|_| FlowError::CounterOverflow)
}

fn record(
    outcome: &mut FlowDispatch,
    id: RequestId,
    request: &ResourceRequest,
) -> Result<(), FlowError> {
    let transition = match request.state {
        RequestState::Queued => LifecycleTransition::Queued,
        RequestState::Active => LifecycleTransition::Granted,
        RequestState::Released => LifecycleTransition::Released,
        RequestState::Cancelled => LifecycleTransition::Cancelled,
        RequestState::TimedOut => LifecycleTransition::TimedOut,
        RequestState::Completed => LifecycleTransition::Completed,
        RequestState::Aborted => LifecycleTransition::Aborted,
        RequestState::Suspended => LifecycleTransition::Preempted,
        RequestState::Pending => return Err(FlowError::InvalidState),
    };
    record_transition(outcome, id, request, transition)
}
fn record_transition(
    outcome: &mut FlowDispatch,
    id: RequestId,
    request: &ResourceRequest,
    transition: LifecycleTransition,
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
        transition,
    });
    Ok(())
}
fn complete_timed(
    outcome: &mut FlowDispatch,
    resource: &mut ResourceStage,
    id: RequestId,
    request: &mut ResourceRequest,
    progress: &mut WorkProgress,
    lease: LeaseId,
) -> Result<(), FlowError> {
    progress.checkpoint(outcome.at)?;
    if progress.remaining != SimDuration::ZERO {
        return Err(FlowError::InvalidState);
    }
    resource
        .active
        .leases
        .remove(&lease)
        .ok_or(FlowError::InvalidState)?;
    progress.state = WorkState::Completed;
    request.state = RequestState::Completed;
    request.lease = None;
    record(outcome, id, request)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stale_completion_cannot_process_same_tick_neighbor_or_mismatched_future_boundary() {
        let mut f = FlowRuntime::new();
        let owner = f.spawn_actor().unwrap();
        let r = f.create_resource(1).unwrap();
        let low = f.create_work(owner, duration(2), "stale.low", ()).unwrap();
        let low_request = f
            .acquire(r)
            .owner(owner)
            .timed_work(low)
            .priority(9)
            .preemptible(PreemptionStrategy::Suspend)
            .submit()
            .unwrap();
        let urgent = f
            .create_work(owner, duration(1), "stale.urgent", ())
            .unwrap();
        let urgent_request = f
            .acquire(r)
            .owner(owner)
            .timed_work(urgent)
            .priority(1)
            .can_preempt(true)
            .at(ticks(1))
            .submit()
            .unwrap();
        f.step().unwrap();
        let old_lease = f.request(low_request).unwrap().lease.unwrap();
        f.step().unwrap();
        let lease = f.request(urgent_request).unwrap().lease.unwrap();
        let revision = f
            .registry
            .get::<WorkProgress>(urgent.0)
            .unwrap()
            .execution_revision;
        let before = f.resource(r).unwrap();
        let p = f.registry.get::<WorkProgress>(urgent.0).unwrap().clone();
        for (command, at) in [
            (
                Command::Completion(urgent_request, lease, revision + 1, ticks(2)),
                2,
            ),
            (
                Command::Completion(urgent_request, lease, revision, ticks(3)),
                3,
            ),
            (
                Command::Completion(urgent_request, lease, revision, ticks(2)),
                10,
            ),
            (Command::Completion(low_request, old_lease, 1, ticks(2)), 2),
        ] {
            let outcome = dispatch_at(&mut f, command, at).unwrap();
            assert!(outcome.records.is_empty());
            assert!(outcome.error.is_none());
            assert_eq!(f.resource(r).unwrap(), before);
            assert_eq!(f.registry.get::<WorkProgress>(urgent.0), Some(&p));
        }
        // Original low completion was inserted before urgent's same-tick completion.
        let stale = f.step().unwrap().unwrap();
        assert_eq!(stale.at, ticks(2));
        assert!(stale.records.is_empty());
        assert!(stale.error.is_none());
        assert_eq!(
            f.request(urgent_request).unwrap().state,
            RequestState::Active
        );
        let live = f.step().unwrap().unwrap();
        assert_eq!(live.at, ticks(2));
        assert_eq!(live.records[0].transition, LifecycleTransition::Completed);
        assert_eq!(live.records[0].request, urgent_request);
        assert_eq!(
            f.request(urgent_request).unwrap().state,
            RequestState::Completed
        );
        f.run_for(32).unwrap();
        assert_eq!(
            f.request(low_request).unwrap().state,
            RequestState::Completed
        );
    }
    fn fresh_initial(initial: &u32) -> u32 {
        *initial
    }
    #[test]
    fn repeated_interruption_grid_preserves_effort_and_terminal_uniqueness() {
        for cycles in 1..=8u128 {
            for strategy in [
                PreemptionStrategy::Suspend,
                PreemptionStrategy::Restart,
                PreemptionStrategy::Abort,
            ] {
                let mut f = FlowRuntime::new();
                let owner = f.spawn_actor().unwrap();
                let r = f.create_resource(1).unwrap();
                let w = if strategy == PreemptionStrategy::Restart {
                    f.create_restartable_work(owner, duration(30), "grid.low", 17u32, fresh_initial)
                        .unwrap()
                } else {
                    f.create_work(owner, duration(30), "grid.low", 17u32)
                        .unwrap()
                };
                let q = f
                    .acquire(r)
                    .owner(owner)
                    .timed_work(w)
                    .priority(9)
                    .preemptible(strategy)
                    .submit()
                    .unwrap();
                for i in 1..=cycles {
                    let urgent = f
                        .create_work(owner, duration(1), &format!("grid.urgent.{i}"), ())
                        .unwrap();
                    f.acquire(r)
                        .owner(owner)
                        .timed_work(urgent)
                        .priority(1)
                        .can_preempt(true)
                        .at(ticks(2 * i))
                        .submit()
                        .unwrap();
                }
                let run = f.run_for(1024).unwrap();
                assert!(!run.budget_exhausted);
                assert!(run.dispatches.iter().all(|d| d.error.is_none()));
                let p = f.work_progress(w).unwrap();
                let terminal: Vec<_> = run
                    .dispatches
                    .iter()
                    .flat_map(|d| &d.records)
                    .filter(|row| {
                        row.request == q
                            && matches!(
                                row.transition,
                                LifecycleTransition::Completed | LifecycleTransition::Aborted
                            )
                    })
                    .collect();
                assert_eq!(terminal.len(), 1);
                let (at, busy, attempt, execution) = match strategy {
                    PreemptionStrategy::Suspend => (30 + cycles, 30, 0, cycles + 1),
                    PreemptionStrategy::Restart => {
                        (2 * cycles + 31, 31 + cycles, cycles, cycles + 1)
                    }
                    PreemptionStrategy::Abort => (2, 2, 0, 1),
                };
                assert_eq!(terminal[0].at, ticks(at));
                assert_eq!(p.cumulative_busy, duration(busy));
                assert_eq!(
                    (p.attempt_revision, p.execution_revision),
                    (attempt as u64, execution as u64)
                );
                assert_eq!(f.work_context::<u32>(w).unwrap(), &17);
                let snapshot = f.resource(r).unwrap();
                assert_eq!(
                    (
                        snapshot.available,
                        snapshot.active.len(),
                        snapshot.queued.len()
                    ),
                    (1, 0, 0)
                );
            }
        }
    }
    #[test]
    fn invalid_timed_capabilities_reject_before_any_request_or_work_association() {
        let mut f = FlowRuntime::new();
        let owner = f.spawn_actor().unwrap();
        let r = f.create_resource(1).unwrap();
        let w = f.create_work(owner, duration(1), "guards", ()).unwrap();
        let before = (f.created, f.scheduled, f.world.snapshot());
        assert_eq!(
            f.acquire(r)
                .owner(owner)
                .preemptible(PreemptionStrategy::Suspend)
                .submit(),
            Err(FlowError::InvalidWork)
        );
        assert_eq!(
            f.acquire(r)
                .owner(owner)
                .for_work(w)
                .preemptible(PreemptionStrategy::Abort)
                .submit(),
            Err(FlowError::InvalidWork)
        );
        assert_eq!(
            f.acquire(r)
                .owner(owner)
                .timed_work(w)
                .preemptible(PreemptionStrategy::Restart)
                .submit(),
            Err(FlowError::InvalidWork)
        );
        assert_eq!((f.created, f.scheduled, f.world.snapshot()), before);
        assert!(f.work(w).unwrap().request.is_none());
        f.register_work_handlers("typed", WorkHandlers::<u32>::default())
            .unwrap();
        assert_eq!(
            f.create_work(owner, duration(1), "typed", String::new()),
            Err(FlowError::InvalidWork)
        );
        assert_eq!(
            f.register_work_handlers("typed", WorkHandlers::<u32>::default()),
            Err(FlowError::InvalidWork)
        );
        assert_eq!((f.created, f.scheduled, f.world.snapshot()), before);
    }
    fn ticks(n: u128) -> SimTime {
        SimTime::from_ticks(n)
    }
    fn duration(n: u128) -> SimDuration {
        SimDuration::from_ticks(n)
    }
    fn dispatch_at(
        f: &mut FlowRuntime,
        command: Command,
        at: u128,
    ) -> Result<FlowDispatch, FlowError> {
        let mut outcome = FlowDispatch {
            event: EventId::new(0, 0),
            at: ticks(at),
            records: Vec::new(),
            error: None,
        };
        f.dispatch(command, &mut outcome)?;
        Ok(outcome)
    }
    fn active_timed(
        strategy: PreemptionStrategy,
    ) -> (FlowRuntime, EntityId, ResourceId, WorkId, RequestId) {
        let mut f = FlowRuntime::new();
        let owner = f.spawn_actor().unwrap();
        let resource = f.create_resource(1).unwrap();
        let work = f
            .create_work(owner, duration(10), "fault.low", 0u32)
            .unwrap();
        let request = f
            .acquire(resource)
            .owner(owner)
            .timed_work(work)
            .priority(9)
            .preemptible(strategy)
            .submit()
            .unwrap();
        f.step().unwrap().unwrap();
        (f, owner, resource, work, request)
    }
    #[test]
    fn timed_preflight_failures_leave_holder_request_progress_and_queue_unchanged() {
        for fault in 0..5 {
            let (mut f, owner, resource, work, low) = active_timed(PreemptionStrategy::Suspend);
            let urgent = f
                .create_work(
                    owner,
                    duration(if fault == 4 { u128::MAX } else { 2 }),
                    "fault.high",
                    (),
                )
                .unwrap();
            let high = f
                .acquire(resource)
                .owner(owner)
                .timed_work(urgent)
                .priority(1)
                .can_preempt(true)
                .at(ticks(1))
                .submit()
                .unwrap();
            match fault {
                0 => f.next_lease = u64::MAX,
                1 => f.scheduled = OPERATION_CAP,
                2 => {
                    f.registry
                        .store_mut::<WorkProgress>()
                        .unwrap()
                        .get_mut(urgent.0)
                        .unwrap()
                        .execution_revision = u64::MAX
                }
                3 => {
                    f.registry
                        .store_mut::<WorkProgress>()
                        .unwrap()
                        .get_mut(work.0)
                        .unwrap()
                        .cumulative_busy = duration(u128::MAX)
                }
                _ => {}
            }
            let before = f.resource(resource).unwrap();
            let progress = f.registry.get::<WorkProgress>(work.0).unwrap().clone();
            let scheduled = f.scheduled;
            let revision = f.next_lease;
            let admission = f.next_admission;
            let outcome = f.step().unwrap().unwrap();
            assert_eq!(outcome.error, Some(FlowError::CounterOverflow));
            assert!(outcome.records.is_empty());
            assert_eq!(f.resource(resource).unwrap(), before);
            assert_eq!(f.registry.get::<WorkProgress>(work.0), Some(&progress));
            assert_eq!(f.request(low).unwrap().state, RequestState::Active);
            assert_eq!(f.request(high).unwrap().state, RequestState::Pending);
            assert_eq!(
                (f.scheduled, f.next_lease, f.next_admission),
                (scheduled, revision, admission)
            );
        }
    }
    #[test]
    fn second_replacement_overflow_rolls_back_first_eviction() {
        let mut f = FlowRuntime::new();
        let owner = f.spawn_actor().unwrap();
        let resource = f.create_resource(2).unwrap();
        let mut works = Vec::new();
        for i in 0..2 {
            let w = f
                .create_work(owner, duration(10), &format!("holder.{i}"), ())
                .unwrap();
            f.acquire(resource)
                .owner(owner)
                .timed_work(w)
                .priority(9)
                .preemptible(PreemptionStrategy::Suspend)
                .submit()
                .unwrap();
            f.step().unwrap();
            works.push(w);
        }
        let mut highs = Vec::new();
        for i in 0..2 {
            let w = f
                .create_work(owner, duration(2), &format!("waiter.{i}"), ())
                .unwrap();
            let q = f
                .acquire(resource)
                .owner(owner)
                .timed_work(w)
                .priority(1)
                .at(ticks(1))
                .submit()
                .unwrap();
            f.step().unwrap();
            highs.push(q);
        }
        // A single command triggers two replacements; the second lease reservation fails.
        for q in &highs {
            f.registry
                .store_mut::<ResourceRequest>()
                .unwrap()
                .get_mut(q.0)
                .unwrap()
                .can_preempt = true;
        }
        f.next_lease = u64::MAX - 1;
        let before = f.resource(resource).unwrap();
        let progress: Vec<_> = works
            .iter()
            .map(|w| f.registry.get::<WorkProgress>(w.0).unwrap().clone())
            .collect();
        assert_eq!(
            dispatch_at(&mut f, Command::Capacity(resource, 2), 1),
            Err(FlowError::CounterOverflow)
        );
        assert_eq!(f.resource(resource).unwrap(), before);
        for (w, p) in works.iter().zip(progress) {
            assert_eq!(f.registry.get::<WorkProgress>(w.0), Some(&p));
        }
    }
    #[test]
    fn rejected_release_preserves_due_completion_and_replacement_boundary() {
        let mut f = FlowRuntime::new();
        let owner = f.spawn_actor().unwrap();
        let r = f.create_resource(1).unwrap();
        let w = f
            .create_work(owner, duration(2), "boundary.low", ())
            .unwrap();
        let q = f.acquire(r).owner(owner).timed_work(w).submit().unwrap();
        f.step().unwrap();
        let lease = f.request(q).unwrap().lease.unwrap();
        let waiting = f.submit(r, owner, ticks(0)).unwrap();
        f.step().unwrap();
        let outcome = dispatch_at(&mut f, Command::Release(lease), 2).unwrap();
        assert_eq!(outcome.error, Some(FlowError::InvalidLease));
        assert_eq!(
            outcome
                .records
                .iter()
                .map(|r| r.transition)
                .collect::<Vec<_>>(),
            vec![LifecycleTransition::Completed, LifecycleTransition::Granted]
        );
        assert_eq!(f.request(waiting).unwrap().state, RequestState::Active);
        assert_eq!(
            f.registry.get::<WorkProgress>(w.0).unwrap().cumulative_busy,
            duration(2)
        );
    }
    #[test]
    fn overflow_rejects_even_due_completion_boundary() {
        let mut f = FlowRuntime::new();
        let owner = f.spawn_actor().unwrap();
        let r = f.create_resource(1).unwrap();
        let w = f
            .create_work(owner, duration(2), "boundary.overflow", ())
            .unwrap();
        let q = f.acquire(r).owner(owner).timed_work(w).submit().unwrap();
        f.step().unwrap();
        f.submit(r, owner, ticks(0)).unwrap();
        f.step().unwrap();
        f.next_lease = u64::MAX;
        let before = f.resource(r).unwrap();
        let progress = f.registry.get::<WorkProgress>(w.0).unwrap().clone();
        assert_eq!(
            dispatch_at(&mut f, Command::Capacity(r, 1), 2),
            Err(FlowError::CounterOverflow)
        );
        assert_eq!(f.resource(r).unwrap(), before);
        assert_eq!(f.request(q).unwrap().state, RequestState::Active);
        assert_eq!(f.registry.get::<WorkProgress>(w.0), Some(&progress));
    }
    static FACTORY_CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    fn counted_factory(initial: &u32) -> u32 {
        FACTORY_CALLS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        *initial
    }
    #[test]
    fn restart_factory_waits_for_aggregate_preflight_and_attempt_overflow_is_atomic() {
        FACTORY_CALLS.store(0, std::sync::atomic::Ordering::SeqCst);
        let mut f = FlowRuntime::new();
        let owner = f.spawn_actor().unwrap();
        let r = f.create_resource(1).unwrap();
        let low = f
            .create_restartable_work(owner, duration(10), "restart.fault", 7u32, counted_factory)
            .unwrap();
        let q = f
            .acquire(r)
            .owner(owner)
            .timed_work(low)
            .priority(9)
            .preemptible(PreemptionStrategy::Restart)
            .submit()
            .unwrap();
        f.step().unwrap();
        let urgent = f
            .create_work(owner, duration(2), "restart.urgent", ())
            .unwrap();
        let high = f
            .acquire(r)
            .owner(owner)
            .timed_work(urgent)
            .priority(1)
            .can_preempt(true)
            .at(ticks(1))
            .submit()
            .unwrap();
        f.registry
            .store_mut::<WorkProgress>()
            .unwrap()
            .get_mut(low.0)
            .unwrap()
            .attempt_revision = u64::MAX;
        let before = f.resource(r).unwrap();
        assert_eq!(
            f.step().unwrap().unwrap().error,
            Some(FlowError::CounterOverflow)
        );
        assert_eq!(f.resource(r).unwrap(), before);
        assert_eq!(FACTORY_CALLS.load(std::sync::atomic::Ordering::SeqCst), 1);
        f.registry
            .store_mut::<WorkProgress>()
            .unwrap()
            .get_mut(low.0)
            .unwrap()
            .attempt_revision = 0;
        dispatch_at(&mut f, Command::Submit(high), 1).unwrap();
        assert_eq!(f.request(q).unwrap().state, RequestState::Suspended);
        let lease = f.request(high).unwrap().lease.unwrap();
        f.scheduled = OPERATION_CAP;
        let before = f.resource(r).unwrap();
        assert_eq!(
            dispatch_at(&mut f, Command::Release(lease), 2),
            Err(FlowError::CounterOverflow)
        );
        assert_eq!(f.resource(r).unwrap(), before);
        assert_eq!(FACTORY_CALLS.load(std::sync::atomic::Ordering::SeqCst), 1);
        f.scheduled = 10;
        dispatch_at(&mut f, Command::Release(lease), 2).unwrap();
        assert_eq!(FACTORY_CALLS.load(std::sync::atomic::Ordering::SeqCst), 2);
        assert_eq!(f.work_context::<u32>(low).unwrap(), &7);
        assert_eq!(
            f.registry
                .get::<WorkProgress>(low.0)
                .unwrap()
                .attempt_revision,
            1
        );
    }
    fn observe_cancel(context: &mut Vec<WorkProgress>, progress: &WorkProgress) {
        context.push(progress.clone());
    }
    #[test]
    fn notification_budget_is_atomic_and_consumption_and_removed_context_are_once_only() {
        let mut f = FlowRuntime::new();
        let owner = f.spawn_actor().unwrap();
        let r = f.create_resource(1).unwrap();
        f.register_work_handlers(
            "callback",
            WorkHandlers {
                on_cancel: Some(observe_cancel),
                ..WorkHandlers::default()
            },
        )
        .unwrap();
        let w = f
            .create_work(owner, duration(10), "callback", Vec::<WorkProgress>::new())
            .unwrap();
        let q = f.acquire(r).owner(owner).timed_work(w).submit().unwrap();
        f.step().unwrap();
        f.scheduled = OPERATION_CAP;
        let before = f.resource(r).unwrap();
        assert_eq!(
            dispatch_at(&mut f, Command::Cancel(q), 1),
            Err(FlowError::CounterOverflow)
        );
        assert_eq!(f.resource(r).unwrap(), before);
        assert!(f.notifications.is_empty());
        assert!(f.work_context::<Vec<WorkProgress>>(w).unwrap().is_empty());
        f.scheduled = 10;
        let outcome = dispatch_at(&mut f, Command::Cancel(q), 1).unwrap();
        let (token, n) = f.notifications.iter().next().unwrap();
        let token = *token;
        assert_eq!((n.origin, n.ordinal), (outcome.event, 0));
        assert_eq!(n.progress.cumulative_busy, duration(1));
        f.step().unwrap();
        assert_eq!(f.work_context::<Vec<WorkProgress>>(w).unwrap().len(), 1);
        assert!(!f.notifications.contains_key(&token));
        f.run_for(32).unwrap();
        assert_eq!(f.work_context::<Vec<WorkProgress>>(w).unwrap().len(), 1);
        // A separately committed notification loses delivery when owner cleanup removes context.
        let owner2 = f.spawn_actor().unwrap();
        let w2 = f
            .create_work(owner2, duration(1), "callback", Vec::<WorkProgress>::new())
            .unwrap();
        let q2 = f.acquire(r).owner(owner2).timed_work(w2).submit().unwrap();
        f.step().unwrap();
        let now = f.now().ticks();
        dispatch_at(&mut f, Command::Cancel(q2), now).unwrap();
        dispatch_at(&mut f, Command::Despawn(owner2), now).unwrap();
        assert_eq!(f.work(w2), Err(FlowError::InvalidWork));
        f.run_for(32).unwrap();
    }
    #[test]
    fn timed_owner_cleanup_overflow_preserves_all_live_context_and_progress() {
        let (mut f, owner, r, w, q) = active_timed(PreemptionStrategy::Suspend);
        f.destroyed = OPERATION_CAP - 1;
        let before = f.resource(r).unwrap();
        let p = f.registry.get::<WorkProgress>(w.0).unwrap().clone();
        assert_eq!(
            dispatch_at(&mut f, Command::Despawn(owner), 1),
            Err(FlowError::CounterOverflow)
        );
        assert_eq!(f.resource(r).unwrap(), before);
        assert_eq!(f.registry.get::<WorkProgress>(w.0), Some(&p));
        assert_eq!(f.work_context::<u32>(w).unwrap(), &0);
        assert_eq!(f.request(q).unwrap().state, RequestState::Active);
    }
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
            let mut sequences = BTreeMap::new();
            let mut terminals = BTreeSet::new();
            for _ in 0..100 {
                random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
                let now = f.now();
                let candidates: Vec<_> = f
                    .requests
                    .iter()
                    .copied()
                    .filter(|id| !terminal(f.request(*id).unwrap().state))
                    .collect();
                let chosen = candidates
                    .get((random >> 16) as usize % candidates.len().max(1))
                    .copied();
                match random % 6 {
                    0 => {
                        f.acquire(r)
                            .owner(actor)
                            .priority((random >> 32) as i32)
                            .deadline(SimTime::from_ticks(now.ticks() + 1))
                            .submit()
                            .unwrap();
                    }
                    1 => {
                        if let Some(lease) = f.resource(r).unwrap().active.first().copied() {
                            let _ = f.release(lease, now);
                        }
                    }
                    2 => {
                        let _ = f.set_capacity(r, ((random >> 32) % 4) as u32);
                    }
                    3 => {
                        if let Some(id) = chosen {
                            let _ = f.cancel(id, now);
                        }
                    }
                    4 => {
                        if let Some(id) = chosen {
                            let _ = f.reprioritize(id, (random >> 32) as i32, now);
                        }
                    }
                    _ => {
                        f.submit(r, actor, now).unwrap();
                    }
                }
                let run = f.run_for(1).unwrap();
                for row in run.dispatches.iter().flat_map(|d| &d.records) {
                    if terminal(row.state) {
                        assert!(terminals.insert(row.request));
                    }
                }
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
                for id in &f.requests {
                    let req = f.request(*id).unwrap();
                    if terminal(req.state) {
                        assert!(!snapshot.queued.contains(id));
                        assert!(req.lease.is_none());
                    }
                    if let Some(seq) = req.admission_sequence {
                        assert_eq!(*sequences.entry(*id).or_insert(seq), seq);
                    }
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
    fn capacity_growth_cannot_grant_at_deadline_in_either_token_order() {
        for growth_first in [false, true] {
            let mut f = FlowRuntime::new();
            let a = f.spawn_actor().unwrap();
            let r = f.create_resource(0).unwrap();
            let at = SimTime::from_ticks(5);
            if growth_first {
                f.schedule(Command::Capacity(r, 1), at).unwrap();
            }
            let q = f.acquire(r).owner(a).deadline(at).submit().unwrap();
            f.step().unwrap();
            if !growth_first {
                f.schedule(Command::Capacity(r, 1), at).unwrap();
            }
            let run = f.run_for(10).unwrap();
            assert_eq!(f.request(q).unwrap().state, RequestState::TimedOut);
            let state = f.resource(r).unwrap();
            assert_eq!(state.total, 1);
            assert_eq!(state.available, 1);
            assert_eq!(
                run.dispatches
                    .iter()
                    .flat_map(|d| &d.records)
                    .filter(|e| e.request == q && e.state == RequestState::TimedOut)
                    .count(),
                1
            );
            assert!(!run
                .dispatches
                .iter()
                .flat_map(|d| &d.records)
                .any(|e| e.request == q && e.state == RequestState::Active));
        }
    }
    #[test]
    fn deadline_admission_reserves_both_tokens_before_work_association() {
        let mut f = FlowRuntime::new();
        let a = f.spawn_actor().unwrap();
        let r = f.create_resource(1).unwrap();
        let w = f
            .create_work(a, SimDuration::from_ticks(1), "budget.v1", ())
            .unwrap();
        f.scheduled = OPERATION_CAP - 1;
        let before = f.world.snapshot();
        assert_eq!(
            f.acquire(r)
                .owner(a)
                .for_work(w)
                .deadline(SimTime::from_ticks(1))
                .submit(),
            Err(FlowError::CounterOverflow)
        );
        assert_eq!(f.world.snapshot(), before);
        assert!(f.work(w).unwrap().request.is_none());
        assert_eq!(f.scheduled, OPERATION_CAP - 1);
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
