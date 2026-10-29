use kairo_ecs_des::{StagedRequestState, StagedResourceRuntime};
use kairo_ecs_types::SimTime;

#[test]
fn capacity_one_contention_promotes_fifo_request_after_scheduled_release() {
    let mut runtime = StagedResourceRuntime::new().unwrap();
    let actor_a = runtime.spawn_actor().unwrap();
    let actor_b = runtime.spawn_actor().unwrap();
    let resource = runtime.create_resource(1).unwrap();
    let request_a = runtime
        .submit(resource, actor_a, SimTime::from_ticks(1))
        .unwrap();
    let request_b = runtime
        .submit(resource, actor_b, SimTime::from_ticks(1))
        .unwrap();

    assert_eq!(runtime.run_for(2).unwrap(), 2);
    let snap = runtime.resource(resource).unwrap();
    assert_eq!(snap.capacity, 1);
    assert_eq!(snap.available, 0);
    assert_eq!(snap.active.len(), 1);
    assert_eq!(snap.active[0].request_id(), request_a);
    assert_eq!(snap.queued, vec![request_b]);
    assert_eq!(
        runtime.request(request_a).unwrap().state,
        StagedRequestState::Active
    );
    assert_eq!(
        runtime.request(request_b).unwrap().state,
        StagedRequestState::Queued
    );

    let lease_a = snap.active[0];
    assert_eq!(lease_a.revision(), 1);
    runtime.release(lease_a, runtime.now()).unwrap();
    assert!(
        runtime.release(lease_a, runtime.now()).is_err(),
        "duplicate pending release must fail"
    );
    assert_eq!(runtime.run_for(1).unwrap(), 1);
    assert_eq!(
        runtime.request(request_a).unwrap().state,
        StagedRequestState::Released
    );
    assert_eq!(runtime.request(request_a).unwrap().lease, None);
    let after = runtime.resource(resource).unwrap();
    assert_eq!(after.active.len(), 1);
    assert_eq!(after.active[0].request_id(), request_b);
    assert_eq!(after.queued, Vec::new());
    assert_eq!(
        runtime.request(request_b).unwrap().state,
        StagedRequestState::Active
    );
}

#[test]
fn invalid_times_and_zero_capacity_are_rejected_without_mutation() {
    let mut runtime = StagedResourceRuntime::new().unwrap();
    let actor = runtime.spawn_actor().unwrap();
    assert!(runtime.create_resource(0).is_err());
    let resource = runtime.create_resource(1).unwrap();
    let request = runtime
        .submit(resource, actor, SimTime::from_ticks(5))
        .unwrap();
    assert_eq!(runtime.run_for(1).unwrap(), 1);
    assert!(runtime
        .submit(resource, actor, SimTime::from_ticks(4))
        .is_err());
    assert_eq!(
        runtime.request(request).unwrap().state,
        StagedRequestState::Active
    );
}

#[test]
fn handles_from_another_runtime_cannot_alias_local_ids() {
    let mut first = StagedResourceRuntime::new().unwrap();
    let actor = first.spawn_actor().unwrap();
    let resource = first.create_resource(1).unwrap();
    let request = first
        .submit(resource, actor, SimTime::from_ticks(1))
        .unwrap();
    first.run_for(1).unwrap();
    let lease = first.request(request).unwrap().lease.unwrap();

    let mut second = StagedResourceRuntime::new().unwrap();
    let second_actor = second.spawn_actor().unwrap();
    let second_resource = second.create_resource(1).unwrap();
    assert_eq!(actor.entity_id(), second_actor.entity_id());
    assert_eq!(resource.entity_id(), second_resource.entity_id());
    assert!(!format!("{actor:?}").contains("runtime"));
    assert!(second
        .submit(resource, second_actor, SimTime::from_ticks(1))
        .is_err());
    assert!(second.release(lease, SimTime::from_ticks(1)).is_err());
}

#[test]
fn large_capacity_is_preserved_without_narrowing() {
    let mut runtime = StagedResourceRuntime::new().unwrap();
    let resource = runtime.create_resource(u64::MAX).unwrap();
    let snapshot = runtime.resource(resource).unwrap();
    assert_eq!(snapshot.capacity, u64::MAX);
    assert_eq!(snapshot.available, u64::MAX);
}
