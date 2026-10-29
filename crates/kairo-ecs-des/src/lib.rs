#![forbid(unsafe_code)]

pub mod boundary;
pub mod egress;
pub mod hazard_clock;
pub mod staged_runtime;
pub use boundary::*;
pub use egress::*;
pub use hazard_clock::*;
pub use staged_runtime::*;

use kairo_ecs_core::Scheduler;
use kairo_ecs_state::World;
use kairo_ecs_types::*;
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TrajectoryStep {
    pub at: SimTime,
    pub priority: i32,
    pub entity: Option<EntityId>,
    pub kind: EventKind,
}

impl TrajectoryStep {
    pub const fn new(
        at: SimTime,
        priority: i32,
        entity: Option<EntityId>,
        kind: EventKind,
    ) -> Self {
        Self {
            at,
            priority,
            entity,
            kind,
        }
    }

    fn schedule_request(self) -> ScheduleRequest {
        ScheduleRequest {
            at: self.at,
            priority: self.priority,
            entity: self.entity,
            kind: self.kind,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrajectoryRequest {
    steps: Vec<TrajectoryStep>,
    max_events: u64,
}

impl TrajectoryRequest {
    pub fn new(max_events: u64) -> Self {
        Self {
            steps: Vec::new(),
            max_events,
        }
    }

    pub fn push_step(&mut self, step: TrajectoryStep) {
        self.steps.push(step);
    }

    pub fn with_step(mut self, step: TrajectoryStep) -> Self {
        self.push_step(step);
        self
    }

    pub fn steps(&self) -> &[TrajectoryStep] {
        &self.steps
    }

    pub fn max_events(&self) -> u64 {
        self.max_events
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Trajectory {
    scheduled: Vec<EventId>,
    dispatched: Vec<DispatchedEvent>,
    outcome: StepOutcome,
    final_time: SimTime,
}

impl Trajectory {
    pub fn scheduled(&self) -> &[EventId] {
        &self.scheduled
    }

    pub fn dispatched(&self) -> &[DispatchedEvent] {
        &self.dispatched
    }

    pub fn outcome(&self) -> StepOutcome {
        self.outcome
    }

    pub fn final_time(&self) -> SimTime {
        self.final_time
    }

    pub fn limit_reached(&self) -> bool {
        self.outcome.is_limit_reached()
    }
}

pub struct Resource {
    name: String,
    capacity: u64,
    available: u64,
    queue: VecDeque<EntityId>,
}

impl Resource {
    pub fn new(name: &str, capacity: u64) -> Self {
        Self {
            name: name.to_string(),
            capacity,
            available: capacity,
            queue: VecDeque::new(),
        }
    }

    pub fn request(&mut self, entity: EntityId) -> bool {
        if self.available > 0 {
            self.available -= 1;
            true
        } else {
            self.queue.push_back(entity);
            false
        }
    }

    pub fn release(&mut self) -> Option<EntityId> {
        if let Some(next) = self.queue.pop_front() {
            Some(next)
        } else {
            self.available = (self.available + 1).min(self.capacity);
            None
        }
    }

    pub fn is_available(&self) -> bool {
        self.available > 0
    }

    pub fn queue_length(&self) -> usize {
        self.queue.len()
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn capacity(&self) -> u64 {
        self.capacity
    }

    pub fn available_count(&self) -> u64 {
        self.available
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriorityRequest {
    pub entity: EntityId,
    pub priority: i32,
    pub requested_at: SimTime,
    pub seq: u64,
}

pub struct PriorityResource {
    name: String,
    capacity: u64,
    available: u64,
    queue: Vec<PriorityRequest>,
    next_seq: u64,
}

impl PriorityResource {
    pub fn new(name: &str, capacity: u64) -> Self {
        Self {
            name: name.to_string(),
            capacity,
            available: capacity,
            queue: Vec::new(),
            next_seq: 0,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn request(&mut self, entity: EntityId, priority: i32, now: SimTime) -> bool {
        if self.available > 0 {
            self.available -= 1;
            true
        } else {
            let seq = self.next_seq;
            self.next_seq += 1;
            self.queue.push(PriorityRequest {
                entity,
                priority,
                requested_at: now,
                seq,
            });
            // Sort by priority (higher first, i.e., larger i32 or smaller i32? Standard SimPy: lower priority number = higher priority. Let's support higher priority number = higher priority or vice versa. Let's make higher priority value mean higher priority, or lower priority value mean higher priority. Let's sort by priority descending (higher integer first), with FIFO tie-break on seq ascending).
            self.queue
                .sort_by(|a, b| b.priority.cmp(&a.priority).then_with(|| a.seq.cmp(&b.seq)));
            false
        }
    }

    pub fn release(&mut self) -> Option<EntityId> {
        if !self.queue.is_empty() {
            let req = self.queue.remove(0);
            Some(req.entity)
        } else {
            self.available = (self.available + 1).min(self.capacity);
            None
        }
    }

    pub fn queue_length(&self) -> usize {
        self.queue.len()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreemptedAllocation {
    pub entity: EntityId,
    pub remaining_work: u64,
    pub interrupted_at: SimTime,
}

pub struct PreemptiveResource {
    name: String,
    capacity: u64,
    holders: Vec<(EntityId, i32, u64)>, // (entity, priority, remaining_work)
    queue: Vec<PriorityRequest>,
    next_seq: u64,
}

impl PreemptiveResource {
    pub fn new(name: &str, capacity: u64) -> Self {
        Self {
            name: name.to_string(),
            capacity,
            holders: Vec::new(),
            queue: Vec::new(),
            next_seq: 0,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn request(
        &mut self,
        entity: EntityId,
        priority: i32,
        work: u64,
        now: SimTime,
    ) -> Result<(), PreemptedAllocation> {
        if (self.holders.len() as u64) < self.capacity {
            self.holders.push((entity, priority, work));
            Ok(())
        } else {
            // Check if any holder has lower priority than incoming entity
            // Find holder with lowest priority
            if let Some(min_idx) = self
                .holders
                .iter()
                .enumerate()
                .min_by_key(|(_, &(_, p, _))| p)
                .map(|(idx, _)| idx)
            {
                let holder = self.holders[min_idx];
                if priority > holder.1 {
                    // Preempt holder
                    self.holders.remove(min_idx);
                    self.holders.push((entity, priority, work));
                    return Err(PreemptedAllocation {
                        entity: holder.0,
                        remaining_work: holder.2,
                        interrupted_at: now,
                    });
                }
            }
            let seq = self.next_seq;
            self.next_seq += 1;
            self.queue.push(PriorityRequest {
                entity,
                priority,
                requested_at: now,
                seq,
            });
            self.queue
                .sort_by(|a, b| b.priority.cmp(&a.priority).then_with(|| a.seq.cmp(&b.seq)));
            Ok(())
        }
    }

    pub fn queue_length(&self) -> usize {
        self.queue.len()
    }

    pub fn holders_count(&self) -> usize {
        self.holders.len()
    }
}

pub struct Container {
    name: String,
    level: f64,
    capacity: f64,
}

impl Container {
    pub fn new(name: &str, capacity: f64, initial_level: f64) -> Self {
        Self {
            name: name.to_string(),
            level: initial_level.min(capacity),
            capacity,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn level(&self) -> f64 {
        self.level
    }

    pub fn capacity(&self) -> f64 {
        self.capacity
    }

    pub fn put(&mut self, amount: f64) -> Result<(), f64> {
        if self.level + amount > self.capacity {
            Err(self.level + amount - self.capacity)
        } else {
            self.level += amount;
            Ok(())
        }
    }

    pub fn get(&mut self, amount: f64) -> Result<(), f64> {
        if self.level < amount {
            Err(amount - self.level)
        } else {
            self.level -= amount;
            Ok(())
        }
    }
}

pub struct Store<T> {
    name: String,
    capacity: usize,
    items: VecDeque<T>,
}

impl<T> Store<T> {
    pub fn new(name: &str, capacity: usize) -> Self {
        Self {
            name: name.to_string(),
            capacity,
            items: VecDeque::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn put(&mut self, item: T) -> Result<(), T> {
        if self.items.len() >= self.capacity {
            Err(item)
        } else {
            self.items.push_back(item);
            Ok(())
        }
    }

    pub fn get(&mut self) -> Option<T> {
        self.items.pop_front()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConditionWait {
    AllOf(usize),
    AnyOf(usize),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RenegingRequest {
    pub entity: EntityId,
    pub requested_at: SimTime,
    pub max_wait: SimTime,
}

pub struct RenegingQueue {
    queue: VecDeque<RenegingRequest>,
}

impl RenegingQueue {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub fn push(&mut self, entity: EntityId, requested_at: SimTime, max_wait: SimTime) {
        self.queue.push_back(RenegingRequest {
            entity,
            requested_at,
            max_wait,
        });
    }

    pub fn request_with_deadline(
        &mut self,
        entity: EntityId,
        requested_at: SimTime,
        max_wait: SimTime,
    ) {
        self.push(entity, requested_at, max_wait);
    }

    pub fn expire_reneged(&mut self, now: SimTime) -> Vec<EntityId> {
        let mut reneged = Vec::new();
        let mut remaining = VecDeque::new();
        while let Some(req) = self.queue.pop_front() {
            if now.ticks() >= req.requested_at.ticks() + req.max_wait.ticks() {
                reneged.push(req.entity);
            } else {
                remaining.push_back(req);
            }
        }
        self.queue = remaining;
        reneged
    }

    pub fn request_with_balk(
        &mut self,
        entity: EntityId,
        requested_at: SimTime,
        max_wait: SimTime,
        max_queue_len: usize,
    ) -> bool {
        if self.queue.len() >= max_queue_len {
            false
        } else {
            self.push(entity, requested_at, max_wait);
            true
        }
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

impl Default for RenegingQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessState {
    YieldAt(SimTime),
    WaitResource(usize),
    Completed,
}

pub trait ProcessCoroutine {
    fn step(&mut self, now: SimTime) -> ProcessState;
}

pub struct DESContext {
    pub scheduler: Scheduler,
    pub world: World,
    pub resources: Vec<Resource>,
}

impl DESContext {
    pub fn new(_seed: u64) -> Self {
        Self {
            scheduler: Scheduler::new(),
            world: World::new(),
            resources: Vec::new(),
        }
    }

    pub fn add_resource(&mut self, name: &str, capacity: u64) {
        self.resources.push(Resource::new(name, capacity));
    }

    pub fn resource(&self, name: &str) -> Option<&Resource> {
        self.resources.iter().find(|r| r.name == name)
    }

    pub fn resource_mut(&mut self, name: &str) -> Option<&mut Resource> {
        self.resources.iter_mut().find(|r| r.name == name)
    }

    pub fn schedule_at(&mut self, at: SimTime, priority: i32, kind: EventKind) -> EventId {
        self.scheduler.schedule(ScheduleRequest {
            at,
            priority,
            entity: None,
            kind,
        })
    }

    pub fn step(&mut self) -> StepOutcome {
        self.scheduler.step()
    }

    pub fn run_for(&mut self, max_events: u64) -> StepOutcome {
        self.scheduler.run_for(max_events)
    }

    pub fn run_trajectory(&mut self, request: TrajectoryRequest) -> Trajectory {
        let mut scheduled = Vec::with_capacity(request.steps().len());
        for step in request.steps {
            scheduled.push(self.scheduler.schedule(step.schedule_request()));
        }

        let mut dispatched = Vec::new();
        let mut outcome = if request.max_events == 0 {
            StepOutcome::LimitReached
        } else {
            StepOutcome::Empty
        };

        for _ in 0..request.max_events {
            match self.scheduler.step() {
                StepOutcome::Dispatched(event) => {
                    outcome = StepOutcome::Dispatched(event);
                    dispatched.push(event);
                }
                StepOutcome::Empty => {
                    outcome = StepOutcome::Empty;
                    break;
                }
                StepOutcome::LimitReached => {
                    outcome = StepOutcome::LimitReached;
                    break;
                }
            }
        }

        if request.max_events > 0 && self.scheduler.pending_events() > 0 {
            outcome = StepOutcome::LimitReached;
        }

        Trajectory {
            scheduled,
            dispatched,
            outcome,
            final_time: self.scheduler.now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_request_blocks() {
        let mut r = Resource::new("m", 1);
        assert!(r.request(EntityId::new(0, 0)));
        assert!(!r.request(EntityId::new(1, 0)));
        r.release();
        assert_eq!(r.queue_length(), 0);
    }

    #[test]
    fn des_context_creates() {
        let mut ctx = DESContext::new(42);
        ctx.add_resource("w", 2);
        assert!(ctx.resource("w").is_some());
    }

    #[test]
    fn schedule_and_step() {
        let mut ctx = DESContext::new(7);
        ctx.schedule_at(SimTime::from_ticks(10), 0, EventKind::Custom(1));
        assert!(matches!(ctx.step(), StepOutcome::Dispatched(_)));
    }

    #[test]
    fn trajectory_records_scheduler_order() {
        let request = TrajectoryRequest::new(4)
            .with_step(TrajectoryStep::new(
                SimTime::from_ticks(10),
                2,
                None,
                EventKind::custom(30),
            ))
            .with_step(TrajectoryStep::new(
                SimTime::from_ticks(5),
                0,
                None,
                EventKind::custom(10),
            ))
            .with_step(TrajectoryStep::new(
                SimTime::from_ticks(10),
                1,
                None,
                EventKind::custom(20),
            ));

        let mut ctx = DESContext::new(0);
        let trajectory = ctx.run_trajectory(request);
        let kinds = trajectory
            .dispatched()
            .iter()
            .map(|event| event.kind.code())
            .collect::<Vec<_>>();

        assert_eq!(kinds, vec![10, 20, 30]);
        assert_eq!(trajectory.final_time(), SimTime::from_ticks(10));
        assert!(!trajectory.limit_reached());
    }

    #[test]
    fn trajectory_reports_event_budget_limit() {
        let request = TrajectoryRequest::new(1)
            .with_step(TrajectoryStep::new(
                SimTime::from_ticks(1),
                0,
                None,
                EventKind::custom(1),
            ))
            .with_step(TrajectoryStep::new(
                SimTime::from_ticks(2),
                0,
                None,
                EventKind::custom(2),
            ));

        let mut ctx = DESContext::new(0);
        let trajectory = ctx.run_trajectory(request);

        assert_eq!(trajectory.dispatched().len(), 1);
        assert!(trajectory.limit_reached());
        assert_eq!(ctx.scheduler.pending_events(), 1);
    }

    #[test]
    fn des_resource_preemption_reneging_simpy_parity() {
        let mut pr = PriorityResource::new("prio", 1);
        let e1 = EntityId::new(1, 0);
        let e2 = EntityId::new(2, 0);
        let e3 = EntityId::new(3, 0);

        assert!(pr.request(e1, 1, SimTime::from_ticks(0)));
        assert!(!pr.request(e2, 2, SimTime::from_ticks(5))); // queued, higher priority
        assert!(!pr.request(e3, 0, SimTime::from_ticks(10))); // queued, lower priority

        // Release e1 -> should pop e2 (higher priority than e3)
        assert_eq!(pr.release(), Some(e2));
        assert_eq!(pr.release(), Some(e3));

        // Preemptive resource test
        let mut prem = PreemptiveResource::new("prem", 1);
        assert!(prem.request(e1, 1, 100, SimTime::from_ticks(0)).is_ok());
        let res = prem.request(e2, 5, 50, SimTime::from_ticks(10));
        assert!(res.is_err());
        let preempted = res.unwrap_err();
        assert_eq!(preempted.entity, e1);
        assert_eq!(preempted.remaining_work, 100);

        // Container test
        let mut container = Container::new("tank", 100.0, 50.0);
        assert!(container.put(30.0).is_ok());
        assert_eq!(container.level(), 80.0);
        assert!(container.get(90.0).is_err());
        assert!(container.get(20.0).is_ok());
        assert_eq!(container.level(), 60.0);

        // Store test
        let mut store = Store::new("box", 2);
        assert!(store.put(10_i32).is_ok());
        assert!(store.put(20_i32).is_ok());
        assert!(store.put(30_i32).is_err());
        assert_eq!(store.get(), Some(10));
        assert_eq!(store.get(), Some(20));
        assert_eq!(store.get(), None);

        // Reneging & Balking test
        let mut reneger = RenegingQueue::new();
        assert!(reneger.request_with_balk(e1, SimTime::from_ticks(0), SimTime::from_ticks(10), 1));
        assert!(!reneger.request_with_balk(e2, SimTime::from_ticks(0), SimTime::from_ticks(10), 1));

        reneger.push(e3, SimTime::from_ticks(0), SimTime::from_ticks(5));
        assert_eq!(reneger.expire_reneged(SimTime::from_ticks(4)).len(), 0);
        let expired = reneger.expire_reneged(SimTime::from_ticks(6));
        assert_eq!(expired, vec![e3]);
    }
}
