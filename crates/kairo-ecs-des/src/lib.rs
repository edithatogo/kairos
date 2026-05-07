#![forbid(unsafe_code)]

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
}
