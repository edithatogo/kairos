//! Equal-priority FIFO migration: legacy synchronous helper and buffered Flow leases.
use kairo_ecs_des::{
    FlowError, FlowRuntime, LifecycleRecord, LifecycleTransition, RequestState, Resource,
};
use kairo_ecs_types::{EntityId, SimTime};
fn drain(runtime: &mut FlowRuntime) -> Vec<LifecycleRecord> {
    let mut records = Vec::new();
    for _ in 0..32 {
        match runtime.step().expect("valid scheduled command") {
            None => return records,
            Some(dispatch) => {
                assert!(dispatch.error.is_none(), "{:?}", dispatch.error);
                records.extend(dispatch.records);
            }
        }
    }
    panic!("bounded example did not drain");
}
fn main() {
    let labels = ["first", "second", "third"];
    // Synthetic legacy IDs belong only to this helper, not to the Flow world.
    let legacy_ids = [
        EntityId::new(1, 0),
        EntityId::new(2, 0),
        EntityId::new(3, 0),
    ];
    let mut legacy = Resource::new("single server", 1);
    assert!(legacy.request(legacy_ids[0]));
    assert!(!legacy.request(legacy_ids[1]));
    assert!(!legacy.request(legacy_ids[2]));
    assert_eq!(
        [
            legacy_ids[0],
            legacy.release().unwrap(),
            legacy.release().unwrap()
        ],
        legacy_ids
    );
    assert_eq!(legacy.release(), None);
    assert_eq!(legacy.available_count(), 1);
    println!(
        "Legacy FIFO: {} -> {} -> {}",
        labels[0], labels[1], labels[2]
    );

    let mut flow = FlowRuntime::new();
    let owners = [
        flow.spawn_actor().unwrap(),
        flow.spawn_actor().unwrap(),
        flow.spawn_actor().unwrap(),
    ];
    let resource = flow.create_resource(1).unwrap();
    let requests = owners.map(|owner| flow.acquire(resource).owner(owner).submit().unwrap());
    for request in requests {
        assert_eq!(flow.request(request).unwrap().state, RequestState::Pending);
    }
    let mut records = drain(&mut flow);
    assert_eq!(
        flow.request(requests[0]).unwrap().state,
        RequestState::Active
    );
    assert_eq!(
        flow.request(requests[1]).unwrap().state,
        RequestState::Queued
    );
    assert_eq!(
        flow.request(requests[2]).unwrap().state,
        RequestState::Queued
    );
    for request in requests {
        let lease = flow
            .request(request)
            .unwrap()
            .lease
            .expect("active grant has actual lease");
        flow.release(lease, SimTime::ZERO).unwrap();
        // Ingress buffers the command: state changes only when dispatch commits.
        assert_eq!(flow.request(request).unwrap().state, RequestState::Active);
        records.extend(drain(&mut flow));
        assert_eq!(flow.request(request).unwrap().state, RequestState::Released);
        assert_eq!(
            flow.release(lease, SimTime::ZERO),
            Err(FlowError::InvalidLease)
        );
    }
    let granted = records
        .iter()
        .filter(|r| r.transition == LifecycleTransition::Granted)
        .map(|r| r.request)
        .collect::<Vec<_>>();
    assert_eq!(granted, requests);
    assert!(flow.resource(resource).unwrap().active.is_empty());
    println!("Flow FIFO: {} -> {} -> {}", labels[0], labels[1], labels[2]);
    println!("Flow commands commit at dispatch; released leases cannot be reused.");
}
