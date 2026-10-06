mod lineage {
    use crate::fidelity::{FidelityAdapter, FidelityError, FidelityMode, FidelityPolicy};
    use crate::{FlowRuntime, PreemptionStrategy, WorkId, WorkState};
    use kairo_ecs_types::{SimDuration, SimTime};

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
}

mod mode {
    use crate::{FlowRuntime, PreemptionStrategy, WorkId, WorkState};
    use kairo_ecs_types::{SimDuration, SimTime};

    use crate::fidelity::{
        FidelityAdapter, FidelityDecision, FidelityError, FidelityMode, FidelityPolicy,
        FidelityScope,
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
    fn policy_resolves_all_sixteen_scope_presence_masks() {
        let mut flow = FlowRuntime::new();
        let entity = flow.spawn_actor().unwrap();
        // Bits encode global, subsystem, entity, and exact entity/subsystem policy.
        // Rebuild the policy for every combination so each possible winner is checked.
        for mask in 0_u8..16 {
            let global = (mask & 0b0001 != 0).then_some(FidelityMode::Macro);
            let mut policy = FidelityPolicy::new(1, global).unwrap();
            if mask & 0b0010 != 0 {
                policy.set_subsystem("same", FidelityMode::Micro).unwrap();
            }
            if mask & 0b0100 != 0 {
                policy.set_entity(entity, FidelityMode::Macro).unwrap();
            }
            if mask & 0b1000 != 0 {
                policy
                    .set_entity_subsystem(entity, "same", FidelityMode::Micro)
                    .unwrap();
            }

            let expected = if mask & 0b1000 != 0 {
                Ok(decision(
                    FidelityMode::Micro,
                    FidelityScope::EntitySubsystem,
                ))
            } else if mask & 0b0100 != 0 {
                Ok(decision(FidelityMode::Macro, FidelityScope::Entity))
            } else if mask & 0b0010 != 0 {
                Ok(decision(FidelityMode::Micro, FidelityScope::Subsystem))
            } else if mask & 0b0001 != 0 {
                Ok(decision(FidelityMode::Macro, FidelityScope::Global))
            } else {
                Err(FidelityError::MissingPolicy)
            };
            assert_eq!(policy.resolve(entity, "same"), expected, "mask {mask:04b}");
        }
    }

    #[test]
    fn version_missing_policy_and_subsystem_identity_are_checked() {
        let mut flow = FlowRuntime::new();
        let entity = flow.spawn_actor().unwrap();
        assert_eq!(
            FidelityPolicy::new(2, None),
            Err(FidelityError::UnsupportedVersion(2))
        );

        let policy = FidelityPolicy::new(1, None).unwrap();
        assert_eq!(
            policy.resolve(entity, "unconfigured"),
            Err(FidelityError::MissingPolicy)
        );

        let mut valid_policy = FidelityPolicy::new(1, Some(FidelityMode::Macro)).unwrap();
        valid_policy
            .set_subsystem("valid", FidelityMode::Micro)
            .unwrap();
        valid_policy
            .set_entity_subsystem(entity, "valid", FidelityMode::Macro)
            .unwrap();
        let baseline = valid_policy.clone();

        for invalid in ["", " leading", "trailing ", "line\nbreak", "tab\tname"] {
            assert_eq!(
                valid_policy.resolve(entity, invalid),
                Err(FidelityError::InvalidSubsystem),
                "malformed resolution identity {invalid:?} must be rejected"
            );
            assert_eq!(
                valid_policy, baseline,
                "failed resolution must be read-only"
            );
            assert_eq!(
                valid_policy.set_subsystem(invalid, FidelityMode::Micro),
                Err(FidelityError::InvalidSubsystem),
                "subsystem identity {invalid:?} must be rejected"
            );
            assert_eq!(
                valid_policy, baseline,
                "failed subsystem update must be atomic"
            );
            assert_eq!(
                valid_policy.set_entity_subsystem(entity, invalid, FidelityMode::Micro),
                Err(FidelityError::InvalidSubsystem),
                "entity/subsystem identity {invalid:?} must be rejected"
            );
            assert_eq!(
                valid_policy, baseline,
                "failed entity/subsystem update must be atomic"
            );
        }
        let too_long = "é".repeat(513);
        assert_eq!(too_long.len(), 1026);
        assert_eq!(
            valid_policy.set_entity_subsystem(entity, &too_long, FidelityMode::Micro),
            Err(FidelityError::InvalidSubsystem)
        );
        assert_eq!(valid_policy, baseline);

        let exact_limit = "é".repeat(512);
        assert_eq!(exact_limit.len(), 1024);
        valid_policy
            .set_subsystem(&exact_limit, FidelityMode::Micro)
            .unwrap();
        assert_eq!(
            valid_policy.resolve(entity, &exact_limit).unwrap(),
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
    fn every_terminal_state_preserves_old_work_and_uses_replaced_policy_for_future_work() {
        for terminal_state in [
            WorkState::Completed,
            WorkState::Aborted,
            WorkState::Cancelled,
            WorkState::Released,
        ] {
            let mut flow = FlowRuntime::new();
            let owner = flow.spawn_actor().unwrap();
            let original = make_work(&mut flow, owner);
            let mut adapter =
                FidelityAdapter::new(FidelityPolicy::new(1, Some(FidelityMode::Macro)).unwrap());
            let original_decision = adapter.admit(&flow, original, "model.ed.v1").unwrap();
            let resource = flow.create_resource(1).unwrap();
            let initial_strategy = if terminal_state == WorkState::Aborted {
                PreemptionStrategy::Abort
            } else {
                PreemptionStrategy::Suspend
            };
            let request = flow
                .acquire(resource)
                .owner(owner)
                .at(t(0))
                .priority(9)
                .timed_work(original)
                .preemptible(initial_strategy)
                .submit()
                .unwrap();
            flow.step().unwrap().expect("original work should start");

            match terminal_state {
                WorkState::Completed => {
                    flow.step().unwrap().expect("original work should complete");
                }
                WorkState::Aborted => {
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
                        .expect("urgent work should abort original");
                }
                WorkState::Cancelled => {
                    flow.cancel(request, t(3)).unwrap();
                    flow.step().unwrap().expect("cancellation should dispatch");
                }
                WorkState::Released => {
                    let lease = flow.request(request).unwrap().lease.unwrap();
                    flow.release(lease, t(3)).unwrap();
                    flow.step().unwrap().expect("release should dispatch");
                }
                _ => unreachable!("only terminal work states are listed"),
            }
            assert_eq!(flow.work_progress(original).unwrap().state, terminal_state);

            let spec_before = flow.work(original).unwrap();
            let progress_before = flow.work_progress(original).unwrap();
            let context_before = flow.work_context::<String>(original).unwrap().clone();
            // The second staged value replaces the first future configuration.
            adapter.stage_policy(FidelityPolicy::new(1, None).unwrap());
            adapter.stage_policy(FidelityPolicy::new(1, Some(FidelityMode::Micro)).unwrap());
            adapter.apply_at_boundary(&flow).unwrap();

            assert_eq!(adapter.decision(original), Some(&original_decision));
            assert_eq!(flow.work(original).unwrap(), spec_before);
            assert_eq!(flow.work_progress(original).unwrap(), progress_before);
            assert_eq!(
                flow.work_context::<String>(original).unwrap(),
                &context_before
            );
            let applied = adapter.clone();
            assert_eq!(
                adapter.apply_at_boundary(&flow),
                Err(FidelityError::NoPendingPolicy)
            );
            assert_eq!(adapter, applied);

            let future = make_work(&mut flow, owner);
            assert_eq!(
                adapter.admit(&flow, future, "model.ed.v1").unwrap(),
                decision(FidelityMode::Micro, FidelityScope::Global)
            );
        }
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

    #[test]
    fn active_and_suspended_work_cannot_be_admitted_and_failed_admission_does_not_bind() {
        for requested_state in [WorkState::Active, WorkState::Suspended] {
            let mut busy_flow = FlowRuntime::new();
            let owner = busy_flow.spawn_actor().unwrap();
            let work = make_work(&mut busy_flow, owner);
            let resource = busy_flow.create_resource(1).unwrap();
            busy_flow
                .acquire(resource)
                .owner(owner)
                .at(t(0))
                .priority(9)
                .timed_work(work)
                .preemptible(PreemptionStrategy::Suspend)
                .submit()
                .unwrap();
            busy_flow
                .step()
                .unwrap()
                .expect("work should become active");
            if requested_state == WorkState::Suspended {
                let urgent = make_work(&mut busy_flow, owner);
                busy_flow
                    .acquire(resource)
                    .owner(owner)
                    .at(t(3))
                    .priority(1)
                    .timed_work(urgent)
                    .can_preempt(true)
                    .submit()
                    .unwrap();
                busy_flow
                    .step()
                    .unwrap()
                    .expect("urgent work should suspend the target");
            }
            assert_eq!(
                busy_flow.work_progress(work).unwrap().state,
                requested_state
            );

            let mut other_flow = FlowRuntime::new();
            let other_owner = other_flow.spawn_actor().unwrap();
            let other_work = make_work(&mut other_flow, other_owner);
            let mut adapter =
                FidelityAdapter::new(FidelityPolicy::new(1, Some(FidelityMode::Macro)).unwrap());
            let before = adapter.clone();
            let spec_before = busy_flow.work(work).unwrap();
            let progress_before = busy_flow.work_progress(work).unwrap();
            let context_before = busy_flow.work_context::<String>(work).unwrap().clone();

            assert_eq!(
                adapter.admit(&busy_flow, work, "model.ed.v1"),
                Err(FidelityError::InvalidWork),
                "{requested_state:?} work is not pending"
            );
            assert_eq!(adapter, before);
            assert_eq!(busy_flow.work(work).unwrap(), spec_before);
            assert_eq!(busy_flow.work_progress(work).unwrap(), progress_before);
            assert_eq!(
                busy_flow.work_context::<String>(work).unwrap(),
                &context_before
            );
            assert_eq!(
                adapter
                    .admit(&other_flow, other_work, "model.ed.v1")
                    .unwrap(),
                decision(FidelityMode::Macro, FidelityScope::Global),
                "failed admission must not bind the adapter to its FlowRuntime"
            );
        }
    }

    #[test]
    fn despawned_bound_work_fails_closed_without_consuming_pending_policy() {
        let mut flow = FlowRuntime::new();
        let owner = flow.spawn_actor().unwrap();
        let work = make_work(&mut flow, owner);
        let mut adapter =
            FidelityAdapter::new(FidelityPolicy::new(1, Some(FidelityMode::Macro)).unwrap());
        let admitted = adapter.admit(&flow, work, "model.ed.v1").unwrap();
        adapter.stage_policy(FidelityPolicy::new(1, Some(FidelityMode::Micro)).unwrap());
        let before = adapter.clone();

        flow.despawn_actor(owner).unwrap();
        flow.step().unwrap().expect("actor despawn should dispatch");
        assert!(flow.work(work).is_err());
        assert_eq!(
            adapter.apply_at_boundary(&flow),
            Err(FidelityError::InvalidWork)
        );
        assert_eq!(adapter, before);
        assert_eq!(adapter.decision(work), Some(&admitted));
    }

    #[test]
    fn boundary_checks_every_admitted_work_even_when_the_first_is_terminal() {
        let mut flow = FlowRuntime::new();
        let owner = flow.spawn_actor().unwrap();
        let completed_first = flow
            .create_work(owner, d(1), "fidelity.mode.c21.v1", String::from("first"))
            .unwrap();
        let active_second = make_work(&mut flow, owner);
        let mut adapter =
            FidelityAdapter::new(FidelityPolicy::new(1, Some(FidelityMode::Macro)).unwrap());
        let first_decision = adapter
            .admit(&flow, completed_first, "model.ed.v1")
            .unwrap();
        let second_decision = adapter.admit(&flow, active_second, "model.ed.v1").unwrap();

        let first_resource = flow.create_resource(1).unwrap();
        let second_resource = flow.create_resource(1).unwrap();
        flow.acquire(first_resource)
            .owner(owner)
            .at(t(0))
            .timed_work(completed_first)
            .submit()
            .unwrap();
        flow.acquire(second_resource)
            .owner(owner)
            .at(t(0))
            .timed_work(active_second)
            .submit()
            .unwrap();
        flow.step().unwrap().expect("first work should start");
        flow.step().unwrap().expect("second work should start");
        flow.step().unwrap().expect("first work should complete");
        assert_eq!(
            flow.work_progress(completed_first).unwrap().state,
            WorkState::Completed
        );
        assert_eq!(
            flow.work_progress(active_second).unwrap().state,
            WorkState::Active
        );

        adapter.stage_policy(FidelityPolicy::new(1, Some(FidelityMode::Micro)).unwrap());
        let before = adapter.clone();
        let first_spec = flow.work(completed_first).unwrap();
        let first_progress = flow.work_progress(completed_first).unwrap();
        let first_context = flow
            .work_context::<String>(completed_first)
            .unwrap()
            .clone();
        let second_spec = flow.work(active_second).unwrap();
        let second_progress = flow.work_progress(active_second).unwrap();
        let second_context = flow.work_context::<String>(active_second).unwrap().clone();

        assert_eq!(
            adapter.apply_at_boundary(&flow),
            Err(FidelityError::BusyBoundary),
            "the later active binding must block the boundary"
        );
        assert_eq!(adapter, before);
        assert_eq!(adapter.decision(completed_first), Some(&first_decision));
        assert_eq!(adapter.decision(active_second), Some(&second_decision));
        assert_eq!(flow.work(completed_first).unwrap(), first_spec);
        assert_eq!(flow.work_progress(completed_first).unwrap(), first_progress);
        assert_eq!(
            flow.work_context::<String>(completed_first).unwrap(),
            &first_context
        );
        assert_eq!(flow.work(active_second).unwrap(), second_spec);
        assert_eq!(flow.work_progress(active_second).unwrap(), second_progress);
        assert_eq!(
            flow.work_context::<String>(active_second).unwrap(),
            &second_context
        );
    }
}

mod permit {
    use crate::fidelity::{
        FidelityAdapter, FidelityAdmissionPermit, FidelityDecision, FidelityError, FidelityMode,
        FidelityPolicy, FidelityScope,
    };
    use crate::{FlowRuntime, PreemptionStrategy, WorkId, WorkState};
    use kairo_ecs_types::{EntityId, SimDuration};

    fn work(flow: &mut FlowRuntime, owner: EntityId, ticks: u128) -> WorkId {
        flow.create_work(owner, SimDuration::from_ticks(ticks), "admission.v1", ())
            .unwrap()
    }

    fn adapter() -> FidelityAdapter {
        FidelityAdapter::new(FidelityPolicy::new(1, Some(FidelityMode::Micro)).unwrap())
    }

    fn bind_error<'a>(
        result: Result<FidelityDecision, (FidelityAdmissionPermit<'a>, FidelityError)>,
    ) -> (FidelityAdmissionPermit<'a>, FidelityError) {
        match result {
            Err(error) => error,
            Ok(_) => panic!("invalid admission unexpectedly succeeded"),
        }
    }

    #[test]
    fn permit_binds_one_frozen_decision_to_actual_pending_work() {
        let mut flow = FlowRuntime::new();
        let owner = flow.spawn_actor().unwrap();
        let task = work(&mut flow, owner, 9);
        let mut adapter = adapter();
        let permit = adapter.prepare_admission(&flow, owner, "ed").unwrap();
        let decision = permit.decision();
        assert_eq!(decision.mode, FidelityMode::Micro);
        assert_eq!(decision.scope, FidelityScope::Global);
        assert_eq!(
            permit
                .bind(&flow, task, SimDuration::from_ticks(9))
                .unwrap(),
            decision
        );
        assert_eq!(adapter.decision(task), Some(&decision));
        assert_eq!(flow.work_progress(task).unwrap().state, WorkState::Pending);
    }

    #[test]
    fn invalid_actor_does_not_bind_runtime_and_valid_prepare_can_follow() {
        let mut first = FlowRuntime::new();
        let resource = first.create_resource(1).unwrap();
        let mut adapter = adapter();
        assert_eq!(
            adapter
                .prepare_admission(&first, resource.entity_id(), "ed")
                .err(),
            Some(FidelityError::InvalidWork)
        );

        let mut second = FlowRuntime::new();
        let owner = second.spawn_actor().unwrap();
        let task = work(&mut second, owner, 4);
        let permit = adapter.prepare_admission(&second, owner, "ed").unwrap();
        assert_eq!(
            permit
                .bind(&second, task, SimDuration::from_ticks(4))
                .unwrap(),
            FidelityDecision {
                mode: FidelityMode::Micro,
                scope: FidelityScope::Global,
                policy_version: 1,
            }
        );
    }

    #[test]
    fn owner_and_sampled_duration_mismatches_return_the_same_permit() {
        let mut flow = FlowRuntime::new();
        let owner = flow.spawn_actor().unwrap();
        let other = flow.spawn_actor().unwrap();
        let wrong_owner = work(&mut flow, other, 8);
        let right_owner = work(&mut flow, owner, 8);
        let mut adapter = adapter();
        let permit = adapter.prepare_admission(&flow, owner, "ed").unwrap();

        let (permit, error) =
            bind_error(permit.bind(&flow, wrong_owner, SimDuration::from_ticks(8)));
        assert_eq!(error, FidelityError::InvalidWork);
        assert_eq!(
            flow.work_progress(wrong_owner).unwrap().state,
            WorkState::Pending
        );

        let (permit, error) =
            bind_error(permit.bind(&flow, right_owner, SimDuration::from_ticks(7)));
        assert_eq!(error, FidelityError::InvalidWork);
        assert_eq!(
            flow.work_progress(right_owner).unwrap().state,
            WorkState::Pending
        );

        permit
            .bind(&flow, right_owner, SimDuration::from_ticks(8))
            .expect("failed checks preserve permit for retry");
        assert_eq!(adapter.decision(wrong_owner), None);
        assert!(adapter.decision(right_owner).is_some());
    }

    #[test]
    fn foreign_runtime_collision_rejects_and_returns_original_permit() {
        let mut original = FlowRuntime::new();
        let owner = original.spawn_actor().unwrap();
        let task = work(&mut original, owner, 5);
        let mut foreign = FlowRuntime::new();
        let foreign_owner = foreign.spawn_actor().unwrap();
        let foreign_task = work(&mut foreign, foreign_owner, 5);
        assert_eq!(owner, foreign_owner);
        assert_eq!(task, foreign_task);

        let mut adapter = adapter();
        let permit = adapter.prepare_admission(&original, owner, "ed").unwrap();
        let (permit, error) =
            bind_error(permit.bind(&foreign, foreign_task, SimDuration::from_ticks(5)));
        assert_eq!(error, FidelityError::InvalidWork);
        permit
            .bind(&original, task, SimDuration::from_ticks(5))
            .expect("foreign rejection leaves original permit available");
        assert!(adapter.decision(task).is_some());
        assert_eq!(
            foreign.work_progress(foreign_task).unwrap().state,
            WorkState::Pending
        );
    }

    #[test]
    fn non_pending_and_duplicate_work_reject_without_losing_permit() {
        let mut flow = FlowRuntime::new();
        let owner = flow.spawn_actor().unwrap();
        let active = work(&mut flow, owner, 20);
        let resource = flow.create_resource(1).unwrap();
        flow.acquire(resource)
            .owner(owner)
            .timed_work(active)
            .preemptible(PreemptionStrategy::Suspend)
            .submit()
            .unwrap();
        flow.step().unwrap().unwrap();
        assert_eq!(flow.work_progress(active).unwrap().state, WorkState::Active);

        let mut adapter = adapter();
        let permit = adapter.prepare_admission(&flow, owner, "ed").unwrap();
        let (permit, error) = bind_error(permit.bind(&flow, active, SimDuration::from_ticks(20)));
        assert_eq!(error, FidelityError::InvalidWork);
        assert_eq!(flow.work_progress(active).unwrap().state, WorkState::Active);

        let pending = work(&mut flow, owner, 3);
        permit
            .bind(&flow, pending, SimDuration::from_ticks(3))
            .unwrap();
        assert_eq!(adapter.decision(active), None);
        let duplicate = adapter.prepare_admission(&flow, owner, "ed").unwrap();
        let (duplicate, error) =
            bind_error(duplicate.bind(&flow, pending, SimDuration::from_ticks(3)));
        assert_eq!(error, FidelityError::DuplicateAdmission);
        assert_eq!(duplicate.decision().mode, FidelityMode::Micro);
    }
}
