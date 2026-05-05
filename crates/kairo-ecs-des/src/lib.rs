#![forbid(unsafe_code)]

use kairo_ecs_core::Scheduler;
use kairo_ecs_types::{
    DispatchedEvent, EntityId, EventId, EventKind, ScheduleRequest, SimTime, StepOutcome,
};

/// DES event request expressed in fixed simulation ticks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrajectoryStep {
    pub at: SimTime,
    pub priority: i32,
    pub entity: Option<EntityId>,
    pub kind: EventKind,
}

impl TrajectoryStep {
    pub fn new(at: SimTime, priority: i32, kind: EventKind) -> Self {
        Self {
            at,
            priority,
            entity: None,
            kind,
        }
    }

    pub fn for_entity(mut self, entity: EntityId) -> Self {
        self.entity = Some(entity);
        self
    }
}

impl From<TrajectoryStep> for ScheduleRequest {
    fn from(step: TrajectoryStep) -> Self {
        Self {
            at: step.at,
            priority: step.priority,
            entity: step.entity,
            kind: step.kind,
        }
    }
}

/// Scheduled DES trajectory ready to be replayed by the shared scheduler.
#[derive(Debug, Default)]
pub struct Trajectory {
    scheduler: Scheduler,
    scheduled: Vec<EventId>,
}

impl Trajectory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn schedule(&mut self, step: TrajectoryStep) -> EventId {
        let id = self.scheduler.schedule(step.into());
        self.scheduled.push(id);
        id
    }

    pub fn scheduled_event_ids(&self) -> &[EventId] {
        &self.scheduled
    }

    pub fn run_for(mut self, max_events: u64) -> TrajectoryTrace {
        let mut events = Vec::new();
        for _ in 0..max_events {
            match self.scheduler.step() {
                StepOutcome::Dispatched(event) => events.push(event),
                StepOutcome::Empty | StepOutcome::LimitReached => break,
            }
        }

        TrajectoryTrace { events }
    }
}

/// Deterministic dispatch trace produced by a DES trajectory run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrajectoryTrace {
    events: Vec<DispatchedEvent>,
}

impl TrajectoryTrace {
    pub fn events(&self) -> &[DispatchedEvent] {
        &self.events
    }

    pub fn event_kinds(&self) -> Vec<EventKind> {
        self.events.iter().map(|event| event.kind.clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(at: u128, priority: i32, kind: u32) -> TrajectoryStep {
        TrajectoryStep::new(SimTime::from_ticks(at), priority, EventKind::Custom(kind))
    }

    #[test]
    fn trajectory_replays_scheduler_ordering() {
        let mut trajectory = Trajectory::new();
        trajectory.schedule(step(10, 2, 3));
        trajectory.schedule(step(5, 9, 1));
        trajectory.schedule(step(10, 1, 2));
        trajectory.schedule(step(10, 1, 4));

        let trace = trajectory.run_for(8);

        assert_eq!(
            trace.event_kinds(),
            vec![
                EventKind::Custom(1),
                EventKind::Custom(2),
                EventKind::Custom(4),
                EventKind::Custom(3),
            ]
        );
    }

    #[test]
    fn trajectory_run_for_is_bounded() {
        let mut trajectory = Trajectory::new();
        trajectory.schedule(step(1, 0, 1));
        trajectory.schedule(step(2, 0, 2));

        let trace = trajectory.run_for(1);

        assert_eq!(trace.events().len(), 1);
        assert_eq!(trace.events()[0].kind, EventKind::Custom(1));
    }
}
