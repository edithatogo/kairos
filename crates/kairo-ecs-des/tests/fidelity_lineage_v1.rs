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
    let fixture = include_str!("../../../conformance/c21/flow-runtime-identity-v1.tsv");
    let mut rows = fixture.lines();
    assert_eq!(
        rows.next(),
        Some("case_id\tbound_state\tforeign_state\tforeign_error\tbound_error")
    );
    let cases: Vec<_> = rows.collect();
    assert_eq!(
        cases.len(),
        3,
        "all three nonterminal states must be covered"
    );
    let mut states = std::collections::BTreeSet::new();
    for row in cases {
        let columns: Vec<_> = row.split('\t').collect();
        assert_eq!(columns.len(), 5);
        assert!(
            states.insert(columns[1]),
            "duplicate bound state in fixture"
        );
        let state = match columns[1] {
            "Pending" => WorkState::Pending,
            "Active" => WorkState::Active,
            "Suspended" => WorkState::Suspended,
            other => panic!("unsupported fixture state: {other}"),
        };
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
        assert_eq!(
            format!("{:?}", b.work_progress(wb).unwrap().state),
            columns[2]
        );
        adapter.stage_policy(policy(Some(FidelityMode::Micro)));
        let before = adapter.clone();
        let progress = a.work_progress(wa).unwrap();
        let context = *a.work_context::<u64>(wa).unwrap();
        let foreign_progress = b.work_progress(wb).unwrap();
        let foreign_context = *b.work_context::<u64>(wb).unwrap();
        assert_eq!(
            format!("{:?}", adapter.admit(&b, wb, "ed").unwrap_err()),
            columns[3]
        );
        assert_eq!(
            format!("{:?}", adapter.apply_at_boundary(&b).unwrap_err()),
            columns[3]
        );
        assert_eq!(adapter, before);
        assert_eq!(b.work_progress(wb).unwrap(), foreign_progress);
        assert_eq!(*b.work_context::<u64>(wb).unwrap(), foreign_context);
        assert_eq!(
            format!("{:?}", adapter.apply_at_boundary(&a).unwrap_err()),
            columns[4]
        );
        assert_eq!(adapter.decision(wa), Some(&decision));
        assert_eq!(a.work_progress(wa).unwrap(), progress);
        assert_eq!(*a.work_context::<u64>(wa).unwrap(), context);
    }
    assert_eq!(
        states.into_iter().collect::<Vec<_>>(),
        vec!["Active", "Pending", "Suspended"]
    );
}
