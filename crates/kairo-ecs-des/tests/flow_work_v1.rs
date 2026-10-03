use kairo_ecs_des::{FlowError, FlowRuntime, RequestState};
use kairo_ecs_types::{SimDuration, SimTime};
fn t() -> SimTime {
    SimTime::from_ticks(0)
}
#[test]
fn work_owns_typed_context_and_rejects_wrong_types() {
    let mut f = FlowRuntime::new();
    let a = f.spawn_actor().unwrap();
    let w = f
        .create_work(
            a,
            SimDuration::from_ticks(10),
            "model.test.v1",
            String::from("owned"),
        )
        .unwrap();
    assert_eq!(
        f.work(w).unwrap().original_duration,
        SimDuration::from_ticks(10)
    );
    assert_eq!(f.work_context::<String>(w).unwrap(), "owned");
    assert_eq!(f.work_context::<u32>(w), Err(FlowError::InvalidWork));
    assert_eq!(
        f.create_work(a, SimDuration::from_ticks(1), "model.test.v1", 42u32),
        Err(FlowError::InvalidWork)
    );
}
#[test]
fn work_association_is_checked_and_not_implicitly_reused() {
    let mut f = FlowRuntime::new();
    let a = f.spawn_actor().unwrap();
    let b = f.spawn_actor().unwrap();
    let r = f.create_resource(1).unwrap();
    let w = f
        .create_work(a, SimDuration::from_ticks(3), "model.test.v1", 7u32)
        .unwrap();
    assert_eq!(f.submit_work(r, b, w, t()), Err(FlowError::InvalidWork));
    assert!(f.work(w).unwrap().request.is_none());
    let q = f.submit_work(r, a, w, t()).unwrap();
    assert_eq!(f.request(q).unwrap().work, Some(w));
    assert_eq!(f.work(w).unwrap().request, Some(q));
    assert_eq!(f.submit_work(r, a, w, t()), Err(FlowError::InvalidWork));
    f.step().unwrap();
    let lease = f.request(q).unwrap().lease.unwrap();
    f.release(lease, t()).unwrap();
    f.step().unwrap();
    assert_eq!(f.request(q).unwrap().state, RequestState::Released);
    assert_eq!(f.submit_work(r, a, w, t()), Err(FlowError::InvalidWork));
}
#[test]
fn actor_cleanup_drops_context_and_recycled_work_id_is_stale() {
    use std::{cell::Cell, rc::Rc};
    struct Owned(Rc<Cell<u32>>);
    impl Drop for Owned {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1)
        }
    }
    let dropped = Rc::new(Cell::new(0));
    let mut f = FlowRuntime::new();
    let a = f.spawn_actor().unwrap();
    let w = f
        .create_work(
            a,
            SimDuration::from_ticks(2),
            "drop.v1",
            Owned(dropped.clone()),
        )
        .unwrap();
    f.despawn_actor(a).unwrap();
    f.step().unwrap();
    assert_eq!(dropped.get(), 1);
    assert_eq!(f.work(w), Err(FlowError::InvalidWork));
    let b = f.spawn_actor().unwrap();
    let replacement = f
        .create_work(
            b,
            SimDuration::from_ticks(2),
            "drop.v1",
            Owned(dropped.clone()),
        )
        .unwrap();
    assert_ne!(w, replacement);
    assert!(f.work_context::<Owned>(w).is_err());
}

#[test]
fn manual_allocation_inspection_preserves_owner_work_and_grant_time() {
    let mut f = FlowRuntime::new();
    let owner = f.spawn_actor().unwrap();
    let resource = f.create_resource(1).unwrap();
    let work = f
        .create_work(owner, SimDuration::from_ticks(4), "inspect.v1", 42u32)
        .unwrap();
    let request = f
        .submit_work(resource, owner, work, SimTime::from_ticks(3))
        .unwrap();
    f.step().unwrap();
    let snapshot = f.resource(resource).unwrap();
    let allocation = &snapshot.allocations[0];
    assert_eq!(allocation.owner, owner);
    assert_eq!(allocation.request, request);
    assert_eq!(allocation.work, Some(work));
    assert_eq!(allocation.granted_at, SimTime::from_ticks(3));
    assert_eq!(allocation.segment_started_at, allocation.granted_at);
    assert_eq!(allocation.completion_at, None);
    assert_eq!(allocation.lease, snapshot.active[0]);
}
