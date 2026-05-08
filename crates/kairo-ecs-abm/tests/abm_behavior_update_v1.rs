use kairo_ecs_abm::{
    AgentBehavior, BehaviorContext, BehaviorDecision, BehaviorSimulation,
    BEHAVIOR_UPDATE_EVENT_KIND,
};
use kairo_ecs_types::SimTime;

#[derive(Default)]
struct RecordingBehavior {
    updates: Vec<(u128, u32, u64)>,
}

impl AgentBehavior for RecordingBehavior {
    fn update(&mut self, context: BehaviorContext<'_>) -> BehaviorDecision {
        self.updates.push((
            context.event.at.ticks(),
            context.event.kind.code(),
            context.rng.next_u64(),
        ));
        BehaviorDecision::Continue
    }
}

#[test]
fn abm_behavior_update_v1_runs_updates_in_scheduler_order() {
    let mut simulation = BehaviorSimulation::new(7, RecordingBehavior::default());
    let first = simulation.spawn_agent();
    let second = simulation.spawn_agent();

    simulation.schedule_update(first, SimTime::from_ticks(30));
    simulation.schedule_update(second, SimTime::from_ticks(10));
    simulation.schedule_update(first, SimTime::from_ticks(20));

    let trace = simulation.run_for(3);
    let (_context, behavior) = simulation.into_parts();

    assert_eq!(trace.len(), 3);
    assert_eq!(
        behavior
            .updates
            .iter()
            .map(|(ticks, kind, _rng)| (*ticks, *kind))
            .collect::<Vec<_>>(),
        vec![
            (10, BEHAVIOR_UPDATE_EVENT_KIND),
            (20, BEHAVIOR_UPDATE_EVENT_KIND),
            (30, BEHAVIOR_UPDATE_EVENT_KIND),
        ]
    );
}

#[test]
fn abm_behavior_update_v1_replays_entity_rng_streams() {
    let mut first_run = BehaviorSimulation::new(99, RecordingBehavior::default());
    let first_agent = first_run.spawn_agent();
    first_run.schedule_update(first_agent, SimTime::from_ticks(1));
    let _ = first_run.run_for(1);
    let (_first_context, first_behavior) = first_run.into_parts();

    let mut second_run = BehaviorSimulation::new(99, RecordingBehavior::default());
    let second_agent = second_run.spawn_agent();
    second_run.schedule_update(second_agent, SimTime::from_ticks(1));
    let _ = second_run.run_for(1);
    let (_second_context, second_behavior) = second_run.into_parts();

    assert_eq!(first_agent, second_agent);
    assert_eq!(first_behavior.updates[0].2, second_behavior.updates[0].2);
}
