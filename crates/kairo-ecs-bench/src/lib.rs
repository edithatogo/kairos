#![forbid(unsafe_code)]

use kairo_ecs_core::Scheduler;
use kairo_ecs_rng::DeterministicStream;
use kairo_ecs_state::World;
use kairo_ecs_types::{EventKind, ScheduleRequest, SimTime, StepOutcome};

pub fn schedule_1m_events_preview(seed: u64, count: u64) -> StepOutcome {
    let mut scheduler = Scheduler::new();
    let mut world = World::new();
    let mut rng = DeterministicStream::new(seed);

    for index in 0..count {
        let entity = world.spawn();
        let priority = (rng.next_u64() % 11) as i32;
        scheduler.schedule(ScheduleRequest {
            at: SimTime::from_ticks(index as u128),
            priority,
            entity: Some(entity),
            kind: EventKind::Custom(index as u32),
        });
    }

    scheduler.run_for(count)
}

pub fn hybrid_des_abm_smoke_preview(seed: u64) -> StepOutcome {
    let mut scheduler = Scheduler::new();
    let mut world = World::new();
    let entity = world.spawn();
    let priority = (seed % 7) as i32;

    scheduler.schedule(ScheduleRequest {
        at: SimTime::from_ticks(1),
        priority,
        entity: Some(entity),
        kind: EventKind::Custom(1),
    });

    scheduler.step()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_helpers_are_callable() {
        assert!(matches!(
            schedule_1m_events_preview(7, 4),
            StepOutcome::LimitReached | StepOutcome::Dispatched(_) | StepOutcome::Empty
        ));
        assert!(matches!(
            hybrid_des_abm_smoke_preview(7),
            StepOutcome::Dispatched(_) | StepOutcome::Empty | StepOutcome::LimitReached
        ));
    }
}
