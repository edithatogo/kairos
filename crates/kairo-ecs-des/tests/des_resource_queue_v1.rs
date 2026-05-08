use kairo_ecs_des::{DESContext, Resource, TrajectoryRequest, TrajectoryStep};
use kairo_ecs_types::{EntityId, EventKind, SimTime};

#[test]
fn des_resource_queue_v1_preserves_fifo_admission() {
    let mut resource = Resource::new("single-server", 1);
    let first = EntityId::new(1, 0);
    let second = EntityId::new(2, 0);
    let third = EntityId::new(3, 0);

    assert!(resource.request(first));
    assert!(!resource.request(second));
    assert!(!resource.request(third));
    assert_eq!(resource.queue_length(), 2);

    assert_eq!(resource.release(), Some(second));
    assert_eq!(resource.release(), Some(third));
    assert_eq!(resource.release(), None);
    assert_eq!(resource.available_count(), 1);
}

#[test]
fn des_resource_queue_v1_replays_fixed_tick_trajectory_order() {
    let request = TrajectoryRequest::new(3)
        .with_step(TrajectoryStep::new(
            SimTime::from_ticks(20),
            0,
            None,
            EventKind::custom(20),
        ))
        .with_step(TrajectoryStep::new(
            SimTime::from_ticks(10),
            0,
            None,
            EventKind::custom(10),
        ))
        .with_step(TrajectoryStep::new(
            SimTime::from_ticks(20),
            -1,
            None,
            EventKind::custom(19),
        ));

    let mut context = DESContext::new(42);
    let trajectory = context.run_trajectory(request);
    let replayed = trajectory
        .dispatched()
        .iter()
        .map(|event| (event.at.ticks(), event.priority, event.kind.code()))
        .collect::<Vec<_>>();

    assert_eq!(replayed, vec![(10, 0, 10), (20, -1, 19), (20, 0, 20)]);
    assert_eq!(trajectory.final_time(), SimTime::from_ticks(20));
    assert!(!trajectory.limit_reached());
}
