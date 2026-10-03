use kairo_ecs_des::{FlowError, FlowRuntime, RequestState};
use kairo_ecs_types::SimTime;
fn t(n: u128) -> SimTime {
    SimTime::from_ticks(n)
}

#[test]
fn closed_resource_waits_then_capacity_growth_grants() {
    let mut f = FlowRuntime::new();
    let a = f.spawn_actor().unwrap();
    let r = f.create_resource(0).unwrap();
    let q = f.submit(r, a, t(0)).unwrap();
    f.step().unwrap();
    assert_eq!(f.request(q).unwrap().state, RequestState::Queued);
    assert_eq!(f.resource(r).unwrap().available, 0);
    f.set_capacity(r, 1).unwrap();
    assert_eq!(f.resource(r).unwrap().total, 0);
    f.step().unwrap();
    assert_eq!(f.request(q).unwrap().state, RequestState::Active);
}

#[test]
fn shrink_and_busy_removal_are_transactional() {
    let mut f = FlowRuntime::new();
    let a = f.spawn_actor().unwrap();
    let r = f.create_resource(1).unwrap();
    let q = f.submit(r, a, t(0)).unwrap();
    f.step().unwrap();
    let before = f.resource(r).unwrap();
    assert_eq!(f.set_capacity(r, 0), Err(FlowError::CapacityInUse));
    assert_eq!(f.remove_resource(r), Err(FlowError::ResourceInUse));
    assert_eq!(f.resource(r).unwrap(), before);
    f.release(f.request(q).unwrap().lease.unwrap(), t(0))
        .unwrap();
    f.step().unwrap();
    f.remove_resource(r).unwrap();
    f.step().unwrap();
    assert_eq!(f.resource(r), Err(FlowError::InvalidResource));
    assert_eq!(f.request(q).unwrap().state, RequestState::Released);
}

#[test]
fn past_submission_has_no_partial_state_and_run_budget_preserves_work() {
    let mut f = FlowRuntime::new();
    let a = f.spawn_actor().unwrap();
    let r = f.create_resource(1).unwrap();
    f.submit(r, a, t(5)).unwrap();
    assert!(f.run_for(0).unwrap().budget_exhausted);
    f.step().unwrap();
    let before = f.resource(r).unwrap();
    assert_eq!(f.submit(r, a, t(4)), Err(FlowError::PastCommand));
    assert_eq!(f.resource(r).unwrap(), before);
    assert!(f.step().unwrap().is_none());
}

#[test]
fn dispatch_revalidates_capacity_change_without_partial_mutation() {
    let mut f = FlowRuntime::new();
    let a = f.spawn_actor().unwrap();
    let r = f.create_resource(1).unwrap();
    let q = f.submit(r, a, t(0)).unwrap();
    f.set_capacity(r, 0).unwrap();
    f.step().unwrap();
    let before = f.resource(r).unwrap();
    let outcome = f.step().unwrap().unwrap();
    assert_eq!(outcome.error, Some(FlowError::CapacityInUse));
    assert_eq!(f.resource(r).unwrap(), before);
    assert_eq!(f.request(q).unwrap().state, RequestState::Active);
}
