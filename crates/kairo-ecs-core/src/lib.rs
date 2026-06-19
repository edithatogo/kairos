#![forbid(unsafe_code)]

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};

use kairo_ecs_types::{
    DispatchedEvent, EntityId, EventId, EventKind, ScheduleRequest, SimTime, StepOutcome,
    TypesError,
};

#[derive(Clone, Debug, Eq, PartialEq)]
struct QueueEntry {
    request: ScheduleRequest,
    sequence: u64,
    id: EventId,
}

impl Ord for QueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .request
            .at
            .cmp(&self.request.at)
            .then_with(|| other.request.priority.cmp(&self.request.priority))
            .then_with(|| other.sequence.cmp(&self.sequence))
    }
}

impl PartialOrd for QueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Deterministic single-threaded scheduler.
#[derive(Debug, Default)]
pub struct Scheduler {
    heap: BinaryHeap<QueueEntry>,
    pending: HashSet<EventId>,
    next_event_index: u64,
    next_event_generation: u32,
    next_sequence: u64,
    now: SimTime,
    scheduled_events: u64,
    dispatched_events: u64,
    cancelled_events: u64,
}

impl Scheduler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn now(&self) -> SimTime {
        self.now
    }

    pub fn schedule(&mut self, request: ScheduleRequest) -> EventId {
        let id = EventId {
            index: self.next_event_index,
            generation: self.next_event_generation,
        };
        self.next_event_index += 1;
        self.next_event_generation = self.next_event_generation.wrapping_add(1);

        let entry = QueueEntry {
            request,
            sequence: self.next_sequence,
            id,
        };
        self.next_sequence += 1;
        self.scheduled_events += 1;
        self.pending.insert(id);
        self.heap.push(entry);
        id
    }

    pub fn cancel(&mut self, id: EventId) -> bool {
        let removed = self.pending.remove(&id);
        if removed {
            self.cancelled_events += 1;
        }
        removed
    }

    pub fn pending_events(&self) -> usize {
        self.pending.len()
    }

    pub fn stats(&self) -> SchedulerStats {
        SchedulerStats {
            now: self.now,
            scheduled_events: self.scheduled_events,
            dispatched_events: self.dispatched_events,
            cancelled_events: self.cancelled_events,
            pending_events: self.pending.len() as u64,
        }
    }

    fn prune_dead_entries(&mut self) {
        while let Some(entry) = self.heap.peek() {
            if self.pending.contains(&entry.id) {
                break;
            }

            self.heap
                .pop()
                .expect("heap.peek() returned an entry that must be poppable");
        }
    }

    fn next_event_at(&mut self) -> Option<SimTime> {
        self.prune_dead_entries();
        self.heap.peek().map(|entry| entry.request.at)
    }

    pub fn step(&mut self) -> StepOutcome {
        self.prune_dead_entries();

        match self.heap.pop() {
            Some(entry) => {
                debug_assert!(
                    self.pending.remove(&entry.id),
                    "live heap entry should be tracked as pending"
                );
                self.dispatched_events += 1;
                self.now = entry.request.at;
                StepOutcome::Dispatched(DispatchedEvent {
                    id: entry.id,
                    at: entry.request.at,
                    priority: entry.request.priority,
                    sequence: entry.sequence,
                    entity: entry.request.entity,
                    kind: entry.request.kind,
                })
            }
            None => StepOutcome::Empty,
        }
    }

    pub fn run_for(&mut self, max_events: u64) -> StepOutcome {
        if max_events == 0 {
            return StepOutcome::LimitReached;
        }

        let mut last = StepOutcome::Empty;
        for _ in 0..max_events {
            match self.step() {
                StepOutcome::Dispatched(event) => {
                    last = StepOutcome::Dispatched(event);
                }
                StepOutcome::Empty => {
                    return if matches!(last, StepOutcome::Empty) {
                        StepOutcome::Empty
                    } else {
                        last
                    };
                }
                StepOutcome::LimitReached => return StepOutcome::LimitReached,
            }
        }

        if self.next_event_at().is_some() {
            StepOutcome::LimitReached
        } else if matches!(last, StepOutcome::Empty) {
            StepOutcome::Empty
        } else {
            last
        }
    }

    pub fn run_until(&mut self, time_limit: SimTime) -> StepOutcome {
        let mut last = StepOutcome::Empty;

        while let Some(next_at) = self.next_event_at() {
            if next_at > time_limit {
                return last;
            }

            match self.step() {
                StepOutcome::Dispatched(event) => last = StepOutcome::Dispatched(event),
                StepOutcome::Empty => return last,
                StepOutcome::LimitReached => return StepOutcome::LimitReached,
            }
        }

        last
    }

    pub fn run_until_or_for(&mut self, time_limit: SimTime, max_events: u64) -> StepOutcome {
        if max_events == 0 {
            return StepOutcome::LimitReached;
        }

        let mut last = StepOutcome::Empty;
        for _ in 0..max_events {
            match self.next_event_at() {
                Some(next_at) if next_at > time_limit => {
                    return last;
                }
                Some(_) => match self.step() {
                    StepOutcome::Dispatched(event) => last = StepOutcome::Dispatched(event),
                    StepOutcome::Empty => return last,
                    StepOutcome::LimitReached => return StepOutcome::LimitReached,
                },
                None => return last,
            }
        }

        match self.next_event_at() {
            Some(next_at) if next_at <= time_limit => StepOutcome::LimitReached,
            Some(_) | None => {
                if matches!(last, StepOutcome::Empty) {
                    StepOutcome::Empty
                } else {
                    last
                }
            }
        }
    }
}

/// Snapshot of scheduler counters and current virtual time.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SchedulerStats {
    pub now: SimTime,
    pub scheduled_events: u64,
    pub dispatched_events: u64,
    pub cancelled_events: u64,
    pub pending_events: u64,
}

/// Stable status surface for the pure Rust facade.
///
/// Track 02 can map these values onto C/FFI status codes without depending on
/// scheduler internals.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoreStatus {
    Ok = 0,
    Dispatched = 1,
    Empty = 2,
    LimitReached = 3,
    NotFound = 4,
    InvalidPriority = 5,
}

impl CoreStatus {
    pub const fn code(self) -> u32 {
        self as u32
    }
}

impl From<StepOutcome> for CoreStatus {
    fn from(outcome: StepOutcome) -> Self {
        match outcome {
            StepOutcome::Dispatched(_) => Self::Dispatched,
            StepOutcome::Empty => Self::Empty,
            StepOutcome::LimitReached => Self::LimitReached,
        }
    }
}

impl From<TypesError> for CoreStatus {
    fn from(error: TypesError) -> Self {
        match error {
            TypesError::PriorityOverflow => Self::InvalidPriority,
            TypesError::TicksOverflow
            | TypesError::IndexOverflow
            | TypesError::GenerationOverflow
            | TypesError::UnsupportedDtoVersion(_) => Self::NotFound,
        }
    }
}

/// Result returned by facade scheduling calls.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScheduleStatus {
    pub status: CoreStatus,
    pub event_id: Option<EventId>,
}

impl ScheduleStatus {
    pub const fn ok(event_id: EventId) -> Self {
        Self {
            status: CoreStatus::Ok,
            event_id: Some(event_id),
        }
    }

    pub const fn error(status: CoreStatus) -> Self {
        Self {
            status,
            event_id: None,
        }
    }
}

/// Result returned by facade step/run calls.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StepStatus {
    pub status: CoreStatus,
    pub event: Option<DispatchedEvent>,
}

impl StepStatus {
    pub const fn new(status: CoreStatus, event: Option<DispatchedEvent>) -> Self {
        Self { status, event }
    }
}

impl From<StepOutcome> for StepStatus {
    fn from(outcome: StepOutcome) -> Self {
        match outcome {
            StepOutcome::Dispatched(event) => Self::new(CoreStatus::Dispatched, Some(event)),
            StepOutcome::Empty => Self::new(CoreStatus::Empty, None),
            StepOutcome::LimitReached => Self::new(CoreStatus::LimitReached, None),
        }
    }
}

/// Pure Rust facade for handle/status oriented integrations.
#[derive(Debug, Default)]
pub struct SchedulerFacade {
    scheduler: Scheduler,
}

impl SchedulerFacade {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn now(&self) -> SimTime {
        self.scheduler.now()
    }

    pub fn pending_events(&self) -> usize {
        self.scheduler.pending_events()
    }

    pub fn stats(&self) -> SchedulerStats {
        self.scheduler.stats()
    }

    pub fn schedule(&mut self, request: ScheduleRequest) -> ScheduleStatus {
        ScheduleStatus::ok(self.scheduler.schedule(request))
    }

    pub fn schedule_custom(
        &mut self,
        at_ticks: u128,
        priority: i64,
        entity: Option<EntityId>,
        kind_code: u32,
    ) -> ScheduleStatus {
        match ScheduleRequest::try_new(
            SimTime::from_ticks(at_ticks),
            priority,
            entity,
            EventKind::custom(kind_code),
        ) {
            Ok(request) => self.schedule(request),
            Err(error) => ScheduleStatus::error(CoreStatus::from(error)),
        }
    }

    pub fn cancel(&mut self, id: EventId) -> CoreStatus {
        if self.scheduler.cancel(id) {
            CoreStatus::Ok
        } else {
            CoreStatus::NotFound
        }
    }

    pub fn step(&mut self) -> StepStatus {
        self.scheduler.step().into()
    }

    pub fn run_for(&mut self, max_events: u64) -> StepStatus {
        self.scheduler.run_for(max_events).into()
    }

    pub fn run_until(&mut self, time_limit: SimTime) -> StepStatus {
        self.scheduler.run_until(time_limit).into()
    }

    pub fn run_until_or_for(&mut self, time_limit: SimTime, max_events: u64) -> StepStatus {
        self.scheduler
            .run_until_or_for(time_limit, max_events)
            .into()
    }
}

/// A recorded event in the trace.
#[derive(Debug, Clone)]
pub struct RecordedEvent {
    pub tick: u64,
    pub event_id: u64,
    pub entity_id: Option<u64>,
    pub priority: i32,
    pub sequence: u64,
    pub kind: u32,
}

#[cfg(feature = "numa")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NumaSupport {
    Supported,
    Unsupported,
}

#[cfg(feature = "numa")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NumaError {
    UnsupportedHost,
    EmptyTopology,
    InvalidAffinity,
}

#[cfg(feature = "numa")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NumaNode {
    pub id: u32,
    pub cores: Vec<u32>,
    pub memory_bytes: Option<u64>,
}

#[cfg(feature = "numa")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NumaTopology {
    pub support: NumaSupport,
    pub nodes: Vec<NumaNode>,
    pub source: String,
}

#[cfg(feature = "numa")]
impl NumaTopology {
    pub fn unsupported(reason: impl Into<String>) -> Self {
        Self {
            support: NumaSupport::Unsupported,
            nodes: Vec::new(),
            source: reason.into(),
        }
    }

    pub fn synthetic_single_node(core_count: u32, memory_bytes: Option<u64>) -> Self {
        Self {
            support: NumaSupport::Supported,
            nodes: vec![NumaNode {
                id: 0,
                cores: (0..core_count).collect(),
                memory_bytes,
            }],
            source: "synthetic".to_string(),
        }
    }

    pub fn validate(&self) -> Result<(), NumaError> {
        match self.support {
            NumaSupport::Unsupported => Err(NumaError::UnsupportedHost),
            NumaSupport::Supported if self.nodes.is_empty() => Err(NumaError::EmptyTopology),
            NumaSupport::Supported => Ok(()),
        }
    }
}

#[cfg(feature = "numa")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AffinityAction {
    Bind,
    RestorePrevious,
}

#[cfg(feature = "numa")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AffinityBindingPlan {
    pub thread_id: u64,
    pub core_id: u32,
    pub opt_in: bool,
    pub action: AffinityAction,
}

#[cfg(feature = "numa")]
impl AffinityBindingPlan {
    pub fn new(thread_id: u64, core_id: u32, opt_in: bool) -> Result<Self, NumaError> {
        if !opt_in {
            return Err(NumaError::InvalidAffinity);
        }
        Ok(Self {
            thread_id,
            core_id,
            opt_in,
            action: AffinityAction::Bind,
        })
    }

    pub fn reversal(&self) -> Self {
        Self {
            thread_id: self.thread_id,
            core_id: self.core_id,
            opt_in: self.opt_in,
            action: AffinityAction::RestorePrevious,
        }
    }
}

#[cfg(feature = "numa")]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EventPoolMetrics {
    pub created: u64,
    pub reused: u64,
    pub recycled: u64,
    pub uses_global_lock: bool,
}

#[cfg(feature = "numa")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PooledEvent<T> {
    pub value: T,
}

#[cfg(feature = "numa")]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EventPool<T> {
    free: Vec<PooledEvent<T>>,
    metrics: EventPoolMetrics,
}

#[cfg(feature = "numa")]
impl<T> EventPool<T> {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            free: Vec::with_capacity(capacity),
            metrics: EventPoolMetrics::default(),
        }
    }

    pub fn checkout(&mut self, value: T) -> PooledEvent<T> {
        if let Some(mut event) = self.free.pop() {
            event.value = value;
            self.metrics.reused += 1;
            event
        } else {
            self.metrics.created += 1;
            PooledEvent { value }
        }
    }

    pub fn recycle(&mut self, event: PooledEvent<T>) {
        self.metrics.recycled += 1;
        self.free.push(event);
    }

    pub fn metrics(&self) -> EventPoolMetrics {
        self.metrics
    }
}

/// A scheduler wrapper that records all dispatched events.
pub struct RecordingScheduler {
    pub inner: Scheduler,
    pub recorded: Vec<RecordedEvent>,
}

impl RecordingScheduler {
    pub fn new(_seed: u64) -> Self {
        Self {
            inner: Scheduler::new(),
            recorded: Vec::new(),
        }
    }

    pub fn schedule(&mut self, req: ScheduleRequest) -> EventId {
        self.inner.schedule(req)
    }

    pub fn cancel(&mut self, id: EventId) -> bool {
        self.inner.cancel(id)
    }

    pub fn step(&mut self) -> StepOutcome {
        let outcome = self.inner.step();
        if let StepOutcome::Dispatched(ref ev) = outcome {
            self.recorded.push(RecordedEvent {
                tick: ev.at.ticks() as u64,
                event_id: ev.id.index,
                entity_id: ev.entity.map(|e| e.index),
                priority: ev.priority,
                sequence: ev.sequence,
                kind: match ev.kind {
                    EventKind::Custom(v) => v,
                },
            });
        }
        outcome
    }

    pub fn run_for(&mut self, max: u64) -> u64 {
        let mut count = 0;
        while count < max {
            match self.step() {
                StepOutcome::Dispatched(_) => count += 1,
                _ => break,
            }
        }
        count
    }

    pub fn pending_events(&self) -> usize {
        self.inner.pending_events()
    }

    pub fn now(&self) -> SimTime {
        self.inner.now()
    }
}

#[cfg(test)]
mod tests {
    use kairo_ecs_types::{EntityId, EventKind, ScheduleRequest, SimTime, StepOutcome};

    use super::*;

    fn request(at: u128, priority: i32, kind: u32) -> ScheduleRequest {
        ScheduleRequest {
            at: SimTime::from_ticks(at),
            priority,
            entity: None,
            kind: EventKind::Custom(kind),
        }
    }

    #[test]
    fn dispatches_by_time_priority_then_sequence() {
        let mut scheduler = Scheduler::new();
        scheduler.schedule(request(10, 2, 3));
        scheduler.schedule(request(5, 9, 1));
        scheduler.schedule(request(10, 1, 2));
        scheduler.schedule(request(10, 1, 4));

        let mut kinds = Vec::new();
        while let StepOutcome::Dispatched(event) = scheduler.step() {
            let EventKind::Custom(kind) = event.kind;
            kinds.push(kind);
        }

        assert_eq!(kinds, vec![1, 2, 4, 3]);
    }

    #[test]
    fn cancellation_skips_event_without_reordering_rest() {
        let mut scheduler = Scheduler::new();
        scheduler.schedule(request(1, 0, 1));
        let cancelled = scheduler.schedule(request(2, 0, 2));
        scheduler.schedule(request(3, 0, 3));

        assert!(scheduler.cancel(cancelled));
        assert!(!scheduler.cancel(cancelled));
        assert_eq!(scheduler.pending_events(), 2);

        let first = scheduler.step();
        let second = scheduler.step();
        let third = scheduler.step();

        assert!(matches!(first, StepOutcome::Dispatched(_)));
        assert!(matches!(second, StepOutcome::Dispatched(_)));
        assert_eq!(third, StepOutcome::Empty);
        assert_eq!(scheduler.pending_events(), 0);
    }

    #[test]
    fn cancellation_rejects_unknown_and_dispatched_events() {
        let mut scheduler = Scheduler::new();
        let dispatched = scheduler.schedule(request(1, 0, 1));

        assert!(matches!(scheduler.step(), StepOutcome::Dispatched(_)));
        assert!(!scheduler.cancel(dispatched));
        assert!(!scheduler.cancel(EventId {
            index: 999,
            generation: 0,
        }));
        assert_eq!(scheduler.pending_events(), 0);
    }

    #[test]
    fn cancelled_future_event_does_not_force_limit_after_active_events_finish() {
        let mut scheduler = Scheduler::new();
        scheduler.schedule(request(1, 0, 1));
        let cancelled = scheduler.schedule(request(100, 0, 2));
        assert!(scheduler.cancel(cancelled));

        let outcome = scheduler.run_for(8);

        assert!(matches!(outcome, StepOutcome::Dispatched(_)));
        assert_eq!(scheduler.pending_events(), 0);
        assert_eq!(scheduler.step(), StepOutcome::Empty);
    }

    #[test]
    fn run_for_is_bounded() {
        let mut scheduler = Scheduler::new();
        scheduler.schedule(request(1, 0, 1));
        scheduler.schedule(request(2, 0, 2));
        scheduler.schedule(request(3, 0, 3));

        let outcome = scheduler.run_for(2);
        assert_eq!(outcome, StepOutcome::LimitReached);
        assert_eq!(scheduler.pending_events(), 1);
    }

    #[test]
    fn run_for_with_zero_limit_reports_limit_reached() {
        let mut scheduler = Scheduler::new();
        scheduler.schedule(request(1, 0, 1));

        assert_eq!(scheduler.run_for(0), StepOutcome::LimitReached);
        assert_eq!(scheduler.pending_events(), 1);
    }

    #[test]
    fn run_until_stops_at_time_limit() {
        let mut scheduler = Scheduler::new();
        scheduler.schedule(request(1, 0, 1));
        scheduler.schedule(request(4, 0, 2));

        let outcome = scheduler.run_until(SimTime::from_ticks(1));
        assert!(matches!(outcome, StepOutcome::Dispatched(_)));
        assert_eq!(scheduler.now(), SimTime::from_ticks(1));
        assert_eq!(scheduler.pending_events(), 1);
    }

    #[test]
    fn run_until_or_for_stops_on_bounds() {
        let mut scheduler = Scheduler::new();
        scheduler.schedule(request(1, 0, 1));
        scheduler.schedule(request(2, 0, 2));

        let outcome = scheduler.run_until_or_for(SimTime::from_ticks(1), 1);
        assert!(matches!(outcome, StepOutcome::Dispatched(_)));
        assert_eq!(scheduler.pending_events(), 1);

        let bounded = scheduler.run_until_or_for(SimTime::from_ticks(0), 0);
        assert_eq!(bounded, StepOutcome::LimitReached);
    }

    #[test]
    fn run_until_or_for_reports_limit_reached_when_event_budget_is_exhausted_first() {
        let mut scheduler = Scheduler::new();
        scheduler.schedule(request(1, 0, 1));
        scheduler.schedule(request(1, 0, 2));
        scheduler.schedule(request(2, 0, 3));

        let outcome = scheduler.run_until_or_for(SimTime::from_ticks(10), 2);

        assert_eq!(outcome, StepOutcome::LimitReached);
        assert_eq!(scheduler.pending_events(), 1);
        assert_eq!(scheduler.now(), SimTime::from_ticks(1));
    }

    #[test]
    fn facade_schedules_steps_and_exposes_status_codes() {
        let mut facade = SchedulerFacade::new();

        let first = facade.schedule_custom(10, 1, None, 10);
        let second = facade.schedule_custom(5, 0, Some(EntityId::new(7, 2)), 20);

        assert_eq!(first.status, CoreStatus::Ok);
        assert_eq!(second.status, CoreStatus::Ok);
        assert_eq!(facade.pending_events(), 2);

        let first_step = facade.step();
        assert_eq!(first_step.status, CoreStatus::Dispatched);
        assert_eq!(first_step.event.unwrap().kind.code(), 20);

        let second_step = facade.run_for(1);
        assert_eq!(second_step.status, CoreStatus::Dispatched);
        assert_eq!(second_step.event.unwrap().kind.code(), 10);

        let empty = facade.step();
        assert_eq!(empty.status, CoreStatus::Empty);
        assert_eq!(empty.event, None);
    }

    #[test]
    fn facade_reports_invalid_priority_without_scheduling() {
        let mut facade = SchedulerFacade::new();

        let result = facade.schedule_custom(0, i64::from(i32::MAX) + 1, None, 1);

        assert_eq!(result.status, CoreStatus::InvalidPriority);
        assert_eq!(result.event_id, None);
        assert_eq!(facade.pending_events(), 0);
    }

    #[test]
    fn facade_maps_cancellation_and_limit_statuses() {
        let mut facade = SchedulerFacade::new();
        let scheduled = facade.schedule_custom(1, 0, None, 1);
        let id = scheduled.event_id.unwrap();

        assert_eq!(facade.cancel(id), CoreStatus::Ok);
        assert_eq!(facade.cancel(id), CoreStatus::NotFound);

        let zero_budget = facade.run_for(0);
        assert_eq!(zero_budget.status, CoreStatus::LimitReached);
        assert_eq!(zero_budget.event, None);
    }

    #[test]
    fn scheduler_stats_track_scheduled_cancelled_dispatched_and_pending() {
        let mut scheduler = Scheduler::new();
        let first = scheduler.schedule(request(1, 0, 1));
        let second = scheduler.schedule(request(2, 0, 2));

        assert_eq!(
            scheduler.stats(),
            SchedulerStats {
                now: SimTime::ZERO,
                scheduled_events: 2,
                dispatched_events: 0,
                cancelled_events: 0,
                pending_events: 2,
            }
        );

        assert!(scheduler.cancel(second));
        assert!(!scheduler.cancel(second));
        assert!(matches!(scheduler.step(), StepOutcome::Dispatched(event) if event.id == first));

        assert_eq!(
            scheduler.stats(),
            SchedulerStats {
                now: SimTime::from_ticks(1),
                scheduled_events: 2,
                dispatched_events: 1,
                cancelled_events: 1,
                pending_events: 0,
            }
        );
    }

    #[test]
    fn facade_exposes_scheduler_stats_snapshot() {
        let mut facade = SchedulerFacade::new();
        facade.schedule_custom(4, 0, None, 4);

        assert_eq!(facade.stats().scheduled_events, 1);
        assert_eq!(facade.stats().pending_events, 1);

        let _ = facade.step();

        assert_eq!(facade.stats().now, SimTime::from_ticks(4));
        assert_eq!(facade.stats().dispatched_events, 1);
        assert_eq!(facade.stats().pending_events, 0);
    }

    #[cfg(feature = "numa")]
    #[test]
    fn numa_topology_reports_typed_unsupported_fallback() {
        let topology = NumaTopology::unsupported("hwloc unavailable in local test");

        assert_eq!(topology.support, NumaSupport::Unsupported);
        assert_eq!(topology.nodes.len(), 0);
        assert_eq!(topology.validate(), Err(NumaError::UnsupportedHost));
    }

    #[cfg(feature = "numa")]
    #[test]
    fn affinity_plan_is_opt_in_and_reversible_without_binding() {
        let plan = AffinityBindingPlan::new(0, 3, true).unwrap();
        let reversal = plan.reversal();

        assert!(plan.opt_in);
        assert_eq!(plan.core_id, 3);
        assert_eq!(reversal.thread_id, 0);
        assert_eq!(reversal.action, AffinityAction::RestorePrevious);
    }

    #[cfg(feature = "numa")]
    #[test]
    fn event_pool_reuses_slots_without_global_lock_contract() {
        let mut pool = EventPool::with_capacity(2);

        let first = pool.checkout("first");
        assert_eq!(pool.metrics().created, 1);
        pool.recycle(first);
        let second = pool.checkout("second");

        assert_eq!(second.value, "second");
        assert_eq!(pool.metrics().created, 1);
        assert_eq!(pool.metrics().reused, 1);
        assert!(!pool.metrics().uses_global_lock);
    }
}
