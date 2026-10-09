use kairo_ecs_des::fidelity::{
    FidelityAdapter, FidelityCheckpointError, FidelityCheckpointLimits, FidelityDecision,
    FidelityError, FidelityMode, FidelityPolicy, FidelityScope,
};
use kairo_ecs_des::{FlowRuntime, PreemptionStrategy, WorkId, WorkState};
use kairo_ecs_types::{SimDuration, SimTime};

fn policy(mode: FidelityMode) -> FidelityPolicy {
    FidelityPolicy::new(1, Some(mode)).unwrap()
}

fn work(flow: &mut FlowRuntime, duration: u128) -> WorkId {
    let actor = flow.spawn_actor().unwrap();
    flow.create_work(
        actor,
        SimDuration::from_ticks(duration),
        "checkpoint.v1",
        7_u64,
    )
    .unwrap()
}

fn limits() -> FidelityCheckpointLimits {
    FidelityCheckpointLimits {
        max_admitted: 8,
        max_overrides: 16,
        max_subsystem_bytes: 1024,
    }
}

#[test]
fn checkpoint_keeps_frozen_mixed_decisions_and_pending_policy_after_rebind() {
    let mut source = FlowRuntime::new();
    let macro_work = work(&mut source, 10);
    let micro_work = work(&mut source, 10);
    let micro_actor = source.work(micro_work).unwrap().owner;
    let mut current = policy(FidelityMode::Macro);
    current
        .set_entity_subsystem(micro_actor, "triage", FidelityMode::Micro)
        .unwrap();
    let pending = policy(FidelityMode::Micro);
    let mut adapter = FidelityAdapter::new(current);
    let macro_decision = adapter.admit(&source, macro_work, "triage").unwrap();
    let micro_decision = adapter.admit(&source, micro_work, "triage").unwrap();
    adapter.stage_policy(pending);

    let image = adapter.checkpoint(&source, limits()).unwrap();
    let mut restored_flow = FlowRuntime::new();
    let new_macro_work = work(&mut restored_flow, 10);
    let new_micro_work = work(&mut restored_flow, 10);
    assert_ne!(source.identity(), restored_flow.identity());
    assert_eq!(macro_work.entity_id(), new_macro_work.entity_id());
    assert_eq!(micro_work.entity_id(), new_micro_work.entity_id());
    let mut restored = FidelityAdapter::from_checkpoint(
        image,
        &restored_flow,
        &[(macro_work, new_macro_work), (micro_work, new_micro_work)],
        limits(),
    )
    .unwrap();

    assert_eq!(restored.decision(new_macro_work), Some(&macro_decision));
    assert_eq!(restored.decision(new_micro_work), Some(&micro_decision));
    assert_eq!(macro_decision.mode, FidelityMode::Macro);
    assert_eq!(micro_decision.mode, FidelityMode::Micro);
    assert_eq!(micro_decision.scope, FidelityScope::EntitySubsystem);
    assert_eq!(macro_decision.policy_version, 1);
    assert_eq!(micro_decision.policy_version, 1);
    assert_eq!(
        restored.apply_at_boundary(&restored_flow),
        Err(FidelityError::BusyBoundary)
    );
    assert!(restored
        .checkpoint(&restored_flow, limits())
        .unwrap()
        .pending
        .is_some());
    assert_eq!(
        restored.decision(new_macro_work),
        Some(&FidelityDecision {
            mode: FidelityMode::Macro,
            scope: FidelityScope::Global,
            policy_version: 1,
        })
    );
}

#[test]
fn active_and_suspended_work_survive_rebind_and_still_block_policy_boundary() {
    for suspended in [false, true] {
        let mut source = FlowRuntime::new();
        let first = work(&mut source, 10);
        let actor = source.work(first).unwrap().owner;
        let mut adapter = FidelityAdapter::new(policy(FidelityMode::Macro));
        let decision = adapter.admit(&source, first, "ed").unwrap();
        let resource = source.create_resource(1).unwrap();
        source
            .acquire(resource)
            .owner(actor)
            .timed_work(first)
            .priority(9)
            .preemptible(PreemptionStrategy::Suspend)
            .submit()
            .unwrap();
        source.step().unwrap().unwrap();
        adapter.stage_policy(policy(FidelityMode::Micro));
        let second = if suspended {
            let urgent = work(&mut source, 1);
            let actor = source.work(urgent).unwrap().owner;
            source
                .acquire(resource)
                .owner(actor)
                .timed_work(urgent)
                .at(SimTime::from_ticks(3))
                .priority(1)
                .can_preempt(true)
                .submit()
                .unwrap();
            source.step().unwrap().unwrap();
            assert_eq!(
                source.work_progress(first).unwrap().state,
                WorkState::Suspended
            );
            Some(urgent)
        } else {
            assert_eq!(
                source.work_progress(first).unwrap().state,
                WorkState::Active
            );
            None
        };
        let image = adapter.checkpoint(&source, limits()).unwrap();

        let mut restored_flow = FlowRuntime::new();
        let rebound = work(&mut restored_flow, 10);
        let rebound_actor = restored_flow.work(rebound).unwrap().owner;
        let rebound_resource = restored_flow.create_resource(1).unwrap();
        restored_flow
            .acquire(rebound_resource)
            .owner(rebound_actor)
            .timed_work(rebound)
            .priority(9)
            .preemptible(PreemptionStrategy::Suspend)
            .submit()
            .unwrap();
        restored_flow.step().unwrap().unwrap();
        if second.is_some() {
            let urgent = work(&mut restored_flow, 1);
            let actor = restored_flow.work(urgent).unwrap().owner;
            restored_flow
                .acquire(rebound_resource)
                .owner(actor)
                .timed_work(urgent)
                .at(SimTime::from_ticks(3))
                .priority(1)
                .can_preempt(true)
                .submit()
                .unwrap();
            restored_flow.step().unwrap().unwrap();
            assert_eq!(
                restored_flow.work_progress(rebound).unwrap().state,
                WorkState::Suspended
            );
        }
        let mut restored =
            FidelityAdapter::from_checkpoint(image, &restored_flow, &[(first, rebound)], limits())
                .unwrap();
        assert_eq!(restored.decision(rebound), Some(&decision));
        assert_eq!(
            restored.apply_at_boundary(&restored_flow),
            Err(FidelityError::BusyBoundary)
        );
    }
}

#[test]
fn capture_checks_lineage_and_import_requires_exact_work_mapping() {
    let mut source = FlowRuntime::new();
    let admitted = work(&mut source, 5);
    let admitted_second = work(&mut source, 7);
    let mut adapter = FidelityAdapter::new(policy(FidelityMode::Macro));
    adapter.admit(&source, admitted, "ed").unwrap();
    adapter.admit(&source, admitted_second, "ed").unwrap();
    let other = FlowRuntime::new();
    assert_eq!(
        adapter.checkpoint(&other, limits()),
        Err(FidelityCheckpointError::WrongRuntime)
    );
    let image = adapter.checkpoint(&source, limits()).unwrap();
    let mut target = FlowRuntime::new();
    let rebound = work(&mut target, 5);
    let rebound_second = work(&mut target, 7);
    let target_before = target.work_progress(rebound).unwrap();
    assert_eq!(
        FidelityAdapter::from_checkpoint(image.clone(), &target, &[], limits()),
        Err(FidelityCheckpointError::MissingMapping)
    );
    assert_eq!(
        FidelityAdapter::from_checkpoint(
            image.clone(),
            &target,
            &[(admitted, rebound), (admitted, rebound_second)],
            limits(),
        ),
        Err(FidelityCheckpointError::DuplicateMapping)
    );
    assert!(target.work(rebound).is_ok());
    assert!(target.work(rebound_second).is_ok());
    assert_eq!(
        FidelityAdapter::from_checkpoint(
            image.clone(),
            &target,
            &[(admitted, rebound_second), (admitted_second, rebound)],
            limits(),
        ),
        Err(FidelityCheckpointError::InvalidWork)
    );
    assert_eq!(
        FidelityAdapter::from_checkpoint(
            image.clone(),
            &target,
            &[(admitted, rebound), (admitted_second, rebound)],
            limits(),
        ),
        Err(FidelityCheckpointError::DuplicateMapping)
    );
    let mut incomplete_target = FlowRuntime::new();
    let incomplete_rebound = work(&mut incomplete_target, 5);
    assert_eq!(
        FidelityAdapter::from_checkpoint(
            image.clone(),
            &incomplete_target,
            &[
                (admitted, incomplete_rebound),
                (admitted_second, admitted_second)
            ],
            limits(),
        ),
        Err(FidelityCheckpointError::InvalidWork)
    );
    assert_eq!(target.work_progress(rebound).unwrap(), target_before);
}

#[test]
fn stale_generation_mapping_is_rejected_without_mutating_target_flow() {
    let mut source = FlowRuntime::new();
    let original = work(&mut source, 5);
    let mut adapter = FidelityAdapter::new(policy(FidelityMode::Macro));
    adapter.admit(&source, original, "ed").unwrap();
    let image = adapter.checkpoint(&source, limits()).unwrap();

    let mut target = FlowRuntime::new();
    let stale = work(&mut target, 5);
    let owner = target.work(stale).unwrap().owner;
    target.despawn_actor(owner).unwrap();
    target.step().unwrap().unwrap();
    assert!(target.work(stale).is_err());
    assert_eq!(
        FidelityAdapter::from_checkpoint(image, &target, &[(original, stale)], limits()),
        Err(FidelityCheckpointError::InvalidWork)
    );
}

#[test]
fn valid_reused_index_with_new_generation_cannot_receive_old_decision() {
    let mut source = FlowRuntime::new();
    let original = work(&mut source, 5);
    let mut adapter = FidelityAdapter::new(policy(FidelityMode::Macro));
    adapter.admit(&source, original, "ed").unwrap();
    let image = adapter.checkpoint(&source, limits()).unwrap();

    let mut target = FlowRuntime::new();
    let first_generation = work(&mut target, 5);
    let first_owner = target.work(first_generation).unwrap().owner;
    target.despawn_actor(first_owner).unwrap();
    target.step().unwrap().unwrap();
    let next_owner = target.spawn_actor().unwrap();
    let next_generation = target
        .create_work(
            next_owner,
            SimDuration::from_ticks(5),
            "checkpoint.v1",
            7_u64,
        )
        .unwrap();
    assert_eq!(
        first_generation.entity_id().index,
        next_generation.entity_id().index
    );
    assert_ne!(
        first_generation.entity_id().generation,
        next_generation.entity_id().generation
    );
    assert!(target.work(next_generation).is_ok());
    assert_eq!(
        FidelityAdapter::from_checkpoint(image, &target, &[(original, next_generation)], limits(),),
        Err(FidelityCheckpointError::InvalidWork)
    );
}

#[test]
fn restore_rejects_impossible_bound_state_empty_or_missing_binding() {
    let source = FlowRuntime::new();
    let target = FlowRuntime::new();
    let unbound = FidelityAdapter::new(policy(FidelityMode::Macro));
    let mut bound_empty = unbound.checkpoint(&source, limits()).unwrap();
    assert!(!bound_empty.bound_runtime);
    assert!(bound_empty.admitted.is_empty());
    bound_empty.bound_runtime = true;
    assert_eq!(
        FidelityAdapter::from_checkpoint(bound_empty, &target, &[], limits()),
        Err(FidelityCheckpointError::InvalidWork)
    );

    let mut source_with_work = FlowRuntime::new();
    let admitted = work(&mut source_with_work, 5);
    let mut adapter = FidelityAdapter::new(policy(FidelityMode::Macro));
    adapter.admit(&source_with_work, admitted, "ed").unwrap();
    let mut missing_binding = adapter.checkpoint(&source_with_work, limits()).unwrap();
    assert!(missing_binding.bound_runtime);
    assert_eq!(missing_binding.admitted.len(), 1);
    missing_binding.bound_runtime = false;
    assert_eq!(
        FidelityAdapter::from_checkpoint(
            missing_binding,
            &target,
            &[(admitted, admitted)],
            limits(),
        ),
        Err(FidelityCheckpointError::InvalidWork)
    );
}

#[test]
fn checkpoint_limits_and_invalid_canonical_policy_data_reject_atomically() {
    let mut source = FlowRuntime::new();
    let admitted = work(&mut source, 5);
    let mut adapter = FidelityAdapter::new(policy(FidelityMode::Macro));
    adapter.admit(&source, admitted, "ed").unwrap();
    assert_eq!(
        adapter.checkpoint(
            &source,
            FidelityCheckpointLimits {
                max_admitted: 0,
                ..limits()
            }
        ),
        Err(FidelityCheckpointError::LimitExceeded)
    );
    let mut image = adapter.checkpoint(&source, limits()).unwrap();
    image.current.version = 2;
    let mut target = FlowRuntime::new();
    let rebound = work(&mut target, 5);
    assert_eq!(
        FidelityAdapter::from_checkpoint(image, &target, &[(admitted, rebound)], limits()),
        Err(FidelityCheckpointError::UnsupportedVersion(2))
    );
}

#[test]
fn future_actor_overrides_and_unbound_adapter_remain_unbound() {
    let mut source = FlowRuntime::new();
    let future_actor = source.spawn_actor().unwrap();
    source.despawn_actor(future_actor).unwrap();
    source.step().unwrap().unwrap();
    let mut configured = policy(FidelityMode::Macro);
    configured
        .set_entity(future_actor, FidelityMode::Micro)
        .unwrap();
    let adapter = FidelityAdapter::new(configured);
    let image = adapter.checkpoint(&source, limits()).unwrap();
    assert!(!image.bound_runtime);

    let target = FlowRuntime::new();
    let restored = FidelityAdapter::from_checkpoint(image, &target, &[], limits()).unwrap();
    let rebound_image = restored.checkpoint(&target, limits()).unwrap();
    assert!(!rebound_image.bound_runtime);
    assert_eq!(
        rebound_image.current.entity_overrides,
        vec![(future_actor, FidelityMode::Micro)]
    );
}

#[test]
fn terminal_work_keeps_decision_and_allows_pending_policy_after_rebind() {
    fn complete(flow: &mut FlowRuntime, work: WorkId) {
        let owner = flow.work(work).unwrap().owner;
        let resource = flow.create_resource(1).unwrap();
        flow.acquire(resource)
            .owner(owner)
            .timed_work(work)
            .submit()
            .unwrap();
        flow.step().unwrap().unwrap();
        flow.step().unwrap().unwrap();
        assert_eq!(
            flow.work_progress(work).unwrap().state,
            WorkState::Completed
        );
    }

    let mut source = FlowRuntime::new();
    let original = work(&mut source, 4);
    let mut adapter = FidelityAdapter::new(policy(FidelityMode::Macro));
    let decision = adapter.admit(&source, original, "ed").unwrap();
    adapter.stage_policy(policy(FidelityMode::Micro));
    complete(&mut source, original);
    let image = adapter.checkpoint(&source, limits()).unwrap();

    let mut target = FlowRuntime::new();
    let rebound = work(&mut target, 4);
    complete(&mut target, rebound);
    let mut restored =
        FidelityAdapter::from_checkpoint(image, &target, &[(original, rebound)], limits()).unwrap();
    assert_eq!(
        target.work_progress(rebound).unwrap().state,
        WorkState::Completed
    );
    assert_eq!(restored.decision(rebound), Some(&decision));
    restored.apply_at_boundary(&target).unwrap();
    assert_eq!(restored.decision(rebound), Some(&decision));
}

#[test]
fn subsystem_byte_budget_and_canonical_order_are_checked_before_restore() {
    let source = FlowRuntime::new();
    let mut configured = policy(FidelityMode::Macro);
    configured
        .set_subsystem("alpha", FidelityMode::Micro)
        .unwrap();
    configured
        .set_subsystem("omega", FidelityMode::Macro)
        .unwrap();
    let adapter = FidelityAdapter::new(configured);
    assert_eq!(
        adapter.checkpoint(
            &source,
            FidelityCheckpointLimits {
                max_subsystem_bytes: 1,
                ..limits()
            }
        ),
        Err(FidelityCheckpointError::LimitExceeded)
    );

    let image = adapter.checkpoint(&source, limits()).unwrap();
    let mut reversed = image.clone();
    reversed.current.subsystem_overrides.reverse();
    let target = FlowRuntime::new();
    assert_eq!(
        FidelityAdapter::from_checkpoint(reversed, &target, &[], limits()),
        Err(FidelityCheckpointError::NonCanonical)
    );
    let mut malformed = image;
    malformed.current.subsystem_overrides[0].0 = " invalid".to_owned();
    assert_eq!(
        FidelityAdapter::from_checkpoint(malformed, &target, &[], limits()),
        Err(FidelityCheckpointError::InvalidPolicy)
    );
}
