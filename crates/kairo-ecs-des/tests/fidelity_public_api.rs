use kairo_ecs_des::fidelity::{
    FidelityAdapter, FidelityDecision, FidelityError, FidelityMode, FidelityPolicy, FidelityScope,
};
use kairo_ecs_des::FlowRuntime;
use kairo_ecs_types::SimDuration;

#[test]
fn downstream_policy_import_resolves_declared_precedence() {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let mut policy = FidelityPolicy::new(1, Some(FidelityMode::Macro)).unwrap();
    policy.set_subsystem("ed", FidelityMode::Micro).unwrap();
    assert_eq!(
        policy.resolve(owner, "ed").unwrap(),
        FidelityDecision {
            mode: FidelityMode::Micro,
            scope: FidelityScope::Subsystem,
            policy_version: 1,
        }
    );

    policy.set_entity(owner, FidelityMode::Macro).unwrap();
    assert_eq!(
        policy.resolve(owner, "ed").unwrap().scope,
        FidelityScope::Entity
    );
    policy
        .set_entity_subsystem(owner, "ed", FidelityMode::Micro)
        .unwrap();
    assert_eq!(
        policy.resolve(owner, "ed").unwrap().scope,
        FidelityScope::EntitySubsystem
    );
}

#[test]
fn downstream_import_can_prepare_and_bind_actual_pending_flow_work() {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let duration = SimDuration::from_ticks(30);
    let work = flow
        .create_work(owner, duration, "public-fidelity", 17_u64)
        .unwrap();
    let policy = FidelityPolicy::new(1, Some(FidelityMode::Micro)).unwrap();
    let mut adapter = FidelityAdapter::new(policy);

    let permit = adapter
        .prepare_admission(&flow, owner, "ed")
        .expect("configured actor policy resolves");
    assert_eq!(permit.decision().mode, FidelityMode::Micro);
    assert_eq!(
        permit.bind(&flow, work, duration).unwrap(),
        FidelityDecision {
            mode: FidelityMode::Micro,
            scope: FidelityScope::Global,
            policy_version: 1,
        }
    );
    assert_eq!(
        flow.work_progress(work).unwrap().state,
        kairo_ecs_des::WorkState::Pending
    );
    assert_eq!(
        adapter.decision(work),
        Some(&FidelityDecision {
            mode: FidelityMode::Micro,
            scope: FidelityScope::Global,
            policy_version: 1,
        })
    );
    assert_eq!(
        adapter.prepare_admission(&flow, owner, "").err(),
        Some(FidelityError::InvalidSubsystem)
    );
}
