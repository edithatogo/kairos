//! Experimental, single-world Flow facade. No portable checkpoint promise.
use crate::preemption::{select_replacement, HolderCandidate, WaitingCandidate};
use kairo_ecs_core::{ScheduledEventPreview, Scheduler, SchedulerStats};
use kairo_ecs_state::{ComponentRegistry, World};
use kairo_ecs_types::{
    EntityId, EventId, EventKind, ScheduleRequest, SimDuration, SimTime, StepOutcome,
};
use std::any::{Any, TypeId};
use std::collections::{BTreeMap, BTreeSet};
use std::num::{NonZeroU64, NonZeroUsize};

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
    #[error("actor already has a domain context")]
    DuplicateActorDomainContext,
    #[error("actor already has a pending despawn")]
    DuplicateActorDespawn,
    #[error("reserved Flow event kind")]
    ReservedEventKind,
    #[error("unregistered domain event")]
    UnregisteredDomainEvent,
    #[error("invalid callback command ticket")]
    InvalidCommandTicket,
    #[error("callback command batch limit exceeded")]
    CallbackBatchLimitExceeded,
    #[error("same-tick transition budget exceeded at {at_ticks} (limit {limit})")]
    SameTickBudgetExceeded { at_ticks: u128, limit: u64 },
    #[error("Flow run is halted")]
    RunHalted,
}
/// Immutable run configuration for the experimental Flow surface.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FlowConfig {
    pub max_same_tick_flow_transitions: NonZeroU64,
}
impl Default for FlowConfig {
    fn default() -> Self {
        Self {
            max_same_tick_flow_transitions: NonZeroU64::new(100_000).unwrap(),
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FlowBudgetHalt {
    pub at: SimTime,
    pub consumed: u64,
    pub required_cost: u64,
    pub pending: ScheduledEventPreview,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FlowBudgetSnapshot {
    pub tick: Option<SimTime>,
    pub consumed: u64,
    pub limit: NonZeroU64,
    pub halted: Option<FlowBudgetHalt>,
    pub scheduler: SchedulerStats,
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
/// Immutable positive per-callback command bound, selected before execution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FlowCallbackConfig {
    pub max_callback_commands: NonZeroUsize,
}
impl Default for FlowCallbackConfig {
    fn default() -> Self {
        Self {
            max_callback_commands: NonZeroUsize::new(1024).unwrap(),
        }
    }
}
/// Restricted delivery callback; no runtime, scheduler or registry is exposed.
type FlowCallback<C> = fn(&mut C, &FlowCallbackSnapshot, &mut FlowCommandSink);
pub struct FlowContinuations<C> {
    pub on_resume: Option<FlowCallback<C>>,
    pub on_restart: Option<FlowCallback<C>>,
    pub on_abort: Option<FlowCallback<C>>,
    pub on_cancel: Option<FlowCallback<C>>,
    pub on_complete: Option<FlowCallback<C>>,
}
impl<C> Default for FlowContinuations<C> {
    fn default() -> Self {
        Self {
            on_resume: None,
            on_restart: None,
            on_abort: None,
            on_cancel: None,
            on_complete: None,
        }
    }
}
impl<C> FlowContinuations<C> {
    fn select(&self, transition: LifecycleTransition) -> Option<FlowCallback<C>> {
        match transition {
            LifecycleTransition::Resumed => self.on_resume,
            LifecycleTransition::Restarted => self.on_restart,
            LifecycleTransition::Aborted => self.on_abort,
            LifecycleTransition::Cancelled => self.on_cancel,
            LifecycleTransition::Completed => self.on_complete,
            _ => None,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowCallbackSnapshot {
    pub delivery: ScheduledEventPreview,
    pub origin: EventId,
    pub origin_ordinal: Option<u32>,
    pub work: WorkId,
    pub cause: FlowCallbackCause,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FlowCallbackCause {
    Work {
        transition: LifecycleTransition,
        progress: WorkProgress,
    },
    Domain {
        kind: EventKind,
    },
}
/// Borrowed view of the authoritative Flow world at committed delivery time.
/// The view exposes no mutation or registry/scheduler access.
pub struct FlowWorldView<'a> {
    world: &'a World,
    at: SimTime,
}
impl FlowWorldView<'_> {
    pub fn now(&self) -> SimTime {
        self.at
    }
    pub fn is_alive(&self, entity: EntityId) -> bool {
        self.world.is_alive(entity)
    }
}
/// Issued only by a live callback sink, scoped to its nonreused batch identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FlowCommandTicket {
    batch: u64,
    index: usize,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FlowRequestRef {
    Existing(RequestId),
    Submitted(FlowCommandTicket),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowAcquireCommand {
    pub resource: ResourceId,
    pub owner: EntityId,
    pub work: Option<WorkId>,
    pub at: SimTime,
    pub priority_level: i32,
    pub deadline: Option<SimTime>,
    pub scheduler_priority: i32,
    pub timed: bool,
    pub can_preempt: bool,
    pub preemptible: Option<PreemptionStrategy>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FlowOwnedCommand {
    Acquire(FlowAcquireCommand),
    Release {
        lease: LeaseId,
        at: SimTime,
    },
    Cancel {
        request: FlowRequestRef,
        at: SimTime,
        scheduler_priority: i32,
    },
    Reprioritize {
        request: FlowRequestRef,
        level: i32,
        at: SimTime,
        scheduler_priority: i32,
    },
    Domain {
        work: WorkId,
        kind: EventKind,
        at: SimTime,
        scheduler_priority: i32,
    },
    DespawnActor {
        actor: EntityId,
        at: SimTime,
        scheduler_priority: i32,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowCommandAdmission {
    pub ticket: FlowCommandTicket,
    pub event: EventId,
    pub request: Option<RequestId>,
    pub deadline_event: Option<EventId>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowBatchRejection {
    pub failed_ticket: Option<FlowCommandTicket>,
    pub error: FlowError,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FlowBatchReceipt {
    Accepted(Vec<FlowCommandAdmission>),
    Rejected(FlowBatchRejection),
}
/// Append-only, bounded command collection. Only runtime delivery creates a sink.
pub struct FlowCommandSink {
    batch: u64,
    config: FlowCallbackConfig,
    commands: Vec<FlowOwnedCommand>,
    next_index: usize,
    poison: Option<FlowError>,
}
impl FlowCommandSink {
    fn new(batch: u64, config: FlowCallbackConfig) -> Self {
        Self {
            batch,
            config,
            commands: Vec::new(),
            next_index: 0,
            poison: None,
        }
    }
    pub fn emit(&mut self, command: FlowOwnedCommand) -> Result<FlowCommandTicket, FlowError> {
        if let Some(error) = self.poison {
            return Err(error);
        }
        if self.commands.len() >= self.config.max_callback_commands.get() {
            self.poison = Some(FlowError::CallbackBatchLimitExceeded);
            return Err(FlowError::CallbackBatchLimitExceeded);
        }
        let next = match self.next_index.checked_add(1) {
            Some(next) => next,
            None => {
                self.poison = Some(FlowError::CounterOverflow);
                return Err(FlowError::CounterOverflow);
            }
        };
        // Bound requested vector growth by the configured command count.
        if self.commands.len() == self.commands.capacity() {
            let remaining = self.config.max_callback_commands.get() - self.commands.len();
            self.commands
                .reserve_exact(remaining.min(self.commands.len().max(1)));
        }
        let ticket = FlowCommandTicket {
            batch: self.batch,
            index: self.next_index,
        };
        self.commands.push(command);
        self.next_index = next;
        Ok(ticket)
    }
}

type ContinuationBridge =
    fn(&mut ComponentRegistry, EntityId, &FlowCallbackSnapshot, &mut FlowCommandSink, &dyn Any);
struct ContinuationDescriptor {
    context_type: TypeId,
    present: fn(&dyn Any, LifecycleTransition) -> bool,
    invoke: ContinuationBridge,
    context_present: fn(&ComponentRegistry, EntityId) -> bool,
    callbacks: Box<dyn Any>,
}
fn continuation_present<C: 'static>(callbacks: &dyn Any, transition: LifecycleTransition) -> bool {
    callbacks
        .downcast_ref::<FlowContinuations<C>>()
        .expect("validated continuation type")
        .select(transition)
        .is_some()
}
fn invoke_continuation<C: 'static>(
    registry: &mut ComponentRegistry,
    entity: EntityId,
    snapshot: &FlowCallbackSnapshot,
    sink: &mut FlowCommandSink,
    callbacks: &dyn Any,
) {
    let callback = match &snapshot.cause {
        FlowCallbackCause::Work { transition, .. } => callbacks
            .downcast_ref::<FlowContinuations<C>>()
            .expect("validated continuation type")
            .select(*transition),
        FlowCallbackCause::Domain { .. } => None,
    };
    if let Some(callback) = callback {
        if let Some(context) = registry
            .store_mut::<WorkContext<C>>()
            .and_then(|store| store.get_mut(entity))
        {
            callback(&mut context.0, snapshot, sink);
        }
    }
}
struct DomainCallback<C>(FlowCallback<C>);
type DomainViewBridge = fn(
    &World,
    &mut ComponentRegistry,
    EntityId,
    &FlowCallbackSnapshot,
    &mut FlowCommandSink,
    &dyn Any,
);
struct DomainViewCallback<C>(
    for<'a> fn(&'a mut C, &'a FlowCallbackSnapshot, FlowWorldView<'a>, &'a mut FlowCommandSink),
);
enum DomainInvocation {
    Legacy(ContinuationBridge),
    View(DomainViewBridge),
}
struct DomainDescriptor {
    context_type: TypeId,
    invoke: DomainInvocation,
    context_present: fn(&ComponentRegistry, EntityId) -> bool,
    callback: Box<dyn Any>,
}
fn invoke_domain<C: 'static>(
    registry: &mut ComponentRegistry,
    entity: EntityId,
    snapshot: &FlowCallbackSnapshot,
    sink: &mut FlowCommandSink,
    callback: &dyn Any,
) {
    let callback = callback
        .downcast_ref::<DomainCallback<C>>()
        .expect("validated domain context type")
        .0;
    if let Some(context) = registry
        .store_mut::<WorkContext<C>>()
        .and_then(|store| store.get_mut(entity))
    {
        callback(&mut context.0, snapshot, sink);
    }
}

fn invoke_domain_view<C: 'static>(
    world: &World,
    registry: &mut ComponentRegistry,
    entity: EntityId,
    snapshot: &FlowCallbackSnapshot,
    sink: &mut FlowCommandSink,
    callback: &dyn Any,
) {
    let callback = callback
        .downcast_ref::<DomainViewCallback<C>>()
        .expect("validated view-domain context type")
        .0;
    if let Some(context) = registry
        .store_mut::<WorkContext<C>>()
        .and_then(|store| store.get_mut(entity))
    {
        callback(
            &mut context.0,
            snapshot,
            FlowWorldView {
                world,
                at: snapshot.delivery.at,
            },
            sink,
        );
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
    context_present: fn(&ComponentRegistry, EntityId) -> bool,
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
type PrepareContext = fn(&ComponentRegistry, EntityId) -> Box<dyn PreparedContext>;
#[derive(Clone, Copy)]
struct WorkDescriptor {
    cleanup: ContextCleanup,
    context_present: fn(&ComponentRegistry, EntityId) -> bool,
    restart_present: fn(&ComponentRegistry, EntityId) -> bool,
    prepare: Option<PrepareContext>,
}
#[derive(Clone, Copy, Debug)]
enum NotificationKind {
    Legacy,
    Continuation,
}
#[derive(Clone, Debug)]
struct Notification {
    kind: NotificationKind,
    work: WorkId,
    transition: LifecycleTransition,
    progress: WorkProgress,
    origin: EventId,
    ordinal: u32,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WorkRole {
    Task,
    ActorDomain { actor: EntityId, kind: EventKind },
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
    pub callback_batches: Vec<FlowBatchReceipt>,
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
    Domain(WorkId, EventKind),
}
#[derive(Clone, Copy)]
enum BatchRequest {
    Existing(RequestId),
    Acquire(usize),
}
enum BatchCommand {
    Acquire {
        spec: FlowAcquireCommand,
        work_spec: Option<WorkSpec>,
    },
    Release {
        lease: LeaseId,
        at: SimTime,
    },
    Cancel {
        request: BatchRequest,
        at: SimTime,
        priority: i32,
    },
    Reprioritize {
        request: BatchRequest,
        level: i32,
        at: SimTime,
        priority: i32,
    },
    Domain {
        work: WorkId,
        kind: EventKind,
        at: SimTime,
        priority: i32,
    },
    DespawnActor {
        actor: EntityId,
        at: SimTime,
        priority: i32,
    },
}
struct BatchAdmissionPlan {
    batch: u64,
    commands: Vec<BatchCommand>,
    created: u64,
    scheduled: u64,
}

enum PreparedDelivery {
    Notification(Notification),
    Domain { work: WorkId, kind: EventKind },
}
impl PreparedDelivery {
    fn needs_batch(&self) -> bool {
        !matches!(
            self,
            Self::Notification(Notification {
                kind: NotificationKind::Legacy,
                ..
            })
        )
    }
}
#[derive(Clone)]
struct ResourceStage {
    capacity: ResourceCapacity,
    queue: ClaimQueue,
    active: ActiveAllocations,
}
struct DispatchPlan {
    resources: BTreeMap<ResourceId, ResourceStage>,
    requests: BTreeMap<RequestId, ResourceRequest>,
    progress: BTreeMap<WorkId, WorkProgress>,
    tokens: Vec<(Command, SimTime, Option<Notification>)>,
    factories: Vec<WorkId>,
    removed_works: Vec<(WorkId, WorkDescriptor)>,
    remove_resource: Option<ResourceId>,
    remove_actor: Option<EntityId>,
    destroyed: u64,
    scheduled: u64,
    admission: u64,
    lease_revision: u64,
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
        self.runtime.check_running()?;
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
    config: FlowConfig,
    callback_config: FlowCallbackConfig,
    next_batch_identity: u64,
    budget_tick: Option<SimTime>,
    budget_consumed: u64,
    budget_halt: Option<FlowBudgetHalt>,
    scheduler: Scheduler,
    world: World,
    registry: ComponentRegistry,
    resources: BTreeSet<ResourceId>,
    requests: BTreeSet<RequestId>,
    actors: BTreeSet<EntityId>,
    works: BTreeMap<WorkId, WorkDescriptor>,
    actor_domains: BTreeMap<EntityId, WorkId>,
    context_types: BTreeMap<String, TypeId>,
    handlers: BTreeMap<String, HandlerDescriptor>,
    continuations: BTreeMap<String, ContinuationDescriptor>,
    domain_hooks: BTreeMap<(String, EventKind), DomainDescriptor>,
    notifications: BTreeMap<EventId, Notification>,
    commands: BTreeMap<EventId, Command>,
    pending_releases: BTreeSet<LeaseId>,
    pending_despawns: BTreeSet<EntityId>,
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
        Self::with_config(FlowConfig::default())
    }
    pub fn with_config(config: FlowConfig) -> Self {
        Self::with_configs(config, FlowCallbackConfig::default())
    }
    pub fn with_configs(config: FlowConfig, callback_config: FlowCallbackConfig) -> Self {
        Self {
            config,
            callback_config,
            next_batch_identity: 0,
            budget_tick: None,
            budget_consumed: 0,
            budget_halt: None,
            scheduler: Scheduler::new(),
            world: World::new(),
            registry: ComponentRegistry::new(),
            resources: BTreeSet::new(),
            requests: BTreeSet::new(),
            actors: BTreeSet::new(),
            works: BTreeMap::new(),
            actor_domains: BTreeMap::new(),
            context_types: BTreeMap::new(),
            handlers: BTreeMap::new(),
            continuations: BTreeMap::new(),
            domain_hooks: BTreeMap::new(),
            notifications: BTreeMap::new(),
            commands: BTreeMap::new(),
            pending_releases: BTreeSet::new(),
            pending_despawns: BTreeSet::new(),
            created: 0,
            destroyed: 0,
            scheduled: 0,
            next_admission: 0,
            next_lease: 0,
        }
    }
    /// Require registration before any successful task or actor-domain creation.
    /// Historical context metadata keeps this phase closed after work cleanup.
    /// Existing registration methods retain their individual-key semantics.
    pub fn ensure_pre_work_registration(&self) -> Result<(), FlowError> {
        self.check_running()?;
        if !self.context_types.is_empty() {
            return Err(FlowError::InvalidWork);
        }
        Ok(())
    }
    pub fn budget_snapshot(&self) -> FlowBudgetSnapshot {
        FlowBudgetSnapshot {
            tick: self.budget_tick,
            consumed: self.budget_consumed,
            limit: self.config.max_same_tick_flow_transitions,
            halted: self.budget_halt,
            scheduler: self.scheduler.stats(),
        }
    }
    fn check_running(&self) -> Result<(), FlowError> {
        if self.budget_halt.is_some() {
            Err(FlowError::RunHalted)
        } else {
            Ok(())
        }
    }
    fn halt_error(&self) -> Option<FlowError> {
        self.budget_halt.map(|h| FlowError::SameTickBudgetExceeded {
            at_ticks: h.at.ticks(),
            limit: self.config.max_same_tick_flow_transitions.get(),
        })
    }
    pub fn now(&self) -> SimTime {
        self.scheduler.now()
    }
    fn spawn(&mut self) -> Result<EntityId, FlowError> {
        self.check_running()?;
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
        self.check_running()?;
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
        self.schedule_command(command, at, priority).map(|_| ())
    }
    fn schedule_command(
        &mut self,
        command: Command,
        at: SimTime,
        priority: i32,
    ) -> Result<EventId, FlowError> {
        self.check_schedule(at)?;
        self.scheduler
            .stats()
            .scheduled_events
            .checked_add(1)
            .filter(|n| *n <= OPERATION_CAP)
            .ok_or(FlowError::CounterOverflow)?;
        let event = self.scheduler.schedule(ScheduleRequest {
            at,
            priority,
            entity: None,
            kind: match command {
                Command::Domain(_, kind) => kind,
                _ => EventKind::custom(match command {
                    Command::Deadline(_) => FLOW_WAITING_DEADLINE_EVENT_KIND,
                    Command::Completion(..) => FLOW_TIMED_COMPLETION_EVENT_KIND,
                    Command::Notify => FLOW_WORK_NOTIFICATION_EVENT_KIND,
                    _ => FLOW_COMMAND_DISPATCH_EVENT_KIND,
                }),
            },
        });
        self.scheduled += 1;
        self.commands.insert(event, command);
        Ok(event)
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
        self.check_running()?;
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
        let _ = self.registry.insert(id.0, WorkRole::Task);
        self.context_types
            .insert(registration.to_owned(), TypeId::of::<C>());
        let _ = self.registry.insert(id.0, WorkProgress::new(duration));
        self.works.insert(
            id,
            WorkDescriptor {
                cleanup: cleanup_context::<C>,
                context_present: |registry, id| registry.get::<WorkContext<C>>(id).is_some(),
                restart_present: |_, _| false,
                prepare: None,
            },
        );
        Ok(id)
    }
    pub fn create_actor_domain_context<C: 'static>(
        &mut self,
        actor: EntityId,
        registration: &str,
        kind: EventKind,
        context: C,
    ) -> Result<WorkId, FlowError> {
        self.check_running()?;
        Self::check_domain_kind(kind)?;
        self.actor(actor)?;
        self.validate_context::<C>(registration)?;
        let descriptor = self
            .domain_hooks
            .get(&(registration.to_owned(), kind))
            .ok_or(FlowError::UnregisteredDomainEvent)?;
        if descriptor.context_type != TypeId::of::<C>()
            || !matches!(descriptor.invoke, DomainInvocation::View(_))
        {
            return Err(FlowError::InvalidWork);
        }
        if self.actor_domains.contains_key(&actor) {
            self.actor_domain_context(actor)?;
            return Err(FlowError::DuplicateActorDomainContext);
        }
        // A missing index may not conceal an already-owned carrier.
        if self.works.keys().any(|id| {
            matches!(self.registry.get::<WorkRole>(id.0), Some(WorkRole::ActorDomain { actor: owner, .. }) if *owner == actor)
        }) {
            return Err(FlowError::InvalidWork);
        }
        if self.created >= OPERATION_CAP {
            return Err(FlowError::CounterOverflow);
        }
        // All rejection paths precede create_work's one real entity allocation.
        // Remaining role/index writes cannot fail under these checked invariants.
        let work = self.create_work(actor, SimDuration::ZERO, registration, context)?;
        let _ = self
            .registry
            .insert(work.0, WorkRole::ActorDomain { actor, kind });
        self.actor_domains.insert(actor, work);
        Ok(work)
    }
    pub fn actor_domain_context(&self, actor: EntityId) -> Result<WorkId, FlowError> {
        self.actor(actor)?;
        let work = *self
            .actor_domains
            .get(&actor)
            .ok_or(FlowError::InvalidWork)?;
        if !matches!(self.validated_work_role(work), Ok(WorkRole::ActorDomain { actor: owner, .. }) if owner == actor)
            || !self
                .works
                .get(&work)
                .is_some_and(|d| (d.context_present)(&self.registry, work.0))
        {
            return Err(FlowError::InvalidWork);
        }
        Ok(work)
    }
    fn validated_work_role(&self, work: WorkId) -> Result<WorkRole, FlowError> {
        let spec = self.work(work).map_err(|_| FlowError::InvalidState)?;
        let role = *self
            .registry
            .get::<WorkRole>(work.0)
            .ok_or(FlowError::InvalidState)?;
        match role {
            WorkRole::Task => {
                if self.actor_domains.values().any(|id| *id == work) {
                    return Err(FlowError::InvalidState);
                }
            }
            WorkRole::ActorDomain { actor, kind } => {
                let descriptor = self
                    .domain_hooks
                    .get(&(spec.context_type_key.clone(), kind))
                    .ok_or(FlowError::InvalidState)?;
                if spec.owner != actor
                    || spec.original_duration != SimDuration::ZERO
                    || spec.request.is_some()
                    || self.actor_domains.get(&actor) != Some(&work)
                    || self
                        .actor_domains
                        .iter()
                        .any(|(owner, id)| *id == work && *owner != actor)
                    || !self
                        .registry
                        .get::<WorkProgress>(work.0)
                        .is_some_and(|p| p.state == WorkState::Pending)
                    || !matches!(descriptor.invoke, DomainInvocation::View(_))
                    || self.context_types.get(&spec.context_type_key)
                        != Some(&descriptor.context_type)
                {
                    return Err(FlowError::InvalidState);
                }
            }
        }
        Ok(role)
    }
    fn check_work_domain_kind(&self, work: WorkId, kind: EventKind) -> Result<(), FlowError> {
        match self.validated_work_role(work)? {
            WorkRole::ActorDomain { kind: bound, .. } if kind != bound => {
                Err(FlowError::InvalidWork)
            }
            _ => Ok(()),
        }
    }
    pub fn register_work_handlers<C: 'static>(
        &mut self,
        registration: &str,
        handlers: WorkHandlers<C>,
    ) -> Result<(), FlowError> {
        self.check_registration::<C>(registration)?;
        if self.handlers.contains_key(registration) {
            return Err(FlowError::InvalidWork);
        }
        self.handlers.insert(
            registration.to_owned(),
            HandlerDescriptor {
                context_type: TypeId::of::<C>(),
                present: handler_present::<C>,
                invoke: invoke_handler::<C>,
                context_present: |registry, id| registry.get::<WorkContext<C>>(id).is_some(),
                handlers: Box::new(handlers),
            },
        );
        Ok(())
    }
    fn check_registration<C: 'static>(&self, registration: &str) -> Result<(), FlowError> {
        self.check_running()?;
        self.validate_context::<C>(registration)?;
        if self.context_types.contains_key(registration) {
            return Err(FlowError::InvalidWork);
        }
        Ok(())
    }
    fn check_domain_kind(kind: EventKind) -> Result<(), FlowError> {
        if (FLOW_COMMAND_DISPATCH_EVENT_KIND..=FLOW_WORK_NOTIFICATION_EVENT_KIND)
            .contains(&kind.code())
        {
            return Err(FlowError::ReservedEventKind);
        }
        Ok(())
    }
    pub fn register_work_continuations<C: 'static>(
        &mut self,
        registration: &str,
        callbacks: FlowContinuations<C>,
    ) -> Result<(), FlowError> {
        self.check_registration::<C>(registration)?;
        if self.continuations.contains_key(registration) {
            return Err(FlowError::InvalidWork);
        }
        self.continuations.insert(
            registration.to_owned(),
            ContinuationDescriptor {
                context_type: TypeId::of::<C>(),
                present: continuation_present::<C>,
                invoke: invoke_continuation::<C>,
                context_present: |registry, entity| {
                    registry.get::<WorkContext<C>>(entity).is_some()
                },
                callbacks: Box::new(callbacks),
            },
        );
        Ok(())
    }
    pub fn register_domain_hook<C: 'static>(
        &mut self,
        registration: &str,
        kind: EventKind,
        callback: FlowCallback<C>,
    ) -> Result<(), FlowError> {
        self.check_running()?;
        Self::check_domain_kind(kind)?;
        self.check_registration::<C>(registration)?;
        let key = (registration.to_owned(), kind);
        if self.domain_hooks.contains_key(&key) {
            return Err(FlowError::InvalidWork);
        }
        self.domain_hooks.insert(
            key,
            DomainDescriptor {
                context_type: TypeId::of::<C>(),
                invoke: DomainInvocation::Legacy(invoke_domain::<C>),
                context_present: |registry, entity| {
                    registry.get::<WorkContext<C>>(entity).is_some()
                },
                callback: Box::new(DomainCallback(callback)),
            },
        );
        Ok(())
    }
    pub fn register_domain_view_hook<C: 'static>(
        &mut self,
        registration: &str,
        kind: EventKind,
        callback: for<'a> fn(
            &'a mut C,
            &'a FlowCallbackSnapshot,
            FlowWorldView<'a>,
            &'a mut FlowCommandSink,
        ),
    ) -> Result<(), FlowError> {
        self.check_running()?;
        Self::check_domain_kind(kind)?;
        self.check_registration::<C>(registration)?;
        let key = (registration.to_owned(), kind);
        if self.domain_hooks.contains_key(&key) {
            return Err(FlowError::InvalidWork);
        }
        self.domain_hooks.insert(
            key,
            DomainDescriptor {
                context_type: TypeId::of::<C>(),
                invoke: DomainInvocation::View(invoke_domain_view::<C>),
                context_present: |registry, entity| {
                    registry.get::<WorkContext<C>>(entity).is_some()
                },
                callback: Box::new(DomainViewCallback(callback)),
            },
        );
        Ok(())
    }
    pub fn schedule_domain(
        &mut self,
        work: WorkId,
        kind: EventKind,
        at: SimTime,
        priority: i32,
    ) -> Result<EventId, FlowError> {
        self.check_running()?;
        Self::check_domain_kind(kind)?;
        let spec = self.work(work)?;
        self.actor(spec.owner)?;
        self.check_work_domain_kind(work, kind)?;
        let descriptor = self
            .domain_hooks
            .get(&(spec.context_type_key, kind))
            .ok_or(FlowError::UnregisteredDomainEvent)?;
        if !(descriptor.context_present)(&self.registry, work.0) {
            return Err(FlowError::InvalidWork);
        }
        self.schedule_command(Command::Domain(work, kind), at, priority)
    }
    pub fn create_restartable_work<T: 'static, C: 'static>(
        &mut self,
        owner: EntityId,
        duration: SimDuration,
        registration: &str,
        initial_template: T,
        make_context: fn(&T) -> C,
    ) -> Result<WorkId, FlowError> {
        self.check_running()?;
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
                context_present: |registry, id| registry.get::<WorkContext<C>>(id).is_some(),
                restart_present: |registry, id| registry.get::<RestartTemplate<T, C>>(id).is_some(),
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
            || self
                .continuations
                .get(registration)
                .is_some_and(|h| h.context_type != TypeId::of::<C>())
            || self
                .domain_hooks
                .iter()
                .any(|((key, _), h)| key == registration && h.context_type != TypeId::of::<C>())
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
    ) -> Vec<Notification> {
        let Some(spec) = self.registry.get::<WorkSpec>(work.0) else {
            return Vec::new();
        };
        let mut notifications = Vec::new();
        let mut append = |kind| {
            notifications.push(Notification {
                kind,
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
        };
        if self
            .handlers
            .get(&spec.context_type_key)
            .is_some_and(|h| (h.present)(h.handlers.as_ref(), transition))
        {
            append(NotificationKind::Legacy);
        }
        if self
            .continuations
            .get(&spec.context_type_key)
            .is_some_and(|h| (h.present)(h.callbacks.as_ref(), transition))
        {
            append(NotificationKind::Continuation);
        }
        notifications
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
        self.check_running()?;
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
                if self
                    .validated_work_role(id)
                    .map_err(|_| FlowError::InvalidWork)?
                    != WorkRole::Task
                {
                    return Err(FlowError::InvalidWork);
                }
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
        self.check_running()?;
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
        self.check_running()?;
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
        self.check_running()?;
        let r = self.request(id)?;
        if terminal(r.state) {
            return Err(FlowError::TerminalRequest);
        }
        self.schedule_priority(Command::Reprioritize(id, level), at, priority)
    }
    pub fn set_capacity(&mut self, id: ResourceId, total: u32) -> Result<(), FlowError> {
        self.check_running()?;
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
        self.check_running()?;
        self.resource(id)?;
        if self.in_use(id) {
            return Err(FlowError::ResourceInUse);
        }
        self.schedule(Command::Remove(id), self.now())
    }
    pub fn despawn_actor(&mut self, id: EntityId) -> Result<(), FlowError> {
        self.despawn_actor_at_with_scheduler_priority(id, self.now(), 0)
    }
    pub fn despawn_actor_at_with_scheduler_priority(
        &mut self,
        actor: EntityId,
        at: SimTime,
        priority: i32,
    ) -> Result<(), FlowError> {
        self.check_running()?;
        self.actor(actor)?;
        if self.pending_despawns.contains(&actor) {
            return Err(FlowError::DuplicateActorDespawn);
        }
        // Scheduling checks time and both lifetime counters before allocating
        // the actual event. Publishing the reservation is then infallible.
        self.schedule_priority(Command::Despawn(actor), at, priority)?;
        self.pending_despawns.insert(actor);
        Ok(())
    }
    pub fn run_for(&mut self, max_events: u64) -> Result<FlowRun, FlowError> {
        if let Some(error) = self.halt_error() {
            return Err(error);
        }
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
        if let Some(error) = self.halt_error() {
            return Err(error);
        }
        let Some(preview) = self.scheduler.peek_next() else {
            return Ok(None);
        };
        self.scheduler
            .stats()
            .dispatched_events
            .checked_add(1)
            .filter(|n| *n <= OPERATION_CAP)
            .ok_or(FlowError::CounterOverflow)?;
        let command = self
            .commands
            .get(&preview.id)
            .cloned()
            .ok_or(FlowError::InvalidState)?;
        let mut outcome = FlowDispatch {
            event: preview.id,
            at: preview.at,
            records: Vec::new(),
            error: None,
            callback_batches: Vec::new(),
        };
        let plan = if matches!(command, Command::Notify | Command::Domain(..)) {
            None
        } else {
            self.plan(command, &mut outcome)?
        };
        let delivery = match command {
            Command::Notify => self
                .notifications
                .get(&preview.id)
                .filter(|n| self.notification_deliverable(n))
                .cloned()
                .map(PreparedDelivery::Notification),
            Command::Domain(work, kind) => match self.domain_delivery(work, kind) {
                Ok(delivery) => delivery,
                // Structural role/index corruption rejects before consuming the head.
                // Ordinary absent descriptors remain consumed semantic errors.
                Err(FlowError::InvalidState) => return Err(FlowError::InvalidState),
                Err(error) => {
                    outcome.error = Some(error);
                    None
                }
            },
            _ => None,
        };
        let next_batch = if delivery.as_ref().is_some_and(PreparedDelivery::needs_batch) {
            Some(
                self.next_batch_identity
                    .checked_add(1)
                    .ok_or(FlowError::CounterOverflow)?,
            )
        } else {
            None
        };
        let cost = u64::try_from(outcome.records.len())
            .map_err(|_| FlowError::CounterOverflow)?
            .checked_add(u64::from(delivery.is_some()))
            .ok_or(FlowError::CounterOverflow)?;
        let consumed = if self.budget_tick == Some(preview.at) {
            self.budget_consumed
        } else {
            0
        };
        let total = consumed
            .checked_add(cost)
            .ok_or(FlowError::CounterOverflow)?;
        if total > self.config.max_same_tick_flow_transitions.get() {
            self.budget_halt = Some(FlowBudgetHalt {
                at: preview.at,
                consumed,
                required_cost: cost,
                pending: preview,
            });
            return Err(self.halt_error().unwrap());
        }
        match self.scheduler.step() {
            StepOutcome::Dispatched(event) => {
                assert_eq!(event.id, preview.id);
                assert_eq!(event.at, preview.at);
            }
            _ => unreachable!("validated live scheduler head"),
        }
        self.commands.remove(&preview.id);
        if let Command::Release(lease) = command {
            self.pending_releases.remove(&lease);
        }
        if let Command::Despawn(actor) = command {
            self.pending_despawns.remove(&actor);
        }
        if let Some(plan) = plan {
            self.commit_plan(plan);
        }
        self.budget_tick = Some(preview.at);
        self.budget_consumed = total;
        if matches!(command, Command::Notify) {
            self.notifications.remove(&preview.id);
        }
        if let Some(delivery) = delivery {
            match delivery {
                PreparedDelivery::Notification(n) if matches!(n.kind, NotificationKind::Legacy) => {
                    let spec = self
                        .registry
                        .get::<WorkSpec>(n.work.0)
                        .expect("validated notification work");
                    let h = &self.handlers[&spec.context_type_key];
                    (h.invoke)(
                        &mut self.registry,
                        n.work.0,
                        &n.progress,
                        n.transition,
                        h.handlers.as_ref(),
                    );
                }
                delivery => {
                    let batch = self.next_batch_identity;
                    self.next_batch_identity = next_batch.expect("preflighted batch identity");
                    let mut sink = FlowCommandSink::new(batch, self.callback_config);
                    match delivery {
                        PreparedDelivery::Notification(n) => {
                            let key = self
                                .registry
                                .get::<WorkSpec>(n.work.0)
                                .expect("validated continuation work")
                                .context_type_key
                                .clone();
                            let snapshot = FlowCallbackSnapshot {
                                delivery: preview,
                                origin: n.origin,
                                origin_ordinal: Some(n.ordinal),
                                work: n.work,
                                cause: FlowCallbackCause::Work {
                                    transition: n.transition,
                                    progress: n.progress,
                                },
                            };
                            let h = &self.continuations[&key];
                            (h.invoke)(
                                &mut self.registry,
                                n.work.0,
                                &snapshot,
                                &mut sink,
                                h.callbacks.as_ref(),
                            );
                        }
                        PreparedDelivery::Domain { work, kind } => {
                            let key = self
                                .registry
                                .get::<WorkSpec>(work.0)
                                .expect("validated domain work")
                                .context_type_key
                                .clone();
                            let snapshot = FlowCallbackSnapshot {
                                delivery: preview,
                                origin: preview.id,
                                origin_ordinal: None,
                                work,
                                cause: FlowCallbackCause::Domain { kind },
                            };
                            let h = &self.domain_hooks[&(key, kind)];
                            match h.invoke {
                                DomainInvocation::Legacy(invoke) => invoke(
                                    &mut self.registry,
                                    work.0,
                                    &snapshot,
                                    &mut sink,
                                    h.callback.as_ref(),
                                ),
                                DomainInvocation::View(invoke) => invoke(
                                    &self.world,
                                    &mut self.registry,
                                    work.0,
                                    &snapshot,
                                    &mut sink,
                                    h.callback.as_ref(),
                                ),
                            }
                        }
                    }
                    // Validate the entire emitted batch after this once-only delivery.
                    // Rejection retains context effects and consumes no command IDs.
                    outcome
                        .callback_batches
                        .push(self.admit_callback_batch(sink));
                }
            }
        }
        Ok(Some(outcome))
    }
    fn batch_request(
        &self,
        reference: &FlowRequestRef,
        batch: u64,
        position: usize,
        commands: &[FlowOwnedCommand],
    ) -> Result<BatchRequest, FlowError> {
        match reference {
            FlowRequestRef::Existing(id) => {
                if terminal(self.request(*id)?.state) {
                    return Err(FlowError::TerminalRequest);
                }
                Ok(BatchRequest::Existing(*id))
            }
            FlowRequestRef::Submitted(ticket) => {
                if ticket.batch != batch
                    || ticket.index >= position
                    || !matches!(
                        commands.get(ticket.index),
                        Some(FlowOwnedCommand::Acquire(_))
                    )
                {
                    return Err(FlowError::InvalidCommandTicket);
                }
                Ok(BatchRequest::Acquire(ticket.index))
            }
        }
    }
    fn plan_callback_batch(
        &self,
        sink: &FlowCommandSink,
        scheduler_scheduled: u64,
    ) -> Result<BatchAdmissionPlan, FlowBatchRejection> {
        if let Some(error) = sink.poison {
            return Err(FlowBatchRejection {
                failed_ticket: None,
                error,
            });
        }
        let mut commands = Vec::with_capacity(sink.commands.len());
        let mut associated = BTreeSet::new();
        let mut releases = self.pending_releases.clone();
        let mut despawns = self.pending_despawns.clone();
        let mut created = self.created;
        let mut event_count = 0u64;
        for (position, command) in sink.commands.iter().enumerate() {
            let ticket = FlowCommandTicket {
                batch: sink.batch,
                index: position,
            };
            let validated = (|| -> Result<BatchCommand, FlowError> {
                self.check_running()?;
                let (command, at, needed) = match command {
                    FlowOwnedCommand::Acquire(spec) => {
                        self.actor(spec.owner)?;
                        self.resource(spec.resource)?;
                        if (spec.timed && spec.work.is_none())
                            || (spec.preemptible.is_some() && (!spec.timed || spec.work.is_none()))
                        {
                            return Err(FlowError::InvalidWork);
                        }
                        let work_spec = if let Some(work) = spec.work {
                            let work_spec = self.work(work)?;
                            if self
                                .validated_work_role(work)
                                .map_err(|_| FlowError::InvalidWork)?
                                != WorkRole::Task
                            {
                                return Err(FlowError::InvalidWork);
                            }
                            let descriptor = self.works.get(&work).ok_or(FlowError::InvalidWork)?;
                            if work_spec.owner != spec.owner
                                || work_spec.request.is_some()
                                || !associated.insert(work)
                                || !(descriptor.context_present)(&self.registry, work.0)
                            {
                                return Err(FlowError::InvalidWork);
                            }
                            if spec.preemptible == Some(PreemptionStrategy::Restart)
                                && (descriptor.prepare.is_none()
                                    || !(descriptor.restart_present)(&self.registry, work.0))
                            {
                                return Err(FlowError::InvalidWork);
                            }
                            Some(work_spec)
                        } else {
                            None
                        };
                        created = created
                            .checked_add(1)
                            .filter(|n| *n <= OPERATION_CAP)
                            .ok_or(FlowError::CounterOverflow)?;
                        let needed =
                            1 + u64::from(spec.deadline.is_some_and(|deadline| deadline > spec.at));
                        (
                            BatchCommand::Acquire {
                                spec: spec.clone(),
                                work_spec,
                            },
                            spec.at,
                            needed,
                        )
                    }
                    FlowOwnedCommand::Release { lease, at } => {
                        let request = self.request(lease.request)?;
                        if request.state != RequestState::Active
                            || request.lease != Some(*lease)
                            || !releases.insert(*lease)
                        {
                            return Err(FlowError::InvalidLease);
                        }
                        (
                            BatchCommand::Release {
                                lease: *lease,
                                at: *at,
                            },
                            *at,
                            1,
                        )
                    }
                    FlowOwnedCommand::Cancel {
                        request,
                        at,
                        scheduler_priority,
                    } => {
                        let request =
                            self.batch_request(request, sink.batch, position, &sink.commands)?;
                        (
                            BatchCommand::Cancel {
                                request,
                                at: *at,
                                priority: *scheduler_priority,
                            },
                            *at,
                            1,
                        )
                    }
                    FlowOwnedCommand::Reprioritize {
                        request,
                        level,
                        at,
                        scheduler_priority,
                    } => {
                        let request =
                            self.batch_request(request, sink.batch, position, &sink.commands)?;
                        (
                            BatchCommand::Reprioritize {
                                request,
                                level: *level,
                                at: *at,
                                priority: *scheduler_priority,
                            },
                            *at,
                            1,
                        )
                    }
                    FlowOwnedCommand::DespawnActor {
                        actor,
                        at,
                        scheduler_priority,
                    } => {
                        self.actor(*actor)?;
                        if !despawns.insert(*actor) {
                            return Err(FlowError::DuplicateActorDespawn);
                        }
                        (
                            BatchCommand::DespawnActor {
                                actor: *actor,
                                at: *at,
                                priority: *scheduler_priority,
                            },
                            *at,
                            1,
                        )
                    }
                    FlowOwnedCommand::Domain {
                        work,
                        kind,
                        at,
                        scheduler_priority,
                    } => {
                        Self::check_domain_kind(*kind)?;
                        let spec = self.work(*work)?;
                        self.actor(spec.owner)?;
                        self.check_work_domain_kind(*work, *kind)?;
                        let h = self
                            .domain_hooks
                            .get(&(spec.context_type_key, *kind))
                            .ok_or(FlowError::UnregisteredDomainEvent)?;
                        if !(h.context_present)(&self.registry, work.0) {
                            return Err(FlowError::InvalidWork);
                        }
                        (
                            BatchCommand::Domain {
                                work: *work,
                                kind: *kind,
                                at: *at,
                                priority: *scheduler_priority,
                            },
                            *at,
                            1,
                        )
                    }
                };
                if at < self.now() {
                    return Err(FlowError::PastCommand);
                }
                event_count = event_count
                    .checked_add(needed)
                    .ok_or(FlowError::CounterOverflow)?;
                self.scheduled
                    .checked_add(event_count)
                    .filter(|n| *n <= OPERATION_CAP)
                    .ok_or(FlowError::CounterOverflow)?;
                scheduler_scheduled
                    .checked_add(event_count)
                    .filter(|n| *n <= OPERATION_CAP)
                    .ok_or(FlowError::CounterOverflow)?;
                Ok(command)
            })()
            .map_err(|error| FlowBatchRejection {
                failed_ticket: Some(ticket),
                error,
            })?;
            commands.push(validated);
        }
        Ok(BatchAdmissionPlan {
            batch: sink.batch,
            commands,
            created,
            scheduled: self.scheduled + event_count,
        })
    }
    fn admit_callback_batch(&mut self, sink: FlowCommandSink) -> FlowBatchReceipt {
        match self.plan_callback_batch(&sink, self.scheduler.stats().scheduled_events) {
            Ok(plan) => FlowBatchReceipt::Accepted(self.commit_callback_batch(plan)),
            Err(rejection) => FlowBatchReceipt::Rejected(rejection),
        }
    }
    fn commit_callback_batch(&mut self, plan: BatchAdmissionPlan) -> Vec<FlowCommandAdmission> {
        let mut requests = Vec::with_capacity(plan.commands.len());
        let mut receipts = Vec::with_capacity(plan.commands.len());
        for (position, command) in plan.commands.into_iter().enumerate() {
            let mut request_id = None;
            let mut deadline_event = None;
            let (command, at, priority) = match command {
                BatchCommand::Acquire {
                    spec,
                    mut work_spec,
                } => {
                    // World is private and lifetime created/despawn counters are capped.
                    // No prospective allocator ID or public-ingress mutation loop is used.
                    let id = RequestId(self.world.spawn());
                    self.created += 1;
                    let request = ResourceRequest {
                        resource: spec.resource,
                        owner: spec.owner,
                        state: RequestState::Pending,
                        admission_sequence: None,
                        lease: None,
                        priority_level: spec.priority_level,
                        work: spec.work,
                        submitted_at: spec.at,
                        deadline: spec.deadline,
                        timed: spec.timed,
                        can_preempt: spec.can_preempt,
                        preemptible: spec.preemptible,
                    };
                    assert!(
                        self.registry.insert(id.0, request),
                        "preflighted fresh request generation"
                    );
                    self.requests.insert(id);
                    if let (Some(work), Some(work_spec)) = (spec.work, work_spec.as_mut()) {
                        work_spec.request = Some(id);
                        assert!(
                            self.registry.insert(work.0, work_spec.clone()),
                            "preflighted live work metadata"
                        );
                    }
                    request_id = Some(id);
                    // Primary precedes its deadline in admission order, matching public ingress.
                    let event = self.commit_batch_event(
                        Command::Submit(id),
                        spec.at,
                        spec.scheduler_priority,
                    );
                    if let Some(deadline) = spec.deadline.filter(|deadline| *deadline > spec.at) {
                        deadline_event =
                            Some(self.commit_batch_event(Command::Deadline(id), deadline, 0));
                    }
                    requests.push(Some(id));
                    receipts.push(FlowCommandAdmission {
                        ticket: FlowCommandTicket {
                            batch: plan.batch,
                            index: position,
                        },
                        event,
                        request: request_id,
                        deadline_event,
                    });
                    continue;
                }
                BatchCommand::Release { lease, at } => {
                    self.pending_releases.insert(lease);
                    (Command::Release(lease), at, 0)
                }
                BatchCommand::Cancel {
                    request,
                    at,
                    priority,
                } => (
                    Command::Cancel(Self::resolve_batch_request(request, &requests)),
                    at,
                    priority,
                ),
                BatchCommand::Reprioritize {
                    request,
                    level,
                    at,
                    priority,
                } => (
                    Command::Reprioritize(Self::resolve_batch_request(request, &requests), level),
                    at,
                    priority,
                ),
                BatchCommand::DespawnActor {
                    actor,
                    at,
                    priority,
                } => {
                    self.pending_despawns.insert(actor);
                    (Command::Despawn(actor), at, priority)
                }
                BatchCommand::Domain {
                    work,
                    kind,
                    at,
                    priority,
                } => (Command::Domain(work, kind), at, priority),
            };
            let event = self.commit_batch_event(command, at, priority);
            requests.push(None);
            receipts.push(FlowCommandAdmission {
                ticket: FlowCommandTicket {
                    batch: plan.batch,
                    index: position,
                },
                event,
                request: request_id,
                deadline_event,
            });
        }
        assert_eq!(self.created, plan.created, "aggregate creation preflight");
        assert_eq!(self.scheduled, plan.scheduled, "aggregate event preflight");
        receipts
    }
    fn resolve_batch_request(request: BatchRequest, requests: &[Option<RequestId>]) -> RequestId {
        match request {
            BatchRequest::Existing(id) => id,
            BatchRequest::Acquire(index) => {
                requests[index].expect("preflighted earlier acquired request")
            }
        }
    }
    fn commit_batch_event(&mut self, command: Command, at: SimTime, priority: i32) -> EventId {
        // Every scheduler call is owned by this facade. Validated lifetime counts
        // bound index, sequence and generation before the first batch mutation.
        let kind = match command {
            Command::Domain(_, kind) => kind,
            Command::Deadline(_) => EventKind::custom(FLOW_WAITING_DEADLINE_EVENT_KIND),
            _ => EventKind::custom(FLOW_COMMAND_DISPATCH_EVENT_KIND),
        };
        let event = self.scheduler.schedule(ScheduleRequest {
            at,
            priority,
            entity: None,
            kind,
        });
        self.scheduled += 1;
        self.commands.insert(event, command);
        event
    }

    fn notification_deliverable(&self, n: &Notification) -> bool {
        self.works.contains_key(&n.work)
            && self.world.is_alive(n.work.0)
            && self.registry.get::<WorkSpec>(n.work.0).is_some_and(|spec| {
                self.actors.contains(&spec.owner)
                    && self.world.is_alive(spec.owner)
                    && match n.kind {
                        NotificationKind::Legacy => {
                            self.handlers.get(&spec.context_type_key).is_some_and(|h| {
                                (h.present)(h.handlers.as_ref(), n.transition)
                                    && (h.context_present)(&self.registry, n.work.0)
                            })
                        }
                        NotificationKind::Continuation => self
                            .continuations
                            .get(&spec.context_type_key)
                            .is_some_and(|h| {
                                (h.present)(h.callbacks.as_ref(), n.transition)
                                    && (h.context_present)(&self.registry, n.work.0)
                            }),
                    }
            })
    }
    fn domain_delivery(
        &self,
        work: WorkId,
        kind: EventKind,
    ) -> Result<Option<PreparedDelivery>, FlowError> {
        if !self.works.contains_key(&work) || !self.world.is_alive(work.0) {
            return Ok(None);
        }
        let Some(spec) = self.registry.get::<WorkSpec>(work.0) else {
            // A live carrier cannot lose its authoritative metadata and become
            // a stale Task event. Either ownership witness retains this head.
            if matches!(
                self.registry.get::<WorkRole>(work.0),
                Some(WorkRole::ActorDomain { .. })
            ) || self.actor_domains.values().any(|carrier| *carrier == work)
            {
                return Err(FlowError::InvalidState);
            }
            return Ok(None);
        };
        if !self.actors.contains(&spec.owner) || !self.world.is_alive(spec.owner) {
            return Ok(None);
        }
        self.check_work_domain_kind(work, kind)?;
        let h = self
            .domain_hooks
            .get(&(spec.context_type_key.clone(), kind))
            .ok_or(FlowError::UnregisteredDomainEvent)?;
        if !(h.context_present)(&self.registry, work.0) {
            return Ok(None);
        }
        Ok(Some(PreparedDelivery::Domain { work, kind }))
    }
    fn plan(
        &self,
        command: Command,
        outcome: &mut FlowDispatch,
    ) -> Result<Option<DispatchPlan>, FlowError> {
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
                return Ok(None);
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
            Command::Notify | Command::Domain(..) => BTreeSet::new(),
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
                for n in self.notification(
                    work,
                    LifecycleTransition::Completed,
                    &progress[&work],
                    outcome,
                ) {
                    tokens.push((Command::Notify, outcome.at, Some(n)));
                }
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
                            for n in self.notification(
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
                Command::Notify | Command::Domain(..) => return Err(FlowError::InvalidState),
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
                        for n in self.notification(
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
                            for n in
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
                    for n in self.notification(work, transition, &progress[&work], outcome) {
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
                        for n in self.notification(
                            work,
                            LifecycleTransition::Completed,
                            &progress[&work],
                            outcome,
                        ) {
                            tokens.push((Command::Notify, outcome.at, Some(n)));
                        }
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
        // Every Scheduler::schedule call belongs to Flow ingress or this token list.
        // The u32 operation cap also bounds scheduler index/sequence/counter additions
        // before schedule (whose existing API returns EventId without a Result).
        self.scheduler
            .stats()
            .scheduled_events
            .checked_add(token_count)
            .filter(|n| *n <= OPERATION_CAP)
            .ok_or(FlowError::CounterOverflow)?;
        let mut cleanup_ids = BTreeSet::new();
        if let Some(resource) = remove_resource {
            if !self.world.is_alive(resource.0) {
                return Err(FlowError::InvalidState);
            }
            cleanup_ids.insert(resource.0);
        }
        if let Some(owner) = remove_actor {
            self.actor(owner)?;
            if let Some(carrier) = self.actor_domains.get(&owner) {
                if !removed_works.iter().any(|(work, _)| work == carrier)
                    || !matches!(self.validated_work_role(*carrier), Ok(WorkRole::ActorDomain { actor, .. }) if actor == owner)
                {
                    return Err(FlowError::InvalidState);
                }
            }
            cleanup_ids.insert(owner);
        }
        for (work, descriptor) in &removed_works {
            if self.validated_work_role(*work).is_err() {
                return Err(FlowError::InvalidState);
            }
            if !self.world.is_alive(work.0)
                || !(descriptor.context_present)(&self.registry, work.0)
                || !self
                    .registry
                    .get::<WorkSpec>(work.0)
                    .is_some_and(|spec| Some(spec.owner) == remove_actor)
                || self.registry.get::<WorkProgress>(work.0).is_none()
                || (descriptor.prepare.is_some()
                    && !(descriptor.restart_present)(&self.registry, work.0))
                || !cleanup_ids.insert(work.0)
            {
                return Err(FlowError::InvalidState);
            }
        }
        if u64::try_from(cleanup_ids.len()).map_err(|_| FlowError::CounterOverflow)? != despawns {
            return Err(FlowError::InvalidState);
        }
        for work in &factories {
            let descriptor = self.works.get(work).ok_or(FlowError::InvalidState)?;
            if descriptor.prepare.is_none()
                || !(descriptor.context_present)(&self.registry, work.0)
                || !self.world.is_alive(work.0)
                || !(descriptor.restart_present)(&self.registry, work.0)
            {
                return Err(FlowError::InvalidState);
            }
        }
        Ok(Some(DispatchPlan {
            resources,
            requests,
            progress,
            tokens,
            factories,
            removed_works,
            remove_resource,
            remove_actor,
            destroyed,
            scheduled,
            admission,
            lease_revision,
        }))
    }
    fn commit_plan(&mut self, plan: DispatchPlan) {
        let DispatchPlan {
            resources,
            requests,
            progress,
            tokens,
            factories,
            removed_works,
            remove_resource,
            remove_actor,
            destroyed,
            scheduled,
            admission,
            lease_revision,
        } = plan;
        let prepared: Vec<_> = factories
            .into_iter()
            .map(|w| {
                (
                    w,
                    (self.works[&w].prepare.expect("preflighted factory"))(&self.registry, w.0),
                )
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
            self.registry.remove::<WorkRole>(work.0);
            self.world.despawn(work.0);
        }
        if let Some(id) = remove_actor {
            self.actor_domains.remove(&id);
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
            callback_batches: Vec::new(),
        };
        if let Some(plan) = f.plan(command, &mut outcome)? {
            f.commit_plan(plan);
        }
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
            let head = f.scheduler.peek_next();
            let stats = f.scheduler.stats();
            assert_eq!(f.step().unwrap_err(), FlowError::CounterOverflow);
            assert_eq!(f.scheduler.peek_next(), head);
            assert_eq!(f.scheduler.stats(), stats);
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
        f.schedule(Command::Capacity(resource, 2), ticks(1))
            .unwrap();
        let head = f.scheduler.peek_next();
        let stats = f.scheduler.stats();
        assert_eq!(f.step().unwrap_err(), FlowError::CounterOverflow);
        assert_eq!(f.scheduler.peek_next(), head);
        assert_eq!(f.scheduler.stats(), stats);
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
        assert_eq!(f.step().unwrap_err(), FlowError::CounterOverflow);
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
        assert_eq!(f.step().unwrap_err(), FlowError::CounterOverflow);
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
        let head = f.scheduler.peek_next();
        let stats = f.scheduler.stats();
        assert_eq!(f.step().unwrap_err(), FlowError::CounterOverflow);
        assert_eq!(f.scheduler.peek_next(), head);
        assert_eq!(f.scheduler.stats(), stats);
        assert_eq!(f.resource(r).unwrap(), before);
        assert_eq!(f.request(q).unwrap().state, RequestState::Queued);
    }
    #[test]
    fn failed_release_preflight_preserves_lease_head_and_reservation() {
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
        assert_eq!(f.step().unwrap_err(), FlowError::CounterOverflow);
        assert_eq!(f.resource(r).unwrap(), before);
        assert!(f.pending_releases.contains(&lease));
        assert_eq!(f.release(lease, t()), Err(FlowError::InvalidLease));
        f.next_lease = 10;
        f.step().unwrap();
        assert!(!f.pending_releases.contains(&lease));
        assert_eq!(f.request(q).unwrap().state, RequestState::Released);
    }
    #[test]
    fn admission_counter_overflow_preserves_pending_request() {
        let mut f = FlowRuntime::new();
        let a = f.spawn_actor().unwrap();
        let r = f.create_resource(1).unwrap();
        let q = f.submit(r, a, t()).unwrap();
        f.next_admission = u64::MAX;
        assert_eq!(f.step().unwrap_err(), FlowError::CounterOverflow);
        assert_eq!(f.request(q).unwrap().state, RequestState::Pending);
        assert_eq!(f.resource(r).unwrap().available, 1);
    }
}

#[cfg(test)]
mod same_tick_budget_private {
    use super::*;
    fn time(n: u128) -> SimTime {
        SimTime::from_ticks(n)
    }
    fn duration(n: u128) -> SimDuration {
        SimDuration::from_ticks(n)
    }
    fn configured(limit: u64) -> FlowRuntime {
        FlowRuntime::with_config(FlowConfig {
            max_same_tick_flow_transitions: NonZeroU64::new(limit).unwrap(),
        })
    }
    fn pending_preserved(f: &mut FlowRuntime, expected: FlowError) {
        let head = f.scheduler.peek_next();
        let stats = f.scheduler.stats();
        let reservations = f.pending_releases.clone();
        let commands = f.commands.keys().copied().collect::<Vec<_>>();
        let world = f.world.snapshot();
        assert_eq!(f.step().unwrap_err(), expected);
        assert_eq!(f.scheduler.peek_next(), head);
        assert_eq!(f.scheduler.stats(), stats);
        assert_eq!(f.pending_releases, reservations);
        assert_eq!(f.commands.keys().copied().collect::<Vec<_>>(), commands);
        assert_eq!(f.world.snapshot(), world);
    }
    #[test]
    fn arithmetic_injections_preserve_real_head_and_all_admission_state() {
        for injection in 0..5 {
            let mut f = configured(100);
            let owner = f.spawn_actor().unwrap();
            let r = f.create_resource(1).unwrap();
            let w = f
                .create_work(owner, duration(2), "overflow", 42u32)
                .unwrap();
            let q = f
                .acquire(r)
                .owner(owner)
                .at(time(1))
                .timed_work(w)
                .submit()
                .unwrap();
            match injection {
                0 => f.next_admission = u64::MAX,
                1 => f.next_lease = u64::MAX,
                2 => f.scheduled = OPERATION_CAP,
                3 => {
                    f.registry
                        .store_mut::<WorkProgress>()
                        .unwrap()
                        .get_mut(w.0)
                        .unwrap()
                        .execution_revision = u64::MAX
                }
                4 => {
                    f.registry
                        .store_mut::<WorkProgress>()
                        .unwrap()
                        .get_mut(w.0)
                        .unwrap()
                        .remaining = duration(u128::MAX)
                }
                _ => unreachable!(),
            }
            let resource = f.resource(r).unwrap();
            let request = f.request(q).unwrap();
            let progress = f.registry.get::<WorkProgress>(w.0).unwrap().clone();
            pending_preserved(&mut f, FlowError::CounterOverflow);
            assert_eq!(f.resource(r).unwrap(), resource);
            assert_eq!(f.request(q).unwrap(), request);
            assert_eq!(f.registry.get::<WorkProgress>(w.0), Some(&progress));
            assert_eq!(f.work_context::<u32>(w).unwrap(), &42);
        }
    }
    #[test]
    fn cleanup_and_budget_add_overflow_preserve_head() {
        let mut f = configured(u64::MAX);
        let owner = f.spawn_actor().unwrap();
        let r = f.create_resource(1).unwrap();
        let w = f.create_work(owner, duration(2), "cleanup", 9u32).unwrap();
        let q = f.submit_work(r, owner, w, time(0)).unwrap();
        f.step().unwrap();
        f.despawn_actor(owner).unwrap();
        f.destroyed = OPERATION_CAP;
        pending_preserved(&mut f, FlowError::CounterOverflow);
        assert_eq!(f.work_context::<u32>(w).unwrap(), &9);
        assert_eq!(f.request(q).unwrap().state, RequestState::Active);
        f.destroyed = 0;
        f.budget_consumed = u64::MAX;
        pending_preserved(&mut f, FlowError::CounterOverflow);
        assert!(f.budget_halt.is_none());
    }
    #[test]
    fn zero_duration_admission_is_one_three_row_plan_or_retains_pending_command() {
        for limit in [2, 3] {
            let mut f = configured(limit);
            let owner = f.spawn_actor().unwrap();
            let r = f.create_resource(1).unwrap();
            let w = f.create_work(owner, duration(0), "zero", 42u32).unwrap();
            let q = f.acquire(r).owner(owner).timed_work(w).submit().unwrap();
            let resource = f.resource(r).unwrap();
            let progress = f.work_progress(w).unwrap();
            if limit == 2 {
                pending_preserved(
                    &mut f,
                    FlowError::SameTickBudgetExceeded { at_ticks: 0, limit },
                );
                assert_eq!(f.request(q).unwrap().state, RequestState::Pending);
                assert_eq!(f.resource(r).unwrap(), resource);
                assert_eq!(f.work_progress(w).unwrap(), progress);
                assert_eq!(f.work_context::<u32>(w).unwrap(), &42);
                let halt = f.budget_halt.unwrap();
                assert_eq!(halt.required_cost, 3);
                assert_eq!(
                    halt.pending.kind,
                    EventKind::custom(FLOW_COMMAND_DISPATCH_EVENT_KIND)
                );
            } else {
                let completed = f.step().unwrap().unwrap();
                assert_eq!(
                    completed
                        .records
                        .iter()
                        .map(|r| r.transition)
                        .collect::<Vec<_>>(),
                    vec![
                        LifecycleTransition::Queued,
                        LifecycleTransition::Granted,
                        LifecycleTransition::Completed
                    ]
                );
                assert_eq!(f.request(q).unwrap().state, RequestState::Completed);
                assert_eq!(f.budget_consumed, 3);
                // The first command is event0; its one original completion token is event1.
                let preview = f.scheduler.peek_next().unwrap();
                assert_eq!(preview.id, EventId::new(1, 1));
                assert_eq!(
                    preview.kind,
                    EventKind::custom(FLOW_TIMED_COMPLETION_EVENT_KIND)
                );
                assert_eq!(preview.at, time(0));
                let stale = f.step().unwrap().unwrap();
                assert_eq!(stale.event, preview.id);
                assert_eq!(stale.at, time(0));
                assert!(stale.records.is_empty());
                assert!(stale.error.is_none());
                assert_eq!(f.budget_consumed, 3);
                assert!(f.step().unwrap().is_none());
            }
        }
    }
    #[test]
    fn private_release_due_boundary_budget_is_atomic_in_both_dispositions() {
        for preloads in [1, 2] {
            let mut f = configured(3);
            let owner = f.spawn_actor().unwrap();
            let r = f.create_resource(1).unwrap();
            let dummy = f.create_resource(0).unwrap();
            let w = f
                .create_work(owner, duration(2), "due.release", ())
                .unwrap();
            let q = f.acquire(r).owner(owner).timed_work(w).submit().unwrap();
            f.step().unwrap();
            let lease = f.request(q).unwrap().lease.unwrap();
            let waiting = f.submit(r, owner, time(0)).unwrap();
            f.step().unwrap();
            // Private lower scheduler priority puts the invalid release before completion.
            f.schedule_priority(Command::Release(lease), time(2), -1)
                .unwrap();
            f.pending_releases.insert(lease);
            for _ in 0..preloads {
                f.acquire(dummy)
                    .owner(owner)
                    .at(time(2))
                    .scheduler_priority(-2)
                    .submit()
                    .unwrap();
            }
            for _ in 0..preloads {
                f.step().unwrap();
            }
            let before = f.resource(r).unwrap();
            if preloads == 2 {
                pending_preserved(
                    &mut f,
                    FlowError::SameTickBudgetExceeded {
                        at_ticks: 2,
                        limit: 3,
                    },
                );
                assert_eq!(f.resource(r).unwrap(), before);
                assert_eq!(f.request(waiting).unwrap().state, RequestState::Queued);
                assert_eq!(f.budget_halt.unwrap().required_cost, 2);
            } else {
                let d = f.step().unwrap().unwrap();
                assert_eq!(d.error, Some(FlowError::InvalidLease));
                assert_eq!(
                    d.records.iter().map(|r| r.transition).collect::<Vec<_>>(),
                    vec![LifecycleTransition::Completed, LifecycleTransition::Granted]
                );
                assert_eq!(f.request(waiting).unwrap().state, RequestState::Active);
                assert!(!f.pending_releases.contains(&lease));
            }
        }
    }
    fn factory(template: &std::rc::Rc<std::cell::Cell<u32>>) -> std::rc::Rc<std::cell::Cell<u32>> {
        template.set(template.get() + 1);
        template.clone()
    }
    #[test]
    fn restart_factory_is_not_called_for_budget_rejected_resume() {
        let mut f = configured(5);
        let owner = f.spawn_actor().unwrap();
        let r = f.create_resource(1).unwrap();
        let dummy = f.create_resource(0).unwrap();
        let calls = std::rc::Rc::new(std::cell::Cell::new(0));
        let low = f
            .create_restartable_work(owner, duration(10), "factory", calls.clone(), factory)
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
        let urgent = f.create_work(owner, duration(1), "urgent", ()).unwrap();
        f.acquire(r)
            .owner(owner)
            .at(time(1))
            .timed_work(urgent)
            .priority(1)
            .can_preempt(true)
            .submit()
            .unwrap();
        f.step().unwrap();
        assert_eq!(calls.get(), 1);
        // Four dummy rows at tick2 leave fewer than completion + restart grant's two rows.
        for _ in 0..4 {
            f.acquire(dummy)
                .owner(owner)
                .at(time(2))
                .scheduler_priority(-1)
                .submit()
                .unwrap();
        }
        for _ in 0..4 {
            f.step().unwrap();
        }
        pending_preserved(
            &mut f,
            FlowError::SameTickBudgetExceeded {
                at_ticks: 2,
                limit: 5,
            },
        );
        assert_eq!(calls.get(), 1);
        assert_eq!(f.request(q).unwrap().state, RequestState::Suspended);
    }
    #[test]
    fn missing_owner_context_or_handler_notification_is_zero_cost_at_limit() {
        for absent in 0..3 {
            let mut f = configured(2);
            let owner = f.spawn_actor().unwrap();
            let r = f.create_resource(1).unwrap();
            f.register_work_handlers(
                "handler",
                WorkHandlers {
                    on_cancel: Some(|c: &mut u32, _| *c += 1),
                    ..WorkHandlers::default()
                },
            )
            .unwrap();
            let w = f.create_work(owner, duration(10), "handler", 0u32).unwrap();
            let q = f.acquire(r).owner(owner).timed_work(w).submit().unwrap();
            f.step().unwrap();
            f.cancel(q, time(1)).unwrap();
            f.step().unwrap();
            match absent {
                0 => {
                    f.world.despawn(owner);
                }
                1 => {
                    f.registry.remove::<WorkContext<u32>>(w.0);
                }
                _ => {
                    f.handlers.remove("handler");
                }
            }
            f.budget_consumed = 2;
            let d = f.step().unwrap().unwrap();
            assert!(d.records.is_empty());
            assert_eq!(f.budget_consumed, 2);
            assert!(f.budget_halt.is_none());
            if absent != 1 {
                assert_eq!(f.registry.get::<WorkContext<u32>>(w.0).unwrap().0, 0);
            }
        }
    }
    #[test]
    fn cleanup_missing_typed_context_or_dead_work_is_rejected_before_consume() {
        for dead in [false, true] {
            let mut f = configured(100);
            let owner = f.spawn_actor().unwrap();
            let r = f.create_resource(1).unwrap();
            let w = f
                .create_work(owner, duration(2), "cleanup.validation", 42u32)
                .unwrap();
            let q = f.submit_work(r, owner, w, time(0)).unwrap();
            f.step().unwrap();
            f.despawn_actor(owner).unwrap();
            if dead {
                f.world.despawn(w.0);
            } else {
                f.registry.remove::<WorkContext<u32>>(w.0);
            }
            let resource = f.resource(r).unwrap();
            let progress = f.registry.get::<WorkProgress>(w.0).unwrap().clone();
            pending_preserved(&mut f, FlowError::InvalidState);
            assert_eq!(f.resource(r).unwrap(), resource);
            assert_eq!(f.request(q).unwrap().state, RequestState::Active);
            assert_eq!(f.registry.get::<WorkProgress>(w.0), Some(&progress));
        }
    }
}

#[cfg(test)]
mod continuation_private {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    const DOMAIN: EventKind = EventKind::custom(7100);
    fn time(n: u128) -> SimTime {
        SimTime::from_ticks(n)
    }
    fn duration(n: u128) -> SimDuration {
        SimDuration::from_ticks(n)
    }
    #[derive(Clone, Copy)]
    enum Mode {
        Empty,
        Pair(bool),
        Duplicate,
        Past,
        ReleasePair,
    }
    #[derive(Clone)]
    struct Probe {
        mode: Mode,
        owner: EntityId,
        resource: ResourceId,
        targets: [WorkId; 2],
        lease: Option<LeaseId>,
        calls: u32,
        generation: u32,
        tickets: Vec<FlowCommandTicket>,
        progress: Vec<WorkProgress>,
        origin: Option<EventId>,
        ordinal: Option<u32>,
    }
    fn acquire(probe: &Probe, target: usize, deadline: Option<SimTime>) -> FlowOwnedCommand {
        FlowOwnedCommand::Acquire(FlowAcquireCommand {
            resource: probe.resource,
            owner: probe.owner,
            work: Some(probe.targets[target]),
            at: time(1),
            priority_level: 0,
            deadline,
            scheduler_priority: 0,
            timed: true,
            can_preempt: false,
            preemptible: None,
        })
    }
    fn callback(probe: &mut Probe, snapshot: &FlowCallbackSnapshot, sink: &mut FlowCommandSink) {
        probe.calls += 1;
        probe.origin = Some(snapshot.origin);
        probe.ordinal = snapshot.origin_ordinal;
        if let FlowCallbackCause::Work { progress, .. } = &snapshot.cause {
            probe.progress.push(progress.clone());
        }
        match probe.mode {
            Mode::Empty => {}
            Mode::Pair(deadline) => {
                let first = acquire(probe, 0, None);
                let second = acquire(probe, 1, deadline.then(|| time(2)));
                probe.tickets.push(sink.emit(first).unwrap());
                probe.tickets.push(sink.emit(second).unwrap());
            }
            Mode::Duplicate => {
                for _ in 0..2 {
                    let command = acquire(probe, 0, None);
                    probe.tickets.push(sink.emit(command).unwrap());
                }
            }
            Mode::Past => {
                probe.tickets.push(
                    sink.emit(FlowOwnedCommand::Domain {
                        work: snapshot.work,
                        kind: DOMAIN,
                        at: time(0),
                        scheduler_priority: 0,
                    })
                    .unwrap(),
                );
            }
            Mode::ReleasePair => {
                for _ in 0..2 {
                    probe.tickets.push(
                        sink.emit(FlowOwnedCommand::Release {
                            lease: probe.lease.unwrap(),
                            at: time(2),
                        })
                        .unwrap(),
                    );
                }
            }
        }
    }
    fn setup(mode: Mode, limit: u64, complete: bool) -> (FlowRuntime, WorkId) {
        let mut f = FlowRuntime::with_config(FlowConfig {
            max_same_tick_flow_transitions: NonZeroU64::new(limit).unwrap(),
        });
        let owner = f.spawn_actor().unwrap();
        let resource = f.create_resource(1).unwrap();
        let targets = [
            f.create_work(owner, duration(2), "first", ()).unwrap(),
            f.create_work(owner, duration(3), "second", ()).unwrap(),
        ];
        f.register_domain_hook::<Probe>("probe", DOMAIN, callback)
            .unwrap();
        if complete {
            f.register_work_continuations(
                "probe",
                FlowContinuations {
                    on_complete: Some(callback),
                    ..FlowContinuations::default()
                },
            )
            .unwrap();
        }
        let probe = Probe {
            mode,
            owner,
            resource,
            targets,
            lease: None,
            calls: 0,
            generation: 0,
            tickets: vec![],
            progress: vec![],
            origin: None,
            ordinal: None,
        };
        let work = f.create_work(owner, duration(1), "probe", probe).unwrap();
        (f, work)
    }
    fn rejected(dispatch: &FlowDispatch, error: FlowError, ticket: Option<FlowCommandTicket>) {
        assert!(dispatch.error.is_none());
        assert_eq!(dispatch.callback_batches.len(), 1);
        match &dispatch.callback_batches[0] {
            FlowBatchReceipt::Rejected(r) => {
                assert_eq!(r.error, error);
                assert_eq!(r.failed_ticket, ticket);
            }
            FlowBatchReceipt::Accepted(_) => panic!("expected atomic rejection"),
        }
    }
    fn pending_unchanged(f: &mut FlowRuntime, error: FlowError) {
        let preview = f.scheduler.peek_next();
        let stats = f.scheduler.stats();
        let budget = f.budget_snapshot();
        let commands = f.commands.len();
        let notifications = f.notifications.len();
        let world = f.world.snapshot();
        assert_eq!(f.step().unwrap_err(), error);
        assert_eq!(f.scheduler.peek_next(), preview);
        assert_eq!(f.scheduler.stats(), stats);
        assert_eq!(f.budget_snapshot(), budget);
        assert_eq!(f.commands.len(), commands);
        assert_eq!(f.notifications.len(), notifications);
        assert_eq!(f.world.snapshot(), world);
    }
    #[test]
    fn batch_identity_overflow_retains_head_context_and_budget() {
        let (mut f, work) = setup(Mode::Empty, 10, false);
        f.schedule_domain(work, DOMAIN, time(1), 0).unwrap();
        f.next_batch_identity = u64::MAX;
        pending_unchanged(&mut f, FlowError::CounterOverflow);
        assert_eq!(f.work_context::<Probe>(work).unwrap().calls, 0);
        assert_eq!(f.next_batch_identity, u64::MAX);
    }
    #[test]
    fn second_acquire_entity_counter_overflow_rolls_back_every_reservation() {
        let (mut f, work) = setup(Mode::Pair(false), 10, false);
        let targets = f.work_context::<Probe>(work).unwrap().targets;
        f.schedule_domain(work, DOMAIN, time(1), 0).unwrap();
        let actual_created = f.created;
        f.created = OPERATION_CAP - 1;
        let world = f.world.snapshot();
        let requests = f.requests.clone();
        let reservations = f.pending_releases.clone();
        let scheduled = f.scheduled;
        let dispatch = f.step().unwrap().unwrap();
        let probe = f.work_context::<Probe>(work).unwrap();
        rejected(
            &dispatch,
            FlowError::CounterOverflow,
            Some(probe.tickets[1]),
        );
        assert_eq!(probe.calls, 1);
        assert_eq!(f.world.snapshot(), world);
        assert_eq!(f.requests, requests);
        assert_eq!(f.pending_releases, reservations);
        assert_eq!(f.created, OPERATION_CAP - 1);
        assert_eq!(f.scheduled, scheduled);
        assert_eq!(f.next_batch_identity, 1);
        for target in targets {
            assert_eq!(f.work(target).unwrap().request, None);
        }
        f.created = actual_created;
        let owner = f.work_context::<Probe>(work).unwrap().owner;
        let resource = f.work_context::<Probe>(work).unwrap().resource;
        let q = f.submit_work(resource, owner, targets[0], time(2)).unwrap();
        let (mut control, cw) = setup(Mode::Empty, 10, false);
        control.schedule_domain(cw, DOMAIN, time(1), 0).unwrap();
        control.step().unwrap();
        let c = control.work_context::<Probe>(cw).unwrap();
        let (r, o, target) = (c.resource, c.owner, c.targets[0]);
        let expected = control.submit_work(r, o, target, time(2)).unwrap();
        assert_eq!(q, expected);
        assert_eq!(
            f.scheduler.peek_next().unwrap().id,
            control.scheduler.peek_next().unwrap().id
        );
    }
    #[test]
    fn second_command_scheduler_and_deadline_counter_overflow_has_no_partial_ids() {
        let (mut f, work) = setup(Mode::Pair(true), 10, false);
        f.schedule_domain(work, DOMAIN, time(1), 0).unwrap();
        f.scheduled = OPERATION_CAP - 2;
        let world = f.world.snapshot();
        let created = f.created;
        let requests = f.requests.clone();
        let dispatch = f.step().unwrap().unwrap();
        let probe = f.work_context::<Probe>(work).unwrap();
        rejected(
            &dispatch,
            FlowError::CounterOverflow,
            Some(probe.tickets[1]),
        );
        assert_eq!(f.world.snapshot(), world);
        assert_eq!(f.created, created);
        assert_eq!(f.requests, requests);
        assert_eq!(f.scheduled, OPERATION_CAP - 2);
        // This is explicit pure-helper counter injection, not mutation of core-private Scheduler fields.
        let (f, work) = setup(Mode::Empty, 10, false);
        let probe = f.work_context::<Probe>(work).unwrap();
        let mut sink = FlowCommandSink::new(17, FlowCallbackConfig::default());
        sink.emit(acquire(probe, 0, None)).unwrap();
        let bad = sink.emit(acquire(probe, 1, Some(time(2)))).unwrap();
        let world = f.world.snapshot();
        let stats = f.scheduler.stats();
        let rejection = match f.plan_callback_batch(&sink, OPERATION_CAP - 2) {
            Err(r) => r,
            Ok(_) => panic!("pure counter boundary must reject"),
        };
        assert_eq!(rejection.error, FlowError::CounterOverflow);
        assert_eq!(rejection.failed_ticket, Some(bad));
        assert_eq!(f.world.snapshot(), world);
        assert_eq!(f.scheduler.stats(), stats);
        assert!(f.requests.is_empty());
    }
    #[test]
    fn batch_time_and_foreign_forward_non_acquire_ticket_rejection_is_atomic() {
        let (mut f, work) = setup(Mode::Past, 10, false);
        f.schedule_domain(work, DOMAIN, time(1), 0).unwrap();
        let world = f.world.snapshot();
        let scheduled = f.scheduled;
        let dispatch = f.step().unwrap().unwrap();
        let ticket = f.work_context::<Probe>(work).unwrap().tickets[0];
        rejected(&dispatch, FlowError::PastCommand, Some(ticket));
        assert_eq!(f.world.snapshot(), world);
        assert_eq!(f.scheduled, scheduled);
        for invalid in 0..3 {
            let (mut f, work) = setup(Mode::Empty, 10, false);
            f.schedule_domain(work, DOMAIN, time(1), 0).unwrap();
            let probe = f.work_context::<Probe>(work).unwrap();
            let mut sink = FlowCommandSink::new(9, FlowCallbackConfig::default());
            let bad = match invalid {
                0 => {
                    sink.emit(acquire(probe, 0, None)).unwrap();
                    sink.emit(FlowOwnedCommand::Cancel {
                        request: FlowRequestRef::Submitted(FlowCommandTicket {
                            batch: 8,
                            index: 0,
                        }),
                        at: time(1),
                        scheduler_priority: 0,
                    })
                    .unwrap()
                }
                1 => {
                    let bad = sink
                        .emit(FlowOwnedCommand::Cancel {
                            request: FlowRequestRef::Submitted(FlowCommandTicket {
                                batch: 9,
                                index: 1,
                            }),
                            at: time(1),
                            scheduler_priority: 0,
                        })
                        .unwrap();
                    sink.emit(acquire(probe, 0, None)).unwrap();
                    bad
                }
                _ => {
                    let first = sink
                        .emit(FlowOwnedCommand::Domain {
                            work,
                            kind: DOMAIN,
                            at: time(1),
                            scheduler_priority: 0,
                        })
                        .unwrap();
                    sink.emit(FlowOwnedCommand::Cancel {
                        request: FlowRequestRef::Submitted(first),
                        at: time(1),
                        scheduler_priority: 0,
                    })
                    .unwrap()
                }
            };
            let world = f.world.snapshot();
            let preview = f.scheduler.peek_next();
            let stats = f.scheduler.stats();
            let created = f.created;
            match f.admit_callback_batch(sink) {
                FlowBatchReceipt::Rejected(r) => {
                    assert_eq!(r.error, FlowError::InvalidCommandTicket);
                    assert_eq!(r.failed_ticket, Some(bad));
                }
                _ => panic!("invalid reference"),
            }
            assert_eq!(f.world.snapshot(), world);
            assert_eq!(f.scheduler.peek_next(), preview);
            assert_eq!(f.scheduler.stats(), stats);
            assert_eq!(f.created, created);
            assert!(f.requests.is_empty());
        }
    }
    #[test]
    fn second_release_duplicate_reservation_rejection_restores_pending_set() {
        let (mut f, work) = setup(Mode::ReleasePair, 10, false);
        let c = f.work_context::<Probe>(work).unwrap();
        let (owner, resource) = (c.owner, c.resource);
        let q = f.submit(resource, owner, time(0)).unwrap();
        f.step().unwrap();
        let lease = f.request(q).unwrap().lease.unwrap();
        f.registry
            .store_mut::<WorkContext<Probe>>()
            .unwrap()
            .get_mut(work.0)
            .unwrap()
            .0
            .lease = Some(lease);
        f.schedule_domain(work, DOMAIN, time(1), 0).unwrap();
        let reservations = f.pending_releases.clone();
        let scheduled = f.scheduled;
        let dispatch = f.step().unwrap().unwrap();
        let probe = f.work_context::<Probe>(work).unwrap();
        rejected(&dispatch, FlowError::InvalidLease, Some(probe.tickets[1]));
        assert_eq!(probe.calls, 1);
        assert_eq!(f.pending_releases, reservations);
        assert_eq!(f.scheduled, scheduled);
        assert_eq!(f.request(q).unwrap().lease, Some(lease));
        f.release(lease, time(2)).unwrap();
        assert!(f.pending_releases.contains(&lease));
    }
    fn old_count(context: &mut u32, _: &WorkProgress) {
        *context += 1;
    }
    fn new_count(context: &mut u32, _: &FlowCallbackSnapshot, _: &mut FlowCommandSink) {
        *context += 1;
    }
    fn rc_count(context: &mut Rc<Cell<u32>>, _: &FlowCallbackSnapshot, _: &mut FlowCommandSink) {
        context.set(context.get() + 1);
    }
    fn old_rc_count(context: &mut Rc<Cell<u32>>, _: &WorkProgress) {
        context.set(context.get() + 1);
    }
    fn rc_factory(template: &Rc<Cell<u32>>) -> Rc<Cell<u32>> {
        template.set(template.get() + 1);
        template.clone()
    }
    #[test]
    fn legacy_and_continuation_tokens_aggregate_overflow_before_factory_or_cleanup() {
        for cleanup in [false, true] {
            let mut f = FlowRuntime::new();
            let owner = f.spawn_actor().unwrap();
            let r = f.create_resource(1).unwrap();
            f.register_work_handlers(
                "both",
                WorkHandlers {
                    on_cancel: Some(old_count),
                    ..WorkHandlers::default()
                },
            )
            .unwrap();
            f.register_work_continuations(
                "both",
                FlowContinuations {
                    on_cancel: Some(new_count),
                    ..FlowContinuations::default()
                },
            )
            .unwrap();
            let work = f.create_work(owner, duration(10), "both", 0u32).unwrap();
            let q = f.acquire(r).owner(owner).timed_work(work).submit().unwrap();
            f.step().unwrap();
            if cleanup {
                f.despawn_actor(owner).unwrap();
            } else {
                f.cancel(q, time(1)).unwrap();
            }
            f.scheduled = OPERATION_CAP - 1;
            let world = f.world.snapshot();
            let progress = f.work_progress(work).unwrap();
            pending_unchanged(&mut f, FlowError::CounterOverflow);
            assert_eq!(f.world.snapshot(), world);
            assert_eq!(f.work_progress(work).unwrap(), progress);
            assert_eq!(*f.work_context::<u32>(work).unwrap(), 0);
            assert_eq!(f.request(q).unwrap().state, RequestState::Active);
        }
        let mut f = FlowRuntime::new();
        let owner = f.spawn_actor().unwrap();
        let r = f.create_resource(1).unwrap();
        let calls = Rc::new(Cell::new(0));
        f.register_work_handlers(
            "restart",
            WorkHandlers {
                on_restart: Some(old_rc_count),
                ..WorkHandlers::default()
            },
        )
        .unwrap();
        f.register_work_continuations(
            "restart",
            FlowContinuations {
                on_restart: Some(rc_count),
                ..FlowContinuations::default()
            },
        )
        .unwrap();
        let low = f
            .create_restartable_work(owner, duration(5), "restart", calls.clone(), rc_factory)
            .unwrap();
        let low_q = f
            .acquire(r)
            .owner(owner)
            .timed_work(low)
            .priority(9)
            .preemptible(PreemptionStrategy::Restart)
            .submit()
            .unwrap();
        f.step().unwrap();
        let urgent = f.create_work(owner, duration(1), "urgent", ()).unwrap();
        f.acquire(r)
            .owner(owner)
            .at(time(1))
            .timed_work(urgent)
            .priority(1)
            .can_preempt(true)
            .submit()
            .unwrap();
        f.step().unwrap();
        assert_eq!(calls.get(), 1);
        f.scheduled = OPERATION_CAP - 2;
        pending_unchanged(&mut f, FlowError::CounterOverflow);
        assert_eq!(calls.get(), 1);
        assert_eq!(f.request(low_q).unwrap().state, RequestState::Suspended);
    }
    #[test]
    fn cap_poison_overrides_later_validation_and_never_grows_vector() {
        let (mut f, work) = setup(Mode::Empty, 10, false);
        let stats = f.scheduler.stats();
        let world = f.world.snapshot();
        let mut sink = FlowCommandSink::new(
            0,
            FlowCallbackConfig {
                max_callback_commands: NonZeroUsize::new(2).unwrap(),
            },
        );
        sink.emit(FlowOwnedCommand::Domain {
            work,
            kind: DOMAIN,
            at: time(1),
            scheduler_priority: 0,
        })
        .unwrap();
        sink.emit(FlowOwnedCommand::Domain {
            work,
            kind: EventKind::custom(4000),
            at: time(1),
            scheduler_priority: 0,
        })
        .unwrap();
        for _ in 0..3 {
            assert_eq!(
                sink.emit(FlowOwnedCommand::Domain {
                    work,
                    kind: DOMAIN,
                    at: time(1),
                    scheduler_priority: 0
                })
                .unwrap_err(),
                FlowError::CallbackBatchLimitExceeded
            );
        }
        assert_eq!(sink.commands.len(), 2);
        assert_eq!(sink.next_index, 2);
        match f.admit_callback_batch(sink) {
            FlowBatchReceipt::Rejected(r) => {
                assert_eq!(r.error, FlowError::CallbackBatchLimitExceeded);
                assert_eq!(r.failed_ticket, None);
            }
            _ => panic!("poison ignored"),
        }
        assert_eq!(f.scheduler.stats(), stats);
        assert_eq!(f.world.snapshot(), world);
        let mut sink = FlowCommandSink::new(0, FlowCallbackConfig::default());
        sink.next_index = usize::MAX;
        for _ in 0..2 {
            assert_eq!(
                sink.emit(FlowOwnedCommand::Domain {
                    work,
                    kind: DOMAIN,
                    at: time(1),
                    scheduler_priority: 0
                })
                .unwrap_err(),
                FlowError::CounterOverflow
            );
        }
        assert!(sink.commands.is_empty());
        assert_eq!(sink.poison, Some(FlowError::CounterOverflow));
    }
    #[test]
    fn stale_context_or_work_delivery_zero_cost_and_unregistered_domain_defensive_error() {
        for absent in 0..5 {
            let (mut f, work) = setup(Mode::Empty, 1, false);
            f.schedule_domain(work, DOMAIN, time(1), 0).unwrap();
            f.schedule_domain(work, DOMAIN, time(1), 0).unwrap();
            f.step().unwrap();
            assert_eq!(f.budget_consumed, 1);
            let owner = f.work(work).unwrap().owner;
            match absent {
                0 => {
                    f.registry.remove::<WorkContext<Probe>>(work.0);
                }
                1 => {
                    f.world.despawn(work.0);
                }
                2 => {
                    f.works.remove(&work);
                }
                3 => {
                    f.world.despawn(owner);
                }
                _ => {
                    f.domain_hooks.remove(&("probe".to_owned(), DOMAIN));
                }
            }
            let d = f.step().unwrap().unwrap();
            assert!(d.records.is_empty());
            assert!(d.callback_batches.is_empty());
            assert_eq!(f.budget_consumed, 1);
            assert!(f.budget_halt.is_none());
            assert_eq!(
                d.error,
                if absent == 4 {
                    Some(FlowError::UnregisteredDomainEvent)
                } else {
                    None
                }
            );
            if absent != 0 {
                assert_eq!(
                    f.registry
                        .get::<WorkContext<Probe>>(work.0)
                        .unwrap()
                        .0
                        .calls,
                    1
                );
            }
        }
        let mut f = FlowRuntime::with_config(FlowConfig {
            max_same_tick_flow_transitions: NonZeroU64::new(2).unwrap(),
        });
        let owner = f.spawn_actor().unwrap();
        let r = f.create_resource(1).unwrap();
        let calls: Rc<Cell<u32>> = Rc::new(Cell::new(0));
        f.register_domain_hook("stale", DOMAIN, rc_count).unwrap();
        f.register_work_continuations(
            "stale",
            FlowContinuations {
                on_cancel: Some(rc_count),
                ..FlowContinuations::default()
            },
        )
        .unwrap();
        let work = f
            .create_work(owner, duration(10), "stale", calls.clone())
            .unwrap();
        let q = f.acquire(r).owner(owner).timed_work(work).submit().unwrap();
        f.step().unwrap();
        f.schedule_domain(work, DOMAIN, time(1), -1).unwrap();
        f.cancel(q, time(1)).unwrap();
        f.step().unwrap();
        f.step().unwrap();
        assert_eq!(f.budget_consumed, 2);
        assert_eq!(calls.get(), 1);
        f.registry.remove::<WorkContext<Rc<Cell<u32>>>>(work.0);
        let d = f.step().unwrap().unwrap();
        assert!(d.records.is_empty());
        assert!(d.callback_batches.is_empty());
        assert_eq!(f.budget_consumed, 2);
        assert_eq!(calls.get(), 1);
    }
    fn probe_factory(template: &(Rc<Cell<u32>>, Probe)) -> Probe {
        template.0.set(template.0.get() + 1);
        let mut probe = template.1.clone();
        probe.generation = template.0.get();
        probe
    }
    #[test]
    fn completed_context_snapshot_and_restart_factory_nonrollback_delivery() {
        let (mut f, work) = setup(Mode::Duplicate, 100, true);
        let c = f.work_context::<Probe>(work).unwrap();
        let (owner, r) = (c.owner, c.resource);
        f.acquire(r).owner(owner).timed_work(work).submit().unwrap();
        f.step().unwrap();
        let complete = f.step().unwrap().unwrap();
        let notify = f.step().unwrap().unwrap();
        let c = f.work_context::<Probe>(work).unwrap();
        rejected(&notify, FlowError::InvalidWork, Some(c.tickets[1]));
        assert_eq!(c.calls, 1);
        assert_eq!(c.origin, Some(complete.event));
        assert_eq!(c.ordinal, Some(0));
        assert_eq!(c.progress[0].state, WorkState::Completed);
        assert_eq!(c.progress[0].cumulative_busy, duration(1));
        assert!(f.step().unwrap().is_none());
        let mut f = FlowRuntime::new();
        let owner = f.spawn_actor().unwrap();
        let r = f.create_resource(1).unwrap();
        let targets = [
            f.create_work(owner, duration(2), "target-a", ()).unwrap(),
            f.create_work(owner, duration(2), "target-b", ()).unwrap(),
        ];
        f.register_work_continuations(
            "restart-probe",
            FlowContinuations {
                on_restart: Some(callback),
                ..FlowContinuations::default()
            },
        )
        .unwrap();
        let counter = Rc::new(Cell::new(0));
        let seed = Probe {
            mode: Mode::Duplicate,
            owner,
            resource: r,
            targets,
            lease: None,
            calls: 0,
            generation: 0,
            tickets: vec![],
            progress: vec![],
            origin: None,
            ordinal: None,
        };
        let work = f
            .create_restartable_work(
                owner,
                duration(5),
                "restart-probe",
                (counter.clone(), seed),
                probe_factory,
            )
            .unwrap();
        let q = f
            .acquire(r)
            .owner(owner)
            .timed_work(work)
            .priority(9)
            .preemptible(PreemptionStrategy::Restart)
            .submit()
            .unwrap();
        f.step().unwrap();
        let urgent = f.create_work(owner, duration(1), "urgent", ()).unwrap();
        f.acquire(r)
            .owner(owner)
            .at(time(1))
            .timed_work(urgent)
            .priority(1)
            .can_preempt(true)
            .submit()
            .unwrap();
        f.step().unwrap();
        let restart = f.step().unwrap().unwrap();
        assert_eq!(restart.at, time(2));
        let notify = f.step().unwrap().unwrap();
        let c = f.work_context::<Probe>(work).unwrap();
        rejected(&notify, FlowError::PastCommand, Some(c.tickets[0]));
        assert_eq!(counter.get(), 2);
        assert_eq!(c.generation, 2);
        assert_eq!(c.calls, 1);
        assert_eq!(c.origin, Some(restart.event));
        assert_eq!(c.ordinal, Some(1));
        assert_eq!(c.progress[0].state, WorkState::Active);
        assert_eq!(c.progress[0].cumulative_busy, duration(1));
        assert_eq!(c.progress[0].remaining, duration(5));
        let stale = f.step().unwrap().unwrap();
        assert_eq!(stale.at, time(5));
        assert!(stale.records.is_empty());
        assert_eq!(f.work_context::<Probe>(work).unwrap().calls, 1);
        let done = f.step().unwrap().unwrap();
        assert_eq!(done.at, time(7));
        assert_eq!(f.request(q).unwrap().state, RequestState::Completed);
        assert_eq!(counter.get(), 2);
    }
    #[test]
    fn committed_batch_receipt_actual_event_and_deadline_order() {
        let (mut f, work) = setup(Mode::Pair(true), 100, false);
        f.schedule_domain(work, DOMAIN, time(1), 0).unwrap();
        let delivery = f.step().unwrap().unwrap();
        let (first, second, deadline, qa, qb) = match &delivery.callback_batches[0] {
            FlowBatchReceipt::Accepted(v) => {
                assert_eq!(v.len(), 2);
                assert_eq!(v[0].event, EventId::new(1, 1));
                assert_eq!(v[1].event, EventId::new(2, 2));
                assert_eq!(v[0].deadline_event, None);
                assert_eq!(v[1].deadline_event, Some(EventId::new(3, 3)));
                (
                    v[0].event,
                    v[1].event,
                    v[1].deadline_event.unwrap(),
                    v[0].request.unwrap(),
                    v[1].request.unwrap(),
                )
            }
            _ => panic!("valid planned pair"),
        };
        assert_eq!(f.request(qa).unwrap().state, RequestState::Pending);
        assert_eq!(f.request(qb).unwrap().state, RequestState::Pending);
        let d = f.step().unwrap().unwrap();
        assert_eq!(d.event, first);
        assert_eq!(
            d.records.iter().map(|r| r.transition).collect::<Vec<_>>(),
            vec![LifecycleTransition::Queued, LifecycleTransition::Granted]
        );
        let d = f.step().unwrap().unwrap();
        assert_eq!(d.event, second);
        assert_eq!(
            d.records.iter().map(|r| r.transition).collect::<Vec<_>>(),
            vec![LifecycleTransition::Queued]
        );
        let d = f.step().unwrap().unwrap();
        assert_eq!(d.event, deadline);
        assert_eq!(d.at, time(2));
        assert_eq!(
            d.records.iter().map(|r| r.transition).collect::<Vec<_>>(),
            vec![LifecycleTransition::TimedOut]
        );
        let d = f.step().unwrap().unwrap();
        assert_eq!(d.at, time(3));
        assert_eq!(f.request(qa).unwrap().state, RequestState::Completed);
        assert_eq!(f.request(qb).unwrap().state, RequestState::TimedOut);
        assert!(f.step().unwrap().is_none());
    }
}

#[cfg(test)]
mod domain_view_private_tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};
    const KIND: EventKind = EventKind::custom(7405);
    fn callback<'a>(
        c: &'a mut Rc<Cell<u32>>,
        s: &'a FlowCallbackSnapshot,
        v: FlowWorldView<'a>,
        _: &'a mut FlowCommandSink,
    ) {
        assert_eq!(v.now(), s.delivery.at);
        c.set(c.get() + 1);
    }
    fn setup() -> (FlowRuntime, WorkId, Rc<Cell<u32>>) {
        let mut f = FlowRuntime::new();
        f.register_domain_view_hook("view", KIND, callback).unwrap();
        let actor = f.spawn_actor().unwrap();
        let calls: Rc<Cell<u32>> = Rc::new(Cell::new(0));
        let work = f
            .create_work(actor, SimDuration::ZERO, "view", calls.clone())
            .unwrap();
        f.schedule_domain(work, KIND, SimTime::from_ticks(3), 0)
            .unwrap();
        (f, work, calls)
    }
    macro_rules! capture {
        ($f:expr) => {
            (
                $f.scheduler.peek_next(),
                $f.scheduler.stats(),
                $f.budget_snapshot(),
                $f.world.snapshot(),
                $f.created,
                $f.destroyed,
                $f.next_batch_identity,
                $f.commands.keys().copied().collect::<Vec<_>>(),
                $f.actors.clone(),
                $f.works.len(),
                $f.notifications.len(),
            )
        };
    }
    fn missing_context(wrong: bool) {
        let (mut f, work, calls) = setup();
        f.registry
            .remove::<WorkContext<Rc<Cell<u32>>>>(work.0)
            .unwrap();
        if wrong {
            assert!(f.registry.insert(work.0, WorkContext(17u32)));
        }
        let before = capture!(f);
        let spec = f.work(work).unwrap();
        let progress = f.work_progress(work).unwrap();
        assert!(f.domain_delivery(work, KIND).unwrap().is_none());
        assert_eq!(capture!(f), before);
        assert_eq!(f.work(work).unwrap(), spec);
        assert_eq!(f.work_progress(work).unwrap(), progress);
        // Actual stale dispatch is consumed, preserving the existing semantics.
        let d = f.step().unwrap().unwrap();
        assert_eq!(d.at, SimTime::from_ticks(3));
        assert!(d.error.is_none() && d.records.is_empty() && d.callback_batches.is_empty());
        assert_eq!(calls.get(), 0);
        assert_eq!(f.next_batch_identity, 0);
        assert_eq!(f.budget_consumed, 0);
        assert_eq!(f.world.snapshot(), before.3);
        assert_eq!(f.created, before.4);
        assert_eq!(f.destroyed, before.5);
        assert_eq!(
            f.scheduler.stats().dispatched_events,
            before.1.dispatched_events + 1
        );
        assert_eq!(f.work(work).unwrap(), spec);
        assert_eq!(f.work_progress(work).unwrap(), progress);
        if wrong {
            assert_eq!(f.registry.get::<WorkContext<u32>>(work.0).unwrap().0, 17);
        }
        assert!(f.step().unwrap().is_none());
    }
    #[test]
    fn view_missing_context_inspection_is_nonmutating_then_consumed_stale() {
        missing_context(false);
    }
    #[test]
    fn view_wrong_context_inspection_is_nonmutating_then_consumed_stale() {
        missing_context(true);
    }
    #[test]
    fn view_missing_descriptor_preserves_consumed_semantic_error() {
        let (mut f, work, calls) = setup();
        f.domain_hooks.remove(&("view".to_owned(), KIND));
        let before = capture!(f);
        assert!(matches!(
            f.domain_delivery(work, KIND),
            Err(FlowError::UnregisteredDomainEvent)
        ));
        assert_eq!(capture!(f), before);
        let d = f.step().unwrap().unwrap();
        assert_eq!(d.error, Some(FlowError::UnregisteredDomainEvent));
        assert!(d.records.is_empty() && d.callback_batches.is_empty());
        assert_eq!(calls.get(), 0);
        assert_eq!(f.next_batch_identity, 0);
        assert_eq!(f.budget_consumed, 0);
        assert_eq!(f.world.snapshot(), before.3);
        assert!(f.step().unwrap().is_none());
    }
    #[test]
    fn view_batch_identity_overflow_retains_exact_head_and_live_context() {
        let (mut f, work, calls) = setup();
        f.next_batch_identity = u64::MAX;
        let before = capture!(f);
        assert_eq!(f.step().unwrap_err(), FlowError::CounterOverflow);
        assert_eq!(capture!(f), before);
        assert_eq!(f.work_context::<Rc<Cell<u32>>>(work).unwrap().get(), 0);
        assert_eq!(calls.get(), 0);
    }
    #[test]
    fn view_budget_arithmetic_overflow_retains_exact_head_and_live_context() {
        let (mut f, work, calls) = setup();
        // Private arithmetic injection, not a reachable public event-count claim.
        f.budget_tick = Some(SimTime::from_ticks(3));
        f.budget_consumed = u64::MAX;
        let before = capture!(f);
        assert_eq!(f.step().unwrap_err(), FlowError::CounterOverflow);
        assert_eq!(capture!(f), before);
        assert_eq!(f.work_context::<Rc<Cell<u32>>>(work).unwrap().get(), 0);
        assert_eq!(calls.get(), 0);
    }
}

#[cfg(test)]
mod actor_domain_private_tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};
    const KIND: EventKind = EventKind::custom(7510);
    fn callback<'a>(
        c: &'a mut Rc<Cell<u32>>,
        _: &'a FlowCallbackSnapshot,
        _: FlowWorldView<'a>,
        _: &'a mut FlowCommandSink,
    ) {
        c.set(c.get() + 1);
    }
    fn setup() -> (FlowRuntime, EntityId, WorkId, Rc<Cell<u32>>) {
        let mut f = FlowRuntime::new();
        f.register_domain_view_hook("carrier", KIND, callback)
            .unwrap();
        let actor = f.spawn_actor().unwrap();
        let calls: Rc<Cell<u32>> = Rc::new(Cell::new(0));
        let work = f
            .create_actor_domain_context(actor, "carrier", KIND, calls.clone())
            .unwrap();
        (f, actor, work, calls)
    }
    macro_rules! capture {
        ($f:expr) => {
            (
                (
                    $f.scheduler.peek_next(),
                    $f.scheduler.stats(),
                    $f.budget_snapshot(),
                    $f.world.snapshot(),
                ),
                (
                    $f.created,
                    $f.destroyed,
                    $f.scheduled,
                    $f.next_admission,
                    $f.next_lease,
                    $f.next_batch_identity,
                ),
                (
                    $f.commands.keys().copied().collect::<Vec<_>>(),
                    $f.actors.clone(),
                    $f.works.len(),
                    $f.actor_domains.clone(),
                    $f.notifications.len(),
                    $f.pending_releases.clone(),
                ),
            )
        };
    }
    #[test]
    fn carrier_creation_counter_overflow_has_no_partial_entity_or_index() {
        let (mut f, _, _, calls) = setup();
        let actor = f.spawn_actor().unwrap();
        f.created = OPERATION_CAP;
        let before = capture!(f);
        assert_eq!(
            f.create_actor_domain_context(actor, "carrier", KIND, calls.clone()),
            Err(FlowError::CounterOverflow)
        );
        assert_eq!(capture!(f), before);
        assert!(!f.actor_domains.contains_key(&actor));
        assert_eq!(calls.get(), 0);
    }
    #[test]
    fn carrier_creation_orphan_and_task_index_reject_atomically() {
        for orphan in [false, true] {
            let (mut f, actor, work, calls) = setup();
            if orphan {
                f.actor_domains.remove(&actor);
            } else {
                let _ = f.registry.insert(work.0, WorkRole::Task);
            }
            let before = capture!(f);
            assert_eq!(
                f.create_actor_domain_context(actor, "carrier", KIND, calls.clone()),
                Err(FlowError::InvalidWork)
            );
            assert_eq!(capture!(f), before);
            assert_eq!(calls.get(), 0);
        }
    }
    #[test]
    fn carrier_corrupt_role_index_and_metadata_retain_domain_head() {
        for fault in 0..7 {
            let (mut f, actor, work, calls) = setup();
            f.schedule_domain(work, KIND, SimTime::from_ticks(3), 0)
                .unwrap();
            match fault {
                0 => {
                    f.registry.remove::<WorkRole>(work.0);
                }
                1 => {
                    f.actor_domains.remove(&actor);
                }
                2 => {
                    let other = f.spawn_actor().unwrap();
                    f.actor_domains.insert(other, work);
                }
                3 => {
                    f.registry
                        .store_mut::<WorkSpec>()
                        .unwrap()
                        .get_mut(work.0)
                        .unwrap()
                        .original_duration = SimDuration::from_ticks(1);
                }
                4 => {
                    f.registry
                        .store_mut::<WorkProgress>()
                        .unwrap()
                        .get_mut(work.0)
                        .unwrap()
                        .state = WorkState::Completed;
                }
                5 => {
                    f.domain_hooks.remove(&("carrier".into(), KIND));
                }
                _ => {
                    f.context_types
                        .insert("carrier".into(), TypeId::of::<u32>());
                }
            }
            let before = capture!(f);
            assert_eq!(
                f.step().unwrap_err(),
                FlowError::InvalidState,
                "fault {fault}"
            );
            assert_eq!(capture!(f), before);
            assert_eq!(calls.get(), 0);
        }
    }
    #[test]
    fn carrier_missing_context_is_consumed_stale_not_structural_error() {
        let (mut f, _, work, calls) = setup();
        f.schedule_domain(work, KIND, SimTime::from_ticks(3), 0)
            .unwrap();
        f.registry.remove::<WorkContext<Rc<Cell<u32>>>>(work.0);
        let d = f.step().unwrap().unwrap();
        assert!(d.error.is_none() && d.records.is_empty() && d.callback_batches.is_empty());
        assert_eq!(calls.get(), 0);
        assert_eq!(f.next_batch_identity, 0);
        assert!(f.step().unwrap().is_none());
    }
    #[test]
    fn carrier_batch_counter_overflow_reserves_nothing() {
        let (mut f, _, work, calls) = setup();
        let mut sink = FlowCommandSink::new(7, FlowCallbackConfig::default());
        sink.emit(FlowOwnedCommand::Domain {
            work,
            kind: KIND,
            at: SimTime::from_ticks(4),
            scheduler_priority: 0,
        })
        .unwrap();
        f.scheduled = OPERATION_CAP;
        let before = capture!(f);
        assert_eq!(
            f.plan_callback_batch(&sink, f.scheduler.stats().scheduled_events)
                .err()
                .unwrap()
                .error,
            FlowError::CounterOverflow
        );
        assert_eq!(capture!(f), before);
        assert_eq!(calls.get(), 0);
    }
    #[test]
    fn carrier_cleanup_corruption_retains_despawn_head_and_context() {
        for fault in 0..5 {
            let (mut f, actor, work, calls) = setup();
            f.despawn_actor(actor).unwrap();
            match fault {
                0 => {
                    f.registry.remove::<WorkRole>(work.0);
                }
                1 => {
                    f.actor_domains.remove(&actor);
                }
                2 => {
                    f.registry.remove::<WorkSpec>(work.0);
                }
                3 => {
                    f.registry.remove::<WorkContext<Rc<Cell<u32>>>>(work.0);
                }
                _ => {
                    let other = f.spawn_actor().unwrap();
                    let task = f
                        .create_work(other, SimDuration::ZERO, "carrier", calls.clone())
                        .unwrap();
                    f.actor_domains.insert(actor, task);
                }
            }
            let before = capture!(f);
            assert_eq!(
                f.step().unwrap_err(),
                FlowError::InvalidState,
                "fault {fault}"
            );
            assert_eq!(capture!(f), before);
            assert!(f.world.is_alive(actor) && f.world.is_alive(work.0));
            assert_eq!(calls.get(), 0);
        }
    }
    #[test]
    fn carrier_aggregate_cleanup_overflow_retains_exact_head() {
        let (mut f, actor, work, calls) = setup();
        let task = f
            .create_work(actor, SimDuration::ZERO, "carrier", calls.clone())
            .unwrap();
        f.despawn_actor(actor).unwrap();
        f.destroyed = OPERATION_CAP - 2;
        let before = capture!(f);
        assert_eq!(f.step().unwrap_err(), FlowError::CounterOverflow);
        assert_eq!(capture!(f), before);
        assert!(f.world.is_alive(actor) && f.world.is_alive(work.0) && f.world.is_alive(task.0));
        assert_eq!(calls.get(), 0);
    }
    #[test]
    fn carrier_cleanup_counts_actor_and_each_owned_work_once() {
        let (mut f, actor, work, calls) = setup();
        let task = f
            .create_work(actor, SimDuration::ZERO, "carrier", calls.clone())
            .unwrap();
        f.despawn_actor(actor).unwrap();
        let before = f.destroyed;
        f.step().unwrap().unwrap();
        assert_eq!(f.destroyed, before + 3);
        assert!(!f.actor_domains.contains_key(&actor));
        for id in [actor, work.0, task.0] {
            assert!(!f.world.is_alive(id));
        }
        assert_eq!(calls.get(), 0);
    }

    #[test]
    fn carrier_missing_spec_with_either_ownership_witness_retains_exact_head() {
        for witness in 0..3 {
            let (mut f, actor, work, calls) = setup();
            f.schedule_domain(work, KIND, SimTime::from_ticks(3), 0)
                .unwrap();
            f.registry.remove::<WorkSpec>(work.0);
            if witness == 1 {
                f.actor_domains.remove(&actor);
            }
            if witness == 2 {
                f.registry.remove::<WorkRole>(work.0);
            }
            let before = capture!(f);
            assert_eq!(f.step().unwrap_err(), FlowError::InvalidState);
            assert_eq!(capture!(f), before);
            assert_eq!(calls.get(), 0);
            assert_eq!(
                f.registry
                    .get::<WorkContext<Rc<Cell<u32>>>>(work.0)
                    .unwrap()
                    .0
                    .get(),
                0
            );
        }
    }
    #[test]
    fn task_missing_spec_remains_consumed_stale() {
        let (mut f, actor, _, calls) = setup();
        let task = f
            .create_work(actor, SimDuration::ZERO, "carrier", calls.clone())
            .unwrap();
        f.schedule_domain(task, KIND, SimTime::from_ticks(3), 0)
            .unwrap();
        f.registry.remove::<WorkSpec>(task.0);
        let d = f.step().unwrap().unwrap();
        assert!(d.error.is_none() && d.records.is_empty() && d.callback_batches.is_empty());
        assert_eq!(calls.get(), 0);
        assert!(f.step().unwrap().is_none());
    }
    #[test]
    fn carrier_active_resource_cleanup_overflow_retains_allocation_queue_and_progress() {
        let (mut f, actor, carrier, calls) = setup();
        let resource = f.create_resource(1).unwrap();
        let task = f
            .create_work(actor, SimDuration::from_ticks(10), "carrier", calls.clone())
            .unwrap();
        let active = f
            .acquire(resource)
            .owner(actor)
            .timed_work(task)
            .submit()
            .unwrap();
        f.step().unwrap().unwrap();
        assert_eq!(f.request(active).unwrap().state, RequestState::Active);
        let waiter = f.spawn_actor().unwrap();
        let queued = f.submit(resource, waiter, SimTime::ZERO).unwrap();
        f.step().unwrap().unwrap();
        assert_eq!(f.request(queued).unwrap().state, RequestState::Queued);
        let lease = f.request(active).unwrap().lease.unwrap();
        f.despawn_actor(actor).unwrap();
        f.destroyed = OPERATION_CAP - 2;
        let before = capture!(f);
        let capacity = f.resource(resource).unwrap();
        let active_before = f.request(active).unwrap().clone();
        let queued_before = f.request(queued).unwrap().clone();
        let task_spec = f.work(task).unwrap();
        let task_progress = f.work_progress(task).unwrap();
        let carrier_progress = f.work_progress(carrier).unwrap();
        assert_eq!(f.step().unwrap_err(), FlowError::CounterOverflow);
        assert_eq!(capture!(f), before);
        assert_eq!(f.resource(resource).unwrap(), capacity);
        assert_eq!(f.request(active).unwrap(), active_before);
        assert_eq!(f.request(queued).unwrap(), queued_before);
        assert_eq!(f.request(active).unwrap().lease, Some(lease));
        assert_eq!(f.work(task).unwrap(), task_spec);
        assert_eq!(f.work_progress(task).unwrap(), task_progress);
        assert_eq!(f.work_progress(carrier).unwrap(), carrier_progress);
        assert_eq!(calls.get(), 0);
        assert!(f.world.is_alive(actor) && f.world.is_alive(task.0) && f.world.is_alive(carrier.0));
    }
}

#[cfg(test)]
mod buffered_despawn_private_tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};
    const KIND: EventKind = EventKind::custom(7600);
    fn t(n: u128) -> SimTime {
        SimTime::from_ticks(n)
    }
    fn view<'a>(
        c: &'a mut Rc<Cell<u32>>,
        _: &'a FlowCallbackSnapshot,
        _: FlowWorldView<'a>,
        _: &'a mut FlowCommandSink,
    ) {
        c.set(c.get() + 1);
    }
    fn legacy(c: &mut Rc<Cell<u32>>, _: &WorkProgress) {
        c.set(c.get() + 1);
    }
    fn continuation(c: &mut Rc<Cell<u32>>, _: &FlowCallbackSnapshot, _: &mut FlowCommandSink) {
        c.set(c.get() + 1);
    }
    fn setup(limit: u64) -> (FlowRuntime, EntityId, WorkId, Rc<Cell<u32>>) {
        let mut f = FlowRuntime::with_config(FlowConfig {
            max_same_tick_flow_transitions: NonZeroU64::new(limit).unwrap(),
        });
        f.register_domain_view_hook("shared", KIND, view).unwrap();
        f.register_work_handlers(
            "shared",
            WorkHandlers {
                on_cancel: Some(legacy),
                ..Default::default()
            },
        )
        .unwrap();
        f.register_work_continuations(
            "shared",
            FlowContinuations {
                on_cancel: Some(continuation),
                ..Default::default()
            },
        )
        .unwrap();
        let actor = f.spawn_actor().unwrap();
        let calls: Rc<Cell<u32>> = Rc::new(Cell::new(0));
        let carrier = f
            .create_actor_domain_context(actor, "shared", KIND, calls.clone())
            .unwrap();
        (f, actor, carrier, calls)
    }
    macro_rules! capture {
        ($f:expr) => {
            (
                (
                    $f.scheduler.peek_next(),
                    $f.scheduler.stats(),
                    $f.budget_snapshot(),
                    $f.world.snapshot(),
                ),
                (
                    $f.created,
                    $f.destroyed,
                    $f.scheduled,
                    $f.next_admission,
                    $f.next_lease,
                    $f.next_batch_identity,
                ),
                (
                    $f.commands.keys().copied().collect::<Vec<_>>(),
                    $f.actor_domains.clone(),
                    $f.pending_releases.clone(),
                    $f.pending_despawns.clone(),
                    $f.notifications.len(),
                    $f.actors.clone(),
                    $f.works.len(),
                ),
            )
        };
    }
    fn sink() -> FlowCommandSink {
        FlowCommandSink::new(42, FlowCallbackConfig::default())
    }
    fn despawn(actor: EntityId, at: SimTime) -> FlowOwnedCommand {
        FlowOwnedCommand::DespawnActor {
            actor,
            at,
            scheduler_priority: 0,
        }
    }
    fn active(
        limit: u64,
    ) -> (
        FlowRuntime,
        EntityId,
        WorkId,
        WorkId,
        ResourceId,
        RequestId,
        RequestId,
        Rc<Cell<u32>>,
    ) {
        let (mut f, actor, carrier, calls) = setup(limit);
        let resource = f.create_resource(1).unwrap();
        let work = f
            .create_work(actor, SimDuration::from_ticks(10), "shared", calls.clone())
            .unwrap();
        let request = f
            .acquire(resource)
            .owner(actor)
            .timed_work(work)
            .submit()
            .unwrap();
        f.step().unwrap().unwrap();
        let other = f.spawn_actor().unwrap();
        let queued = f.submit(resource, other, t(1)).unwrap();
        f.step().unwrap().unwrap();
        assert_eq!(f.request(request).unwrap().state, RequestState::Active);
        assert_eq!(f.request(queued).unwrap().state, RequestState::Queued);
        (f, actor, carrier, work, resource, request, queued, calls)
    }
    #[test]
    fn despawn_direct_duplicate_reservation_preserves_first_event_and_actor() {
        let (mut f, actor, _, calls) = setup(100);
        f.despawn_actor_at_with_scheduler_priority(actor, t(1), -3)
            .unwrap();
        let before = capture!(f);
        assert_eq!(
            f.despawn_actor(actor),
            Err(FlowError::DuplicateActorDespawn)
        );
        assert_eq!(capture!(f), before);
        assert_eq!(calls.get(), 0);
        assert_eq!(f.scheduler.peek_next().unwrap().priority, -3);
        f.step().unwrap().unwrap();
        assert!(!f.pending_despawns.contains(&actor));
        assert!(!f.world.is_alive(actor));
    }
    #[test]
    fn despawn_batch_acquire_then_despawn_and_reverse_admit_atomically() {
        for reverse in [false, true] {
            let (mut f, actor, carrier, calls) = setup(100);
            let resource = f.create_resource(1).unwrap();
            let work = f
                .create_work(actor, SimDuration::from_ticks(10), "shared", calls.clone())
                .unwrap();
            let acquire = FlowOwnedCommand::Acquire(FlowAcquireCommand {
                resource,
                owner: actor,
                work: Some(work),
                at: t(1),
                priority_level: 0,
                deadline: None,
                scheduler_priority: 0,
                timed: true,
                can_preempt: false,
                preemptible: None,
            });
            let mut s = sink();
            if reverse {
                s.emit(despawn(actor, t(1))).unwrap();
                s.emit(acquire).unwrap();
            } else {
                s.emit(acquire).unwrap();
                s.emit(despawn(actor, t(1))).unwrap();
            }
            let FlowBatchReceipt::Accepted(admitted) = f.admit_callback_batch(s) else {
                panic!("valid batch")
            };
            assert_eq!(admitted.len(), 2);
            let request = admitted.iter().find_map(|a| a.request).unwrap();
            assert!(
                f.world.is_alive(actor) && f.world.is_alive(carrier.0) && f.world.is_alive(work.0)
            );
            assert!(f.pending_despawns.contains(&actor));
            let first = f.step().unwrap().unwrap();
            assert_eq!(first.event, admitted[0].event);
            let second = f.step().unwrap().unwrap();
            assert_eq!(second.event, admitted[1].event);
            if reverse {
                assert_eq!(second.error, Some(FlowError::TerminalRequest));
            } else {
                assert!(first.error.is_none() && second.error.is_none());
            }
            assert_eq!(f.request(request).unwrap().state, RequestState::Cancelled);
            assert!(!f.world.is_alive(actor));
            assert!(f.pending_despawns.is_empty());
            assert_eq!(calls.get(), 0);
        }
    }
    #[test]
    fn despawn_batch_release_then_duplicate_rolls_back_all_reservations() {
        let (mut f, actor, _, _, _, q, _, calls) = active(100);
        let lease = f.request(q).unwrap().lease.unwrap();
        let mut s = sink();
        s.emit(FlowOwnedCommand::Release { lease, at: t(2) })
            .unwrap();
        s.emit(despawn(actor, t(2))).unwrap();
        let failed = s.emit(despawn(actor, t(2))).unwrap();
        let before = capture!(f);
        assert_eq!(
            f.admit_callback_batch(s),
            FlowBatchReceipt::Rejected(FlowBatchRejection {
                failed_ticket: Some(failed),
                error: FlowError::DuplicateActorDespawn
            })
        );
        assert_eq!(capture!(f), before);
        assert_eq!(calls.get(), 0);
    }
    #[test]
    fn despawn_batch_time_counter_and_cap_failures_roll_back() {
        for fault in 0..4 {
            let (mut f, actor, carrier, calls) = setup(100);
            let other = f.spawn_actor().unwrap();
            let mut s = FlowCommandSink::new(
                42,
                FlowCallbackConfig {
                    max_callback_commands: NonZeroUsize::new(if fault == 3 { 1 } else { 3 })
                        .unwrap(),
                },
            );
            if fault == 0 {
                f.schedule_domain(carrier, KIND, t(2), 0).unwrap();
                f.step().unwrap().unwrap();
            }
            s.emit(despawn(actor, if fault == 0 { t(2) } else { t(3) }))
                .unwrap();
            let second = s.emit(despawn(other, if fault == 0 { t(1) } else { t(3) }));
            if fault == 1 {
                f.scheduled = OPERATION_CAP - 1;
            }
            let before = capture!(f);
            let scheduler_count = if fault == 2 {
                OPERATION_CAP - 1
            } else {
                f.scheduler.stats().scheduled_events
            };
            let rejected = f.plan_callback_batch(&s, scheduler_count).err().unwrap();
            let expected = match fault {
                0 => FlowError::PastCommand,
                3 => FlowError::CallbackBatchLimitExceeded,
                _ => FlowError::CounterOverflow,
            };
            assert_eq!(rejected.error, expected);
            assert_eq!(
                rejected.failed_ticket,
                if fault == 3 {
                    None
                } else {
                    Some(second.unwrap())
                }
            );
            assert_eq!(capture!(f), before);
            assert_eq!(calls.get(), u32::from(fault == 0));
        }
        // The scheduler-count case above injects the pure planner argument;
        // it does not mutate or claim rollback of the core scheduler counter.
    }
    #[test]
    fn despawn_aggregate_cleanup_overflow_retains_pending_reservation_and_exact_head() {
        let (mut f, actor, carrier, work, r, q, queued, calls) = active(100);
        let lease = f.request(q).unwrap().lease.unwrap();
        f.release(lease, t(5)).unwrap();
        f.schedule_domain(carrier, KIND, t(4), 0).unwrap();
        f.despawn_actor_at_with_scheduler_priority(actor, t(2), 0)
            .unwrap();
        f.destroyed = OPERATION_CAP - 2;
        let before = capture!(f);
        let resource = f.resource(r).unwrap();
        let progress = f.work_progress(work).unwrap();
        let requests = (f.request(q).unwrap(), f.request(queued).unwrap());
        assert_eq!(f.step().unwrap_err(), FlowError::CounterOverflow);
        assert_eq!(capture!(f), before);
        assert_eq!(f.resource(r).unwrap(), resource);
        assert_eq!(f.work_progress(work).unwrap(), progress);
        assert_eq!(
            (f.request(q).unwrap(), f.request(queued).unwrap()),
            requests
        );
        assert_eq!(calls.get(), 0);
    }
    #[test]
    fn despawn_metadata_and_notification_token_overflow_retains_pending_head() {
        for fault in 0..4 {
            let (mut f, actor, carrier, work, r, q, _, calls) = active(100);
            f.despawn_actor_at_with_scheduler_priority(actor, t(2), 0)
                .unwrap();
            match fault {
                0 => {
                    f.registry.remove::<WorkSpec>(carrier.0);
                }
                1 => {
                    f.actor_domains.remove(&actor);
                }
                2 => {
                    f.registry.remove::<WorkContext<Rc<Cell<u32>>>>(carrier.0);
                }
                _ => {
                    f.scheduled = OPERATION_CAP - 1;
                }
            }
            let before = capture!(f);
            let resource = f.resource(r).unwrap();
            let progress = f.work_progress(work).unwrap();
            let request = f.request(q).unwrap();
            assert_eq!(
                f.step().unwrap_err(),
                if fault == 3 {
                    FlowError::CounterOverflow
                } else {
                    FlowError::InvalidState
                }
            );
            assert_eq!(capture!(f), before);
            assert_eq!(f.resource(r).unwrap(), resource);
            assert_eq!(f.work_progress(work).unwrap(), progress);
            assert_eq!(f.request(q).unwrap(), request);
            assert_eq!(calls.get(), 0);
        }
    }
    #[test]
    fn despawn_budget_fail_stop_retains_pending_head() {
        let (mut f, actor, _, work, r, q, _, calls) = active(2);
        let dummy = f.create_resource(0).unwrap();
        let other = f.spawn_actor().unwrap();
        f.despawn_actor_at_with_scheduler_priority(actor, t(2), 0)
            .unwrap();
        f.acquire(dummy)
            .owner(other)
            .at(t(2))
            .scheduler_priority(-1)
            .submit()
            .unwrap();
        f.step().unwrap().unwrap();
        let head = f.scheduler.peek_next();
        let stats = f.scheduler.stats();
        let resource = f.resource(r).unwrap();
        let request = f.request(q).unwrap();
        let progress = f.work_progress(work).unwrap();
        let world = f.world.snapshot();
        assert_eq!(
            f.step().unwrap_err(),
            FlowError::SameTickBudgetExceeded {
                at_ticks: 2,
                limit: 2
            }
        );
        let halt = f.budget_snapshot().halted.unwrap();
        assert_eq!(halt.required_cost, 2);
        assert_eq!(halt.consumed, 1);
        assert_eq!(halt.pending, head.unwrap());
        for _ in 0..2 {
            assert_eq!(
                f.step().unwrap_err(),
                FlowError::SameTickBudgetExceeded {
                    at_ticks: 2,
                    limit: 2
                }
            );
        }
        assert_eq!(f.scheduler.peek_next(), head);
        assert_eq!(f.scheduler.stats(), stats);
        assert_eq!(f.resource(r).unwrap(), resource);
        assert_eq!(f.request(q).unwrap(), request);
        assert_eq!(f.work_progress(work).unwrap(), progress);
        assert_eq!(f.world.snapshot(), world);
        assert!(f.pending_despawns.contains(&actor));
        assert_eq!(calls.get(), 0);
    }
    #[test]
    fn despawn_cleanup_invalidates_old_domain_completion_and_notification_without_callbacks() {
        let (mut f, actor, carrier, work, r, q, queued, calls) = active(100);
        let lease = f.request(q).unwrap().lease.unwrap();
        f.schedule_domain(carrier, KIND, t(3), 0).unwrap();
        f.release(lease, t(5)).unwrap();
        let survivor = f.spawn_actor().unwrap();
        let survivor_calls: Rc<Cell<u32>> = Rc::new(Cell::new(0));
        let survivor_work = f
            .create_actor_domain_context(survivor, "shared", KIND, survivor_calls.clone())
            .unwrap();
        f.schedule_domain(survivor_work, KIND, t(4), 0).unwrap();
        f.despawn_actor_at_with_scheduler_priority(actor, t(2), 0)
            .unwrap();
        let d = f.step().unwrap().unwrap();
        assert_eq!(
            d.records.iter().map(|r| r.transition).collect::<Vec<_>>(),
            vec![LifecycleTransition::Cancelled, LifecycleTransition::Granted]
        );
        assert_eq!(f.request(queued).unwrap().state, RequestState::Active);
        assert_eq!(f.resource(r).unwrap().available, 0);
        assert!(!f.pending_releases.contains(&lease));
        assert!(!f.world.is_alive(work.0));
        let mut seen = BTreeSet::new();
        for row in &d.records {
            assert!(seen.insert((row.causal_event_id, row.transition_ordinal)));
        }
        let mut notifications = 0;
        let mut domain_stale = 0;
        let mut completion_stale = 0;
        let mut drained = false;
        for _ in 0..16 {
            let Some(d) = f.step().unwrap() else {
                drained = true;
                break;
            };
            for row in &d.records {
                assert!(seen.insert((row.causal_event_id, row.transition_ordinal)));
            }
            assert_eq!(calls.get(), 0);
            if d.at == t(2) {
                assert!(d.error.is_none() && d.records.is_empty() && d.callback_batches.is_empty());
                notifications += 1;
            }
            if d.at == t(3) {
                assert!(d.error.is_none() && d.records.is_empty() && d.callback_batches.is_empty());
                domain_stale += 1;
            }
            if d.at == t(10) {
                assert!(d.error.is_none() && d.records.is_empty() && d.callback_batches.is_empty());
                completion_stale += 1;
            }
        }
        assert!(drained);
        assert_eq!((notifications, domain_stale, completion_stale), (2, 1, 1));
        assert_eq!(survivor_calls.get(), 1);
        assert_eq!(f.request(q).unwrap().state, RequestState::Cancelled);
        assert!(f.pending_despawns.is_empty());
    }
}
