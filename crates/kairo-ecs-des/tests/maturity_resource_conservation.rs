// SPDX-License-Identifier: Apache-2.0
use kairo_ecs_core::Scheduler;
use kairo_ecs_des::{StagedRequestState, StagedResourceRuntime, StagedRuntimeError};
use kairo_ecs_types::{EventKind, ScheduleRequest, SimTime};

#[test]
fn bounded_dispatch_preserves_ties_cancelled_actions_and_future_requests() {
    let tick = |offset| SimTime::from_ticks(u128::from(u64::MAX) + 8 + offset);
    let mut runtime = StagedResourceRuntime::new().unwrap();
    let resource = runtime.create_resource(1).unwrap();
    let [a, b, c] = std::array::from_fn(|_| runtime.spawn_actor().unwrap());
    let first = runtime.submit(resource, a, tick(1)).unwrap();
    let second = runtime.submit(resource, b, tick(1)).unwrap();
    let future = runtime.submit(resource, c, tick(5)).unwrap();

    assert_eq!(runtime.run_until_or_for(tick(0), 4), Ok(0));
    assert_eq!(runtime.run_until_or_for(tick(1), 0), Ok(0));
    assert_eq!(runtime.now(), SimTime::from_ticks(0));
    assert_eq!(runtime.next_event_time(), Some(tick(1)));

    assert_eq!(runtime.run_until_or_for(tick(1), 1), Ok(1));
    let old_lease = runtime.request(first).unwrap().lease.unwrap();
    assert_eq!(
        runtime.request(second).unwrap().state,
        StagedRequestState::Scheduled,
    );
    assert_eq!(runtime.next_event_time(), Some(tick(1)));
    assert_eq!(runtime.run_until_or_for(tick(1), 4), Ok(1));
    assert_eq!(runtime.resource(resource).unwrap().queued, [second]);
    assert_eq!(runtime.next_event_time(), Some(tick(5)));

    runtime.release(old_lease, tick(4)).unwrap();
    assert_eq!(runtime.close_request(first, tick(2)), Ok(true));
    assert_eq!(runtime.run_until_or_for(tick(2), 4), Ok(1));
    assert_eq!(runtime.close_request(first, tick(2)), Ok(false));
    assert_eq!(
        runtime.release(old_lease, tick(4)),
        Err(StagedRuntimeError::InvalidState),
    );

    // The cancelled close at tick four is ahead of the future live request.
    assert_eq!(runtime.run_until_or_for(tick(4), 4), Ok(0));
    assert_eq!(runtime.now(), tick(2));
    assert_eq!(runtime.next_event_time(), Some(tick(5)));
    let snapshot = runtime.resource(resource).unwrap();
    assert_eq!(snapshot.capacity, 1);
    assert_eq!(snapshot.available, 0);
    assert_eq!(
        snapshot.active,
        [runtime.request(second).unwrap().lease.unwrap()]
    );
    assert!(snapshot.queued.is_empty());
    assert_eq!(
        runtime.request(future).unwrap().state,
        StagedRequestState::Scheduled,
    );

    assert_eq!(runtime.run_for(1), Ok(1));
    assert_eq!(runtime.resource(resource).unwrap().queued, [future]);
}

#[test]
fn coordinated_dispatch_rejects_backdated_work_on_either_queue() {
    let tick = SimTime::from_ticks;
    let mut runtime = StagedResourceRuntime::new().unwrap();
    let resource = runtime.create_resource(1).unwrap();
    let actor = runtime.spawn_actor().unwrap();
    let mut model = Scheduler::new();
    let model_request = ScheduleRequest {
        at: tick(8),
        priority: 0,
        entity: None,
        kind: EventKind::custom(1),
    };
    model.schedule(model_request);
    assert_eq!(model.step().into_dispatched().unwrap().at, tick(8));

    let request = runtime.submit(resource, actor, tick(7)).unwrap();
    assert_eq!(
        runtime.step_with_scheduler(&mut model, tick(8)),
        Err(StagedRuntimeError::InvalidState)
    );
    assert_eq!(runtime.now(), tick(0));
    assert_eq!(model.now(), tick(8));
    assert_eq!(runtime.next_event_time(), Some(tick(7)));
    assert_eq!(
        runtime.request(request).unwrap().state,
        StagedRequestState::Scheduled
    );

    runtime.submit(resource, actor, tick(9)).unwrap();
    assert_eq!(runtime.run_until_or_for(tick(9), 2), Ok(2));
    let backdated = model.schedule(model_request);
    assert_eq!(
        runtime.step_with_scheduler(&mut model, tick(9)),
        Err(StagedRuntimeError::InvalidState)
    );
    assert_eq!(runtime.now(), tick(9));
    assert_eq!(model.now(), tick(8));
    assert_eq!(model.peek_next().unwrap().id, backdated);
}
