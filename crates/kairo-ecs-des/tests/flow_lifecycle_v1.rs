use kairo_ecs_des::{FlowError, FlowRuntime, RequestState};
use kairo_ecs_types::SimTime;

fn t(n: u128) -> SimTime {
    SimTime::from_ticks(n)
}

#[test]
fn admission_is_buffered_and_duplicate_release_cannot_free_twice() {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource = flow.create_resource(1).unwrap();
    let request = flow.submit(resource, owner, t(0)).unwrap();
    assert_eq!(flow.request(request).unwrap().state, RequestState::Pending);
    let grant = flow.step().unwrap().unwrap();
    assert_eq!(grant.records.len(), 2);
    let lease = flow.request(request).unwrap().lease.unwrap();
    flow.release(lease, t(0)).unwrap();
    assert_eq!(flow.release(lease, t(0)), Err(FlowError::InvalidLease));
    flow.step().unwrap().unwrap();
    assert_eq!(flow.request(request).unwrap().state, RequestState::Released);
    assert_eq!(flow.release(lease, t(0)), Err(FlowError::InvalidLease));
    assert_eq!(flow.resource(resource).unwrap().active.len(), 0);
}

#[test]
fn recycled_owner_never_releases_a_new_lease() {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource = flow.create_resource(1).unwrap();
    let old_request = flow.submit(resource, owner, t(0)).unwrap();
    flow.step().unwrap();
    let old = flow.request(old_request).unwrap().lease.unwrap();
    flow.despawn_actor(owner).unwrap();
    flow.step().unwrap();
    let new_owner = flow.spawn_actor().unwrap();
    assert_eq!(owner.index, new_owner.index);
    assert_ne!(owner.generation, new_owner.generation);
    assert_eq!(
        flow.submit(resource, owner, t(0)),
        Err(FlowError::InvalidEntity)
    );
    let next = flow.submit(resource, new_owner, t(0)).unwrap();
    flow.step().unwrap();
    assert_eq!(flow.release(old, t(0)), Err(FlowError::InvalidLease));
    assert_eq!(
        flow.request(old_request).unwrap().state,
        RequestState::Cancelled
    );
    assert_eq!(
        flow.resource(resource).unwrap().active,
        vec![flow.request(next).unwrap().lease.unwrap()]
    );
}

#[test]
fn despawn_cancels_all_owned_work_before_granting_other_actors() {
    let mut flow = FlowRuntime::new();
    let a = flow.spawn_actor().unwrap();
    let b = flow.spawn_actor().unwrap();
    let r = flow.create_resource(1).unwrap();
    let first = flow.submit(r, a, t(0)).unwrap();
    let second = flow.submit(r, a, t(0)).unwrap();
    let third = flow.submit(r, b, t(0)).unwrap();
    flow.run_for(3).unwrap();
    flow.despawn_actor(a).unwrap();
    let dispatch = flow.step().unwrap().unwrap();
    assert_eq!(flow.request(first).unwrap().state, RequestState::Cancelled);
    assert_eq!(flow.request(second).unwrap().state, RequestState::Cancelled);
    assert_eq!(flow.request(third).unwrap().state, RequestState::Active);
    assert_eq!(dispatch.records.len(), 3);
    for (i, row) in dispatch.records.iter().enumerate() {
        assert_eq!(row.causal_event_id, dispatch.event);
        assert_eq!(row.transition_ordinal, i as u32);
    }
}

#[test]
fn stale_pending_admission_is_not_mistaken_for_idle() {
    let mut flow = FlowRuntime::new();
    let a = flow.spawn_actor().unwrap();
    let b = flow.spawn_actor().unwrap();
    let r = flow.create_resource(1).unwrap();
    flow.submit(r, a, t(5)).unwrap();
    flow.despawn_actor(a).unwrap();
    let other = flow.submit(r, b, t(6)).unwrap();
    flow.step().unwrap();
    let stale = flow.step().unwrap().unwrap();
    assert_eq!(stale.error, Some(FlowError::TerminalRequest));
    assert_eq!(flow.run_for(1).unwrap().dispatches.len(), 1);
    assert_eq!(flow.request(other).unwrap().state, RequestState::Active);
}
