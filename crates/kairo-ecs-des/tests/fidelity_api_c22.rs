use kairo_ecs_des::fidelity::{
    FidelityAdapter, FidelityDecision, FidelityError, FidelityMode, FidelityPolicy, FidelityScope,
};
use kairo_ecs_des::{FlowRuntime, WorkState};
use kairo_ecs_types::SimDuration;

#[test]
fn external_fidelity_api_controls_actual_flow_policy_boundary() {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().expect("actor should be created");

    let mut initial_policy =
        FidelityPolicy::new(1, Some(FidelityMode::Macro)).expect("policy version is supported");
    initial_policy
        .set_entity_subsystem(owner, "ed.smoke.v1", FidelityMode::Micro)
        .expect("subsystem identity should be accepted");
    let mut adapter = FidelityAdapter::new(initial_policy);

    let original = flow
        .create_work(
            owner,
            SimDuration::from_ticks(2),
            "fidelity-api.c22.v1",
            42_u32,
        )
        .expect("actual Flow work should be created pending");
    let original_decision = adapter
        .admit(&flow, original, "ed.smoke.v1")
        .expect("pending work should be admitted");
    assert_eq!(
        original_decision,
        FidelityDecision {
            mode: FidelityMode::Micro,
            scope: FidelityScope::EntitySubsystem,
            policy_version: 1,
        }
    );
    assert_eq!(
        adapter.admit(&flow, original, "ed.smoke.v1"),
        Err(FidelityError::DuplicateAdmission)
    );
    assert_eq!(adapter.decision(original), Some(&original_decision));

    let resource = flow.create_resource(1).expect("resource should be created");
    flow.acquire(resource)
        .owner(owner)
        .timed_work(original)
        .submit()
        .expect("actual work request should be submitted");
    flow.step()
        .expect("Flow start dispatch should succeed")
        .expect("submitted work should start");
    assert_eq!(flow.work_progress(original).unwrap().state, WorkState::Active);

    adapter.stage_policy(FidelityPolicy::new(1, Some(FidelityMode::Macro)).unwrap());
    assert_eq!(
        adapter.apply_at_boundary(&flow),
        Err(FidelityError::BusyBoundary)
    );
    assert_eq!(adapter.decision(original), Some(&original_decision));

    flow.step()
        .expect("Flow completion dispatch should succeed")
        .expect("started work should complete");
    assert_eq!(
        flow.work_progress(original).unwrap().state,
        WorkState::Completed
    );
    adapter
        .apply_at_boundary(&flow)
        .expect("terminal work allows the staged boundary");
    assert_eq!(adapter.decision(original), Some(&original_decision));

    let future = flow
        .create_work(
            owner,
            SimDuration::from_ticks(2),
            "fidelity-api.c22.v1",
            43_u32,
        )
        .expect("future Flow work should be created");
    assert_eq!(
        adapter.admit(&flow, future, "ed.smoke.v1"),
        Ok(FidelityDecision {
            mode: FidelityMode::Macro,
            scope: FidelityScope::Global,
            policy_version: 1,
        })
    );
}
