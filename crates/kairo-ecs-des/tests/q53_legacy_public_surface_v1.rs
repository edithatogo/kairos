use kairo_ecs_core::Scheduler;
use kairo_ecs_des::{DESContext, Resource, TrajectoryRequest, TrajectoryStep};
use kairo_ecs_state::World;
use kairo_ecs_types::{EntityId, EventKind, SimTime, StepOutcome};

#[test]
fn legacy_context_public_fields_and_methods_remain_usable() {
    let mut context = DESContext {
        scheduler: Scheduler::new(),
        world: World::new(),
        resources: vec![Resource::new("triage", 1)],
    };

    assert_eq!(context.resources.len(), 1);
    assert_eq!(context.resource("triage").unwrap().name(), "triage");
    assert_eq!(context.resource("triage").unwrap().capacity(), 1);
    assert!(context
        .resource_mut("triage")
        .unwrap()
        .request(EntityId::new(1, 0)));
    assert_eq!(context.resource("triage").unwrap().available_count(), 0);

    context.add_resource("imaging", 2);
    assert_eq!(context.resource("imaging").unwrap().capacity(), 2);
    assert!(context.resource("missing").is_none());

    let id = context.schedule_at(SimTime::from_ticks(4), 0, EventKind::custom(9));
    assert!(matches!(context.step(), StepOutcome::Dispatched(event) if event.id == id));
    assert_eq!(context.run_for(1), StepOutcome::Empty);
}

#[test]
fn legacy_resource_capacity_two_fifo_release_conserves_capacity() {
    let mut resource = Resource::new("two-bay", 2);
    let first = EntityId::new(1, 0);
    let second = EntityId::new(2, 0);
    let third = EntityId::new(3, 0);
    let fourth = EntityId::new(4, 0);

    assert_eq!(resource.name(), "two-bay");
    assert_eq!(resource.capacity(), 2);
    assert!(resource.is_available());
    assert!(resource.request(first));
    assert!(resource.request(second));
    assert!(!resource.request(third));
    assert!(!resource.request(fourth));
    assert_eq!(resource.available_count(), 0);
    assert_eq!(resource.queue_length(), 2);

    assert_eq!(resource.release(), Some(third));
    assert_eq!(resource.available_count(), 0);
    assert_eq!(resource.release(), Some(fourth));
    assert_eq!(resource.queue_length(), 0);
    assert_eq!(resource.release(), None);
    assert_eq!(resource.available_count(), 1);
    assert_eq!(resource.release(), None);
    assert_eq!(resource.available_count(), 2);
    assert!(resource.is_available());
    assert_eq!(resource.release(), None);
    assert_eq!(resource.available_count(), 2);
}

#[test]
fn legacy_trajectory_accessors_preserve_dispatch_order_and_event_limit() {
    let mut request = TrajectoryRequest::new(2);
    request.push_step(TrajectoryStep::new(
        SimTime::from_ticks(10),
        1,
        None,
        EventKind::custom(30),
    ));
    request = request.with_step(TrajectoryStep::new(
        SimTime::from_ticks(5),
        0,
        None,
        EventKind::custom(10),
    ));
    request = request.with_step(TrajectoryStep::new(
        SimTime::from_ticks(10),
        0,
        None,
        EventKind::custom(20),
    ));

    assert_eq!(request.steps().len(), 3);
    assert_eq!(request.max_events(), 2);

    let mut context = DESContext::new(42);
    let trajectory = context.run_trajectory(request);
    assert_eq!(trajectory.scheduled().len(), 3);
    assert_eq!(
        trajectory
            .dispatched()
            .iter()
            .map(|event| (event.at.ticks(), event.priority, event.kind.code()))
            .collect::<Vec<_>>(),
        vec![(5, 0, 10), (10, 0, 20)]
    );
    assert_eq!(trajectory.outcome(), StepOutcome::LimitReached);
    assert_eq!(trajectory.final_time(), SimTime::from_ticks(10));
    assert!(trajectory.limit_reached());
    assert_eq!(context.scheduler.pending_events(), 1);
}
