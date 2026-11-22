use kairo_ecs_des::{
    StagedActorId, StagedRequestState, StagedResourceId, StagedResourceRuntime, StagedRuntimeError,
    StagedUsageSource,
};
use kairo_ecs_types::SimTime;
use std::num::NonZeroU128;

fn tick(value: u128) -> SimTime {
    SimTime::from_ticks(value)
}

fn fixture(capacity: u64) -> (StagedResourceRuntime, StagedResourceId, StagedActorId) {
    let mut runtime = StagedResourceRuntime::new_with_usage(StagedUsageSource {
        run: "resource-cleanup".into(),
        scenario: "synthetic".into(),
        branch: "base".into(),
        engine_revision: "fixture".into(),
        model_revision: "fixture".into(),
        rng_revision: "deterministic".into(),
        time_unit: "minute".into(),
        ticks_per_time_unit: NonZeroU128::MIN,
    })
    .unwrap();
    let resource = runtime
        .create_resource_with_unit(capacity, "porter")
        .unwrap();
    let actor = runtime.spawn_actor().unwrap();
    (runtime, resource, actor)
}

#[test]
fn withdrawal_before_submission_leaves_no_dispatch_or_usage() {
    let (mut runtime, resource, actor) = fixture(1);
    let request = runtime.submit(resource, actor, tick(7)).unwrap();
    let (mut foreign, _, _) = fixture(1);
    assert_eq!(
        foreign.close_request(request, tick(1)),
        Err(StagedRuntimeError::UnknownRequest)
    );
    assert_eq!(runtime.close_request(request, tick(3)), Ok(true));
    assert_eq!(runtime.close_request(request, tick(4)), Ok(false));
    assert_eq!(runtime.close_request(request, tick(2)), Ok(true));
    assert_eq!(
        runtime.request(request).unwrap().state,
        StagedRequestState::Scheduled
    );
    assert_eq!(runtime.next_event_time(), Some(tick(2)));
    assert_eq!(runtime.run_for(5), Ok(1));
    assert_eq!(runtime.close_request(request, tick(1)), Ok(false));
    assert_eq!(
        runtime.request(request).unwrap().state,
        StagedRequestState::Released
    );
    assert_eq!(runtime.next_event_time(), None);
    assert_eq!(runtime.resource(resource).unwrap().available, 1);
    let output = runtime.committed_usage(resource, tick(0), tick(2)).unwrap();
    assert_eq!(output.occupied_unit_ticks, 0);
    assert!(output.occupancy.is_empty());
}

#[test]
fn withdrawing_fifo_head_promotes_next_whole_request_at_the_close_tick() {
    let (mut runtime, resource, actor) = fixture(3);
    let first = runtime
        .submit_quantity(resource, actor, tick(0), 2)
        .unwrap();
    let head = runtime
        .submit_quantity(resource, actor, tick(1), 2)
        .unwrap();
    let next = runtime.submit(resource, actor, tick(1)).unwrap();
    let tail = runtime.submit(resource, actor, tick(1)).unwrap();
    assert_eq!(runtime.run_for(4), Ok(4));
    assert_eq!(
        runtime.resource(resource).unwrap().queued,
        [head, next, tail]
    );
    assert_eq!(runtime.close_request(tail, tick(2)), Ok(true));
    assert_eq!(runtime.close_request(head, tick(3)), Ok(true));
    assert_eq!(runtime.run_for(1), Ok(1));
    let waiting = runtime.resource(resource).unwrap();
    assert_eq!(waiting.queued, [head, next]);
    assert_eq!(waiting.available, 1);
    assert_eq!(runtime.run_for(1), Ok(1));
    let active = runtime.resource(resource).unwrap();
    assert_eq!(active.available, 0);
    assert!(active.queued.is_empty());
    assert_eq!(runtime.request(head).unwrap().lease, None);
    assert_eq!(
        runtime.request(next).unwrap().state,
        StagedRequestState::Active
    );
    assert_eq!(runtime.close_request(first, tick(5)), Ok(true));
    assert_eq!(runtime.close_request(next, tick(5)), Ok(true));
    assert_eq!(runtime.run_for(5), Ok(2));
    assert_eq!(runtime.resource(resource).unwrap().available, 3);
    let window = runtime.committed_usage(resource, tick(0), tick(5)).unwrap();
    assert_eq!(window.occupied_unit_ticks, 12);
    assert_eq!(window.occupancy.len(), 2);
    assert_eq!(window.occupancy[1].request, next.entity_id());
    assert_eq!(window.occupancy[1].admission.at, tick(3));
}

#[test]
fn earlier_close_replaces_release_and_returns_quantity_once() {
    let (mut runtime, resource, actor) = fixture(3);
    let request = runtime
        .submit_quantity(resource, actor, tick(1), 3)
        .unwrap();
    assert_eq!(runtime.run_for(1), Ok(1));
    let lease = runtime.request(request).unwrap().lease.unwrap();
    runtime.release(lease, tick(9)).unwrap();
    let sealed = runtime.committed_usage(resource, tick(0), tick(1)).unwrap();
    assert_eq!(
        runtime.close_request(request, tick(1)),
        Err(StagedRuntimeError::InvalidReleaseTime)
    );
    assert_eq!(runtime.next_event_time(), Some(tick(9)));
    assert_eq!(runtime.close_request(request, tick(4)), Ok(true));
    assert_eq!(runtime.close_request(request, tick(4)), Ok(false));
    assert_eq!(
        runtime.release(lease, tick(3)),
        Err(StagedRuntimeError::InvalidState)
    );
    assert_eq!(runtime.run_for(5), Ok(1));
    assert_eq!(runtime.now(), tick(4));
    assert_eq!(runtime.next_event_time(), None);
    assert_eq!(runtime.close_request(request, tick(8)), Ok(false));
    let free = runtime.resource(resource).unwrap();
    assert_eq!(free.available, 3);
    assert!(free.active.is_empty());
    let window = runtime.committed_usage(resource, tick(0), tick(4)).unwrap();
    assert_eq!(window.occupied_unit_ticks, 9);
    assert_eq!(window.occupancy.len(), 1);
    assert_eq!(window.occupancy[0].release.unwrap().at, tick(4));
    assert_eq!(
        runtime.committed_usage(resource, tick(0), tick(1)).unwrap(),
        sealed
    );
}

#[test]
fn closing_at_submission_tick_uses_insertion_order_and_zero_occupancy() {
    let (mut runtime, resource, actor) = fixture(1);
    let request = runtime.submit(resource, actor, tick(2)).unwrap();
    assert_eq!(runtime.close_request(request, tick(2)), Ok(true));
    assert_eq!(runtime.run_for(1), Ok(1));
    assert_eq!(
        runtime.request(request).unwrap().state,
        StagedRequestState::Active
    );
    assert_eq!(
        runtime.committed_usage(resource, tick(0), tick(2)),
        Err(StagedRuntimeError::InvalidUsageWindow)
    );
    assert_eq!(runtime.run_for(1), Ok(1));
    assert_eq!(runtime.resource(resource).unwrap().available, 1);
    assert!(runtime
        .committed_usage(resource, tick(0), tick(2))
        .unwrap()
        .occupancy
        .is_empty());
}
