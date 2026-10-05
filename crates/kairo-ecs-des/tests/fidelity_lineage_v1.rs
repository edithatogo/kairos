use kairo_ecs_des::{FlowRuntime, FlowRuntimeIdentity, PreemptionStrategy, WorkId, WorkState};
use kairo_ecs_types::{SimDuration, SimTime};
#[allow(dead_code)] // This test covers the lineage subset of the private adapter.
#[path = "../src/fidelity.rs"]
mod fidelity;
use fidelity::{FidelityAdapter, FidelityError, FidelityMode, FidelityPolicy};

fn work(flow: &mut FlowRuntime) -> WorkId {
    let owner = flow.spawn_actor().unwrap();
    flow.create_work(owner, SimDuration::from_ticks(10), "lineage.v1", 42_u64)
        .unwrap()
}
fn policy(mode: Option<FidelityMode>) -> FidelityPolicy {
    FidelityPolicy::new(1, mode).unwrap()
}
fn activate(flow: &mut FlowRuntime, work: WorkId, suspended: bool) {
    let resource = flow.create_resource(1).unwrap();
    let owner = flow.work(work).unwrap().owner;
    flow.acquire(resource)
        .owner(owner)
        .timed_work(work)
        .priority(9)
        .preemptible(PreemptionStrategy::Suspend)
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    if suspended {
        let urgent = self::work(flow);
        let owner = flow.work(urgent).unwrap().owner;
        flow.acquire(resource)
            .owner(owner)
            .timed_work(urgent)
            .at(SimTime::from_ticks(3))
            .priority(1)
            .can_preempt(true)
            .submit()
            .unwrap();
        flow.step().unwrap().unwrap();
    }
}
#[test]
fn opaque_identity_survives_moves_and_clone_but_distinguishes_instances() {
    let a = FlowRuntime::new();
    let identity = a.identity();
    assert_eq!(identity, identity.clone());
    assert_ne!(identity, FlowRuntime::new().identity());
    let moved = a;
    assert_eq!(identity, moved.identity());
    assert_eq!(format!("{identity:?}"), "FlowRuntimeIdentity");
}
#[test]
fn failed_initial_admission_does_not_bind_a_runtime() {
    let mut a = FlowRuntime::new();
    let wa = work(&mut a);
    let mut adapter = FidelityAdapter::new(policy(None));
    let before = adapter.clone();
    assert_eq!(
        adapter.admit(&a, wa, "ed"),
        Err(FidelityError::MissingPolicy)
    );
    assert_eq!(adapter, before);
    let mut b = FlowRuntime::new();
    let wb = work(&mut b);
    adapter.stage_policy(policy(Some(FidelityMode::Micro)));
    adapter.apply_at_boundary(&b).unwrap();
    adapter.admit(&b, wb, "ed").unwrap();
    assert_eq!(adapter.admit(&a, wa, "ed"), Err(FidelityError::InvalidWork));
}
#[test]
fn foreign_terminal_work_never_bypasses_bound_live_work() {
    for state in [WorkState::Pending, WorkState::Active, WorkState::Suspended] {
        let mut a = FlowRuntime::new();
        let wa = work(&mut a);
        let mut b = FlowRuntime::new();
        let wb = work(&mut b);
        assert_eq!(wa, wb);
        let mut adapter = FidelityAdapter::new(policy(Some(FidelityMode::Macro)));
        let decision = adapter.admit(&a, wa, "ed").unwrap();
        if state != WorkState::Pending {
            activate(&mut a, wa, state == WorkState::Suspended);
        }
        assert_eq!(a.work_progress(wa).unwrap().state, state);
        activate(&mut b, wb, false);
        b.step().unwrap().unwrap();
        assert_eq!(b.work_progress(wb).unwrap().state, WorkState::Completed);
        adapter.stage_policy(policy(Some(FidelityMode::Micro)));
        let before = adapter.clone();
        let progress = a.work_progress(wa).unwrap();
        let context = *a.work_context::<u64>(wa).unwrap();
        assert_eq!(adapter.admit(&b, wb, "ed"), Err(FidelityError::InvalidWork));
        assert_eq!(
            adapter.apply_at_boundary(&b),
            Err(FidelityError::InvalidWork)
        );
        assert_eq!(adapter, before);
        assert_eq!(
            adapter.apply_at_boundary(&a),
            Err(FidelityError::BusyBoundary)
        );
        assert_eq!(adapter.decision(wa), Some(&decision));
        assert_eq!(a.work_progress(wa).unwrap(), progress);
        assert_eq!(*a.work_context::<u64>(wa).unwrap(), context);
    }
}
