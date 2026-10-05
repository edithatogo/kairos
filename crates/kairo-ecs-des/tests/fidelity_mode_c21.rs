use kairo_ecs_des::{FlowRuntime, PreemptionStrategy, WorkId, WorkState};
use kairo_ecs_types::{SimDuration, SimTime};

#[path = "../src/fidelity.rs"]
mod fidelity;

use fidelity::{
    FidelityAdapter, FidelityDecision, FidelityError, FidelityMode, FidelityPolicy, FidelityScope,
};

fn t(ticks: u128) -> SimTime {
    SimTime::from_ticks(ticks)
}

fn d(ticks: u128) -> SimDuration {
    SimDuration::from_ticks(ticks)
}

fn make_work(flow: &mut FlowRuntime, owner: kairo_ecs_types::EntityId) -> WorkId {
    flow.create_work(
        owner,
        d(10),
        "fidelity.mode.c21.v1",
        String::from("owned-context"),
    )
    .expect("actual Flow work should be created")
}

fn decision(mode: FidelityMode, scope: FidelityScope) -> FidelityDecision {
    FidelityDecision {
        mode,
        scope,
        policy_version: 1,
    }
}

#[test]
fn policy_resolves_every_scope_in_precedence_order() {
    let mut flow = FlowRuntime::new();
    let entity = flow.spawn_actor().unwrap();
    let subsystem_only = flow.spawn_actor().unwrap();
    let global_only = flow.spawn_actor().unwrap();

    let mut policy = FidelityPolicy::new(1, Some(FidelityMode::Macro)).unwrap();
    policy.set_entity(entity, FidelityMode::Macro).unwrap();
    policy
        .set_subsystem("entity-scope", FidelityMode::Micro)
        .unwrap();
    policy
        .set_subsystem("pair-scope", FidelityMode::Micro)
        .unwrap();
    policy
        .set_entity_subsystem(entity, "pair-scope", FidelityMode::Macro)
        .unwrap();

    assert_eq!(
        policy.resolve(entity, "pair-scope").unwrap(),
        decision(FidelityMode::Macro, FidelityScope::EntitySubsystem)
    );
    assert_eq!(
        policy.resolve(entity, "entity-scope").unwrap(),
        decision(FidelityMode::Macro, FidelityScope::Entity)
    );
    assert_eq!(
        policy.resolve(subsystem_only, "pair-scope").unwrap(),
        decision(FidelityMode::Micro, FidelityScope::Subsystem)
    );
    assert_eq!(
        policy.resolve(global_only, "global-scope").unwrap(),
        decision(FidelityMode::Macro, FidelityScope::Global)
    );
}

#[test]
fn version_missing_policy_and_subsystem_identity_are_checked() {
    let mut flow = FlowRuntime::new();
    let entity = flow.spawn_actor().unwrap();
    assert_eq!(
        FidelityPolicy::new(2, None),
        Err(FidelityError::UnsupportedVersion(2))
    );

    let mut policy = FidelityPolicy::new(1, None).unwrap();
    assert_eq!(
        policy.resolve(entity, "unconfigured"),
        Err(FidelityError::MissingPolicy)
    );

    for invalid in ["", " leading", "trailing ", "line\nbreak", "tab\tname"] {
        assert_eq!(
            policy.set_subsystem(invalid, FidelityMode::Macro),
            Err(FidelityError::InvalidSubsystem),
            "subsystem identity {invalid:?} must be rejected"
        );
    }
    let too_long = "é".repeat(513);
    assert_eq!(too_long.len(), 1026);
    assert_eq!(
        policy.set_entity_subsystem(entity, &too_long, FidelityMode::Micro),
        Err(FidelityError::InvalidSubsystem)
    );

    let exact_limit = "é".repeat(512);
    assert_eq!(exact_limit.len(), 1024);
    policy
        .set_subsystem(&exact_limit, FidelityMode::Micro)
        .unwrap();
    assert_eq!(
        policy.resolve(entity, &exact_limit).unwrap(),
        decision(FidelityMode::Micro, FidelityScope::Subsystem)
    );
}

#[test]
fn entity_overrides_use_live_generational_flow_identities() {
    let mut flow = FlowRuntime::new();
    let old_entity = flow.spawn_actor().unwrap();
    let mut policy = FidelityPolicy::new(1, None).unwrap();
    policy.set_entity(old_entity, FidelityMode::Macro).unwrap();
    policy.set_subsystem("shared", FidelityMode::Micro).unwrap();

    flow.despawn_actor(old_entity).unwrap();
    flow.step().unwrap().expect("despawn should be dispatched");
    let new_entity = flow.spawn_actor().unwrap();
    assert_eq!(old_entity.index, new_entity.index);
    assert_ne!(old_entity.generation, new_entity.generation);

    assert_eq!(
        policy.resolve(new_entity, "shared").unwrap(),
        decision(FidelityMode::Micro, FidelityScope::Subsystem)
    );
    assert_eq!(
        policy.resolve(new_entity, "entity-only"),
        Err(FidelityError::MissingPolicy)
    );
}

#[test]
fn actual_pending_active_and_suspended_work_block_policy_application() {
    for requested_state in [WorkState::Pending, WorkState::Active, WorkState::Suspended] {
        let mut flow = FlowRuntime::new();
        let owner = flow.spawn_actor().unwrap();
        let work = make_work(&mut flow, owner);
        assert_eq!(flow.work_progress(work).unwrap().state, WorkState::Pending);

        let policy = FidelityPolicy::new(1, Some(FidelityMode::Macro)).unwrap();
        let mut adapter = FidelityAdapter::new(policy);
        let admitted = adapter.admit(&flow, work, "model.ed.v1").unwrap();

        let resource = flow.create_resource(1).unwrap();
        let mut acquire = flow
            .acquire(resource)
            .owner(owner)
            .at(t(0))
            .timed_work(work);
        if requested_state == WorkState::Suspended {
            acquire = acquire.priority(9).preemptible(PreemptionStrategy::Suspend);
        }
        acquire.submit().unwrap();

        if requested_state != WorkState::Pending {
            flow.step().unwrap().expect("work should become active");
            assert_eq!(flow.work_progress(work).unwrap().state, WorkState::Active);
        }
        if requested_state == WorkState::Suspended {
            let urgent = make_work(&mut flow, owner);
            flow.acquire(resource)
                .owner(owner)
                .at(t(3))
                .priority(1)
                .timed_work(urgent)
                .can_preempt(true)
                .submit()
                .unwrap();
            flow.step()
                .unwrap()
                .expect("urgent work should suspend the bound work");
            assert_eq!(
                flow.work_progress(work).unwrap().state,
                WorkState::Suspended
            );
        }
        assert_eq!(flow.work_progress(work).unwrap().state, requested_state);

        adapter.stage_policy(FidelityPolicy::new(1, Some(FidelityMode::Micro)).unwrap());
        let spec_before = flow.work(work).unwrap();
        let progress_before = flow.work_progress(work).unwrap();
        let context_before = flow.work_context::<String>(work).unwrap().clone();
        assert_eq!(
            adapter.apply_at_boundary(&flow),
            Err(FidelityError::BusyBoundary),
            "bound {requested_state:?} work must prevent a boundary"
        );
        assert_eq!(adapter.decision(work), Some(&admitted));
        assert_eq!(flow.work(work).unwrap(), spec_before);
        assert_eq!(flow.work_progress(work).unwrap(), progress_before);
        assert_eq!(flow.work_context::<String>(work).unwrap(), &context_before);
    }
}

#[test]
fn terminal_work_keeps_its_decision_while_new_policy_applies_to_future_work() {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let original = make_work(&mut flow, owner);
    let mut adapter =
        FidelityAdapter::new(FidelityPolicy::new(1, Some(FidelityMode::Macro)).unwrap());
    let original_decision = adapter.admit(&flow, original, "model.ed.v1").unwrap();
    let resource = flow.create_resource(1).unwrap();
    flow.acquire(resource)
        .owner(owner)
        .at(t(0))
        .timed_work(original)
        .submit()
        .unwrap();
    flow.step().unwrap().expect("work should become active");
    flow.step().unwrap().expect("work should complete");
    assert_eq!(
        flow.work_progress(original).unwrap().state,
        WorkState::Completed
    );

    let spec_before = flow.work(original).unwrap();
    let progress_before = flow.work_progress(original).unwrap();
    let context_before = flow.work_context::<String>(original).unwrap().clone();
    adapter.stage_policy(FidelityPolicy::new(1, Some(FidelityMode::Micro)).unwrap());
    adapter.apply_at_boundary(&flow).unwrap();

    assert_eq!(adapter.decision(original), Some(&original_decision));
    assert_eq!(flow.work(original).unwrap(), spec_before);
    assert_eq!(flow.work_progress(original).unwrap(), progress_before);
    assert_eq!(
        flow.work_context::<String>(original).unwrap(),
        &context_before
    );

    let future = make_work(&mut flow, owner);
    assert_eq!(
        adapter.admit(&flow, future, "model.ed.v1").unwrap(),
        decision(FidelityMode::Micro, FidelityScope::Global)
    );
}

#[test]
fn failed_missing_or_duplicate_admission_preserves_the_first_decision() {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let work = make_work(&mut flow, owner);
    let mut adapter =
        FidelityAdapter::new(FidelityPolicy::new(1, Some(FidelityMode::Macro)).unwrap());
    assert_eq!(
        adapter.admit(&flow, work, "model.ed.v1"),
        Ok(decision(FidelityMode::Macro, FidelityScope::Global))
    );
    assert_eq!(
        adapter.admit(&flow, work, "model.ed.v1"),
        Err(FidelityError::DuplicateAdmission)
    );
    assert_eq!(
        adapter.decision(work),
        Some(&decision(FidelityMode::Macro, FidelityScope::Global))
    );

    let doomed_owner = flow.spawn_actor().unwrap();
    let doomed_work = make_work(&mut flow, doomed_owner);
    flow.despawn_actor(doomed_owner).unwrap();
    flow.step()
        .unwrap()
        .expect("despawn should invalidate its work");
    assert_eq!(
        adapter.admit(&flow, doomed_work, "model.ed.v1"),
        Err(FidelityError::InvalidWork)
    );
    assert_eq!(
        adapter.decision(work),
        Some(&decision(FidelityMode::Macro, FidelityScope::Global))
    );
}

#[test]
fn colliding_work_ids_cannot_cross_flow_runtime_instances() {
    let mut flow_a = FlowRuntime::new();
    let owner_a = flow_a.spawn_actor().unwrap();
    let work_a = make_work(&mut flow_a, owner_a);

    let mut flow_b = FlowRuntime::new();
    let owner_b = flow_b.spawn_actor().unwrap();
    let work_b = make_work(&mut flow_b, owner_b);
    assert_eq!(
        work_a, work_b,
        "separate runtimes can allocate colliding IDs"
    );

    let mut adapter =
        FidelityAdapter::new(FidelityPolicy::new(1, Some(FidelityMode::Macro)).unwrap());
    let original_decision = adapter.admit(&flow_a, work_a, "model.ed.v1").unwrap();
    adapter.stage_policy(FidelityPolicy::new(1, Some(FidelityMode::Micro)).unwrap());

    let spec_a_before = flow_a.work(work_a).unwrap();
    let progress_a_before = flow_a.work_progress(work_a).unwrap();
    let context_a_before = flow_a.work_context::<String>(work_a).unwrap().clone();
    let spec_b_before = flow_b.work(work_b).unwrap();
    let progress_b_before = flow_b.work_progress(work_b).unwrap();
    let context_b_before = flow_b.work_context::<String>(work_b).unwrap().clone();

    assert_eq!(
        adapter.admit(&flow_b, work_b, "model.ed.v1"),
        Err(FidelityError::InvalidWork)
    );
    assert_eq!(
        adapter.apply_at_boundary(&flow_b),
        Err(FidelityError::InvalidWork)
    );
    assert_eq!(
        adapter.apply_at_boundary(&flow_a),
        Err(FidelityError::BusyBoundary),
        "failed cross-runtime boundary must leave the staged policy pending"
    );
    assert_eq!(adapter.decision(work_a), Some(&original_decision));
    assert_eq!(flow_a.work(work_a).unwrap(), spec_a_before);
    assert_eq!(flow_a.work_progress(work_a).unwrap(), progress_a_before);
    assert_eq!(
        flow_a.work_context::<String>(work_a).unwrap(),
        &context_a_before
    );
    assert_eq!(flow_b.work(work_b).unwrap(), spec_b_before);
    assert_eq!(flow_b.work_progress(work_b).unwrap(), progress_b_before);
    assert_eq!(
        flow_b.work_context::<String>(work_b).unwrap(),
        &context_b_before
    );
}
