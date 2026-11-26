use kairo_ecs_abm::{
    AgentBehavior, BehaviorContext, BehaviorDecision, BehaviorSimulation,
    BEHAVIOR_UPDATE_EVENT_KIND,
};
use kairo_ecs_des::{StagedDispatchPreview, StagedRequestState, StagedResourceRuntime};
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

#[test]
fn staged_allocator_preserves_native_behavior_dispatch() {
    let tick = SimTime::from_ticks;
    let mut simulation = BehaviorSimulation::new(7, RecordingBehavior::default());
    let agent = simulation.spawn_agent();
    let cancelled = simulation.schedule_update(agent, tick(1));
    assert!(simulation.context_mut().scheduler.cancel(cancelled));
    let behavior_event = simulation.schedule_update(agent, tick(4));
    let model_before = simulation.context().scheduler.stats();

    let mut allocator = StagedResourceRuntime::new().unwrap();
    let resource = allocator.create_resource(1).unwrap();
    let owner = allocator.spawn_actor().unwrap();
    let next_actor = allocator.spawn_actor().unwrap();
    let request = allocator.submit(resource, owner, tick(4)).unwrap();
    let later = allocator.submit(resource, next_actor, tick(6)).unwrap();

    assert_eq!(
        allocator.peek_with_scheduler(&mut simulation.context_mut().scheduler, tick(3)),
        Ok(None)
    );
    assert_eq!(
        allocator.peek_with_scheduler(&mut simulation.context_mut().scheduler, tick(4)),
        Ok(Some(StagedDispatchPreview::Allocation(tick(4))))
    );
    assert_eq!(allocator.now(), tick(0));
    assert_eq!(
        allocator.request(request).unwrap().state,
        StagedRequestState::Scheduled
    );
    assert_eq!(allocator.run_until_or_for(tick(4), 1), Ok(1));
    assert_eq!(
        allocator.request(request).unwrap().state,
        StagedRequestState::Active
    );

    let Some(StagedDispatchPreview::Model(next)) = allocator
        .peek_with_scheduler(&mut simulation.context_mut().scheduler, tick(6))
        .unwrap()
    else {
        panic!("the native behavior update must precede the tick-six allocation");
    };
    assert_eq!(next.id, behavior_event);
    assert_eq!(next.at, tick(4));
    assert_eq!(simulation.context().scheduler.stats(), model_before);
    assert_eq!(allocator.next_event_time(), Some(tick(6)));

    let trace = simulation.run_for(1);
    let [update] = trace.updates() else {
        panic!("the native behavior handler must run exactly once");
    };
    assert_eq!(update.event.id, next.id);
    assert_eq!(update.agent, agent);
    assert_eq!(update.at, next.at);
    assert_eq!(allocator.close_request(request, update.at), Ok(true));
    assert_eq!(
        allocator.peek_with_scheduler(&mut simulation.context_mut().scheduler, tick(6)),
        Ok(Some(StagedDispatchPreview::Allocation(update.at)))
    );
    assert_eq!(allocator.run_until_or_for(update.at, 1), Ok(1));
    assert_eq!(allocator.resource(resource).unwrap().available, 1);
    assert_eq!(
        allocator.request(later).unwrap().state,
        StagedRequestState::Scheduled
    );

    let (_, behavior) = simulation.into_parts();
    let [(at, kind, _)] = behavior.updates.as_slice() else {
        panic!("expected one recorded behavior update");
    };
    assert_eq!((*at, *kind), (4, BEHAVIOR_UPDATE_EVENT_KIND));
}
