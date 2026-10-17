//! Actual Flow integration tests for C2.0 Suspend/Restart with sampled work.
//! The native runner attaches this module to the calibration crate unit-test root.

#[cfg(test)]
mod tests {
    use crate::flow_bridge::{
        AcquireIntent, PreparationIdentity, SubmittedIntrinsicWork, TransitObservation,
        TransitRequest, WorkPreparationInput,
    };
    use crate::seed_map::{CalibrationSeedMap, SeedPurpose};
    use crate::work_duration::{IntrinsicDurationDistribution, IntrinsicWorkProvider};
    use kairo_ecs_abm::spatial::{
        EdgeId, MovementModeId, MovementProfile, NodeId, TransitContext, TransitEdge,
        TransitGraphV1,
    };
    use kairo_ecs_des::fidelity::{FidelityAdapter, FidelityMode, FidelityPolicy};
    use kairo_ecs_des::{
        FlowCallbackSnapshot, FlowCommandSink, FlowDispatch, FlowRuntime, LifecycleRecord,
        LifecycleTransition, PreemptionStrategy, RequestId, RequestState, ResourceId, WorkHandlers,
        WorkId, WorkProgress, WorkSpec, WorkState,
    };
    use kairo_ecs_types::{SimDuration, SimTime};
    use std::sync::Arc;

    const STUDY: &str = "study-α";
    const SCHEDULE: &str = "crn-v1";
    const CASE: &str = "case-0001";
    const TASK: &str = "triage:1";
    const STRATUM: &str = "triage";
    const WORK_REGISTRATION: &str = "c20.preemption.work";
    const MUTATE_KIND: kairo_ecs_types::EventKind = kairo_ecs_types::EventKind::custom(7_820);
    const TRANSIT_REGISTRATION: &str = "c20.preemption.transit";
    const TRANSIT_KIND: kairo_ecs_types::EventKind = kairo_ecs_types::EventKind::custom(7_821);
    const TEMPLATE_MARKER: u64 = 0xC20;
    const ATTEMPT_MARKER: u64 = 0xBAD;

    fn t(ticks: u128) -> SimTime {
        SimTime::from_ticks(ticks)
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct WorkTemplate {
        marker: u64,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct WorkContext {
        template_marker: u64,
        attempt_marker: u64,
        resumed: u32,
        restarted: u32,
    }

    fn make_work_context(template: &WorkTemplate) -> WorkContext {
        WorkContext {
            template_marker: template.marker,
            attempt_marker: template.marker,
            resumed: 0,
            restarted: 0,
        }
    }

    fn mutate_context(
        context: &mut WorkContext,
        _snapshot: &FlowCallbackSnapshot,
        _sink: &mut FlowCommandSink,
    ) {
        context.attempt_marker = ATTEMPT_MARKER;
    }

    fn on_resume(context: &mut WorkContext, _progress: &WorkProgress) {
        context.resumed += 1;
    }

    fn on_restart(context: &mut WorkContext, _progress: &WorkProgress) {
        context.restarted += 1;
    }

    fn register_work(flow: &mut FlowRuntime) {
        flow.register_work_handlers(
            WORK_REGISTRATION,
            WorkHandlers {
                on_resume: Some(on_resume),
                on_restart: Some(on_restart),
                ..WorkHandlers::default()
            },
        )
        .unwrap();
        flow.register_domain_hook(WORK_REGISTRATION, MUTATE_KIND, mutate_context)
            .unwrap();
    }

    fn provider_inputs(
        owner: kairo_ecs_types::EntityId,
        resource: ResourceId,
        strategy: PreemptionStrategy,
        mode: FidelityMode,
        transit: TransitRequest,
    ) -> (
        IntrinsicWorkProvider,
        FidelityAdapter,
        WorkPreparationInput<WorkTemplate, WorkContext>,
    ) {
        let mut seeds = CalibrationSeedMap::new(1, STUDY, 1234).unwrap();
        let expected = seeds
            .key_for(SCHEDULE, 7, CASE, TASK, SeedPurpose::Service)
            .unwrap();
        let service = seeds
            .stream_for(SCHEDULE, 7, CASE, TASK, SeedPurpose::Service)
            .unwrap();
        let provider = IntrinsicWorkProvider::new(
            1,
            vec![(
                STRATUM.to_owned(),
                IntrinsicDurationDistribution::weighted_ticks(vec![(10, 3), (20, 2), (30, 5)])
                    .unwrap(),
            )],
        )
        .unwrap();
        let adapter = FidelityAdapter::new(FidelityPolicy::new(1, Some(mode)).unwrap());
        let input = WorkPreparationInput::new(
            PreparationIdentity {
                owner,
                subsystem: STRATUM.to_owned(),
                registration: WORK_REGISTRATION.to_owned(),
                stratum: STRATUM.to_owned(),
            },
            service,
            expected,
            WorkTemplate {
                marker: TEMPLATE_MARKER,
            },
            make_work_context,
            AcquireIntent {
                resource,
                owner,
                at: t(1),
                priority_level: 0,
                deadline: None,
                scheduler_priority: 0,
                can_preempt: false,
                preemptible: Some(strategy),
            },
            transit,
        );
        (provider, adapter, input)
    }

    fn run_until_state(
        flow: &mut FlowRuntime,
        request: RequestId,
        state: RequestState,
        records: &mut Vec<LifecycleRecord>,
    ) {
        for _ in 0..64 {
            if flow.request(request).unwrap().state == state {
                return;
            }
            let dispatch = flow.step().unwrap().expect("scheduled Flow event");
            records.extend(dispatch.records);
        }
        panic!("request {request:?} did not reach {state:?}");
    }

    fn release_preemptor_and_resume(
        flow: &mut FlowRuntime,
        high_request: RequestId,
        low_request: RequestId,
        records: &mut Vec<LifecycleRecord>,
    ) {
        let high_lease = flow.request(high_request).unwrap().lease.unwrap();
        let release_at = t(flow.now().ticks() + 1);
        flow.release(high_lease, release_at).unwrap();
        run_until_state(flow, low_request, RequestState::Active, records);
    }

    fn submit_high_priority_preemptor(
        flow: &mut FlowRuntime,
        resource: ResourceId,
        high_owner: kairo_ecs_types::EntityId,
        low_request: RequestId,
        records: &mut Vec<LifecycleRecord>,
    ) -> RequestId {
        let preempt_at = t(flow.now().ticks() + 3);
        let high_request = flow
            .acquire(resource)
            .owner(high_owner)
            .at(preempt_at)
            .priority(-10)
            .can_preempt(true)
            .submit()
            .unwrap();
        run_until_state(flow, low_request, RequestState::Suspended, records);
        assert_eq!(
            flow.request(high_request).unwrap().state,
            RequestState::Active
        );
        high_request
    }

    fn schedule_context_mutation(flow: &mut FlowRuntime, work: WorkId) {
        flow.schedule_domain(work, MUTATE_KIND, t(flow.now().ticks() + 1), 0)
            .unwrap();
    }

    fn must<T, E>(result: Result<T, E>) -> T {
        match result {
            Ok(value) => value,
            Err(_) => panic!("unexpected C2.0 bridge failure"),
        }
    }

    fn assert_submitted_sample(submitted: &SubmittedIntrinsicWork<WorkTemplate, WorkContext>) {
        assert_eq!(submitted.sampled_duration(), SimDuration::from_ticks(30));
        assert_eq!(submitted.service_draw_position(), 1);
    }

    fn actual_task(
        submitted: &SubmittedIntrinsicWork<WorkTemplate, WorkContext>,
    ) -> (WorkId, RequestId) {
        (submitted.work(), submitted.request())
    }

    fn assert_flow_work(flow: &FlowRuntime, work: WorkId) -> (WorkSpec, WorkProgress) {
        let spec = flow.work(work).unwrap();
        let progress = flow.work_progress(work).unwrap();
        assert_eq!(spec.original_duration, SimDuration::from_ticks(30));
        assert_eq!(progress.original_duration, SimDuration::from_ticks(30));
        (spec, progress)
    }

    fn route_request(carrier_actor: kairo_ecs_types::EntityId) -> TransitRequest {
        let mode = MovementModeId::new("walk").unwrap();
        let profile = MovementProfile::new("walk", 1).unwrap();
        let origin = NodeId::new(1);
        let destination = NodeId::new(2);
        let graph = TransitGraphV1::new(
            1,
            vec![origin, destination],
            vec![TransitEdge {
                id: EdgeId::new(1),
                from: origin,
                to: destination,
                length_mm: 1,
                allowed_modes: vec![mode],
            }],
        )
        .unwrap();
        TransitRequest::Route {
            graph: Arc::new(graph),
            origin,
            destination,
            profile,
            ticks_per_second: 1,
            carrier_actor,
            carrier_registration: TRANSIT_REGISTRATION.to_owned(),
            kind: TRANSIT_KIND,
        }
    }

    fn complete_transit(
        flow: &mut FlowRuntime,
        mut bound: crate::flow_bridge::BoundIntrinsicWork<WorkTemplate, WorkContext>,
    ) -> SubmittedIntrinsicWork<WorkTemplate, WorkContext> {
        must(bound.start_transit(flow));
        for _ in 0..32 {
            let dispatch = flow.step().unwrap().expect("scheduled transit event");
            let observation = must(bound.observe_transit_dispatch(flow, &dispatch));
            if matches!(observation, TransitObservation::Arrived) {
                return must(bound.finish_transit(flow));
            }
        }
        panic!("actual route did not arrive");
    }

    fn register_transit(flow: &mut FlowRuntime) {
        flow.register_domain_plan_hook(TRANSIT_REGISTRATION, TRANSIT_KIND, TransitContext::plan)
            .unwrap();
    }

    #[test]
    fn suspend_preemption_resumes_remaining_work_and_preserves_completed_micro_transit() {
        let mut flow = FlowRuntime::new();
        register_work(&mut flow);
        register_transit(&mut flow);
        let owner = flow.spawn_actor().unwrap();
        let preemptor_owner = flow.spawn_actor().unwrap();
        let carrier_actor = flow.spawn_actor().unwrap();
        let resource = flow.create_resource(1).unwrap();
        let (provider, mut adapter, input) = provider_inputs(
            owner,
            resource,
            PreemptionStrategy::Suspend,
            FidelityMode::Micro,
            route_request(carrier_actor),
        );

        let prepared = must(input.prepare(&flow, &mut adapter, &provider));
        assert_eq!(prepared.sampled_duration(), SimDuration::from_ticks(30));
        assert_eq!(prepared.service_draw_position(), 1);
        let created = must(prepared.create(&mut flow));
        let work = created.work();
        let mut bound = must(created.bind(&flow));
        assert_eq!(bound.service_draw_position(), 1);

        let submitted = complete_transit(&mut flow, bound);
        assert_submitted_sample(&submitted);
        let (actual_work, request) = actual_task(&submitted);
        assert_eq!(actual_work, work);
        let (spec, _) = assert_flow_work(&flow, work);
        assert_eq!(spec.request, Some(request));

        let carrier_work = flow.actor_domain_context(carrier_actor).unwrap();
        let carrier_context = flow.work_context::<TransitContext>(carrier_work).unwrap();
        assert_eq!(carrier_context.service_work(), work);
        assert_eq!(
            carrier_context.phase(),
            kairo_ecs_abm::spatial::TransitPhase::Arrived
        );
        let transit_before = carrier_context.progress_at(flow.now()).unwrap();
        assert_eq!(
            transit_before.phase,
            kairo_ecs_abm::spatial::TransitPhase::Arrived
        );
        assert_eq!(transit_before.remaining, SimDuration::ZERO);

        let mut records = Vec::new();
        run_until_state(&mut flow, request, RequestState::Active, &mut records);
        schedule_context_mutation(&mut flow, work);
        let high_request = submit_high_priority_preemptor(
            &mut flow,
            resource,
            preemptor_owner,
            request,
            &mut records,
        );

        let progress = flow.work_progress(work).unwrap();
        assert_eq!(progress.state, WorkState::Suspended);
        assert_eq!(progress.useful_elapsed, SimDuration::from_ticks(3));
        assert_eq!(progress.remaining, SimDuration::from_ticks(27));
        assert_eq!(
            flow.actor_domain_context(carrier_actor).unwrap(),
            carrier_work
        );
        let resource_state = flow.resource(resource).unwrap();
        assert_eq!(resource_state.active.len(), 1);
        assert_eq!(resource_state.allocations.len(), 1);
        assert_eq!(resource_state.allocations[0].request, high_request);
        assert!(records.iter().any(|record| {
            record.request == request
                && record.transition == LifecycleTransition::Preempted
                && record.snapshot.progress.as_ref().is_some_and(|p| {
                    p.state == WorkState::Suspended
                        && p.useful_elapsed == SimDuration::from_ticks(3)
                        && p.remaining == SimDuration::from_ticks(27)
                })
        }));
        assert_eq!(
            flow.work_context::<WorkContext>(work)
                .unwrap()
                .attempt_marker,
            ATTEMPT_MARKER
        );
        assert_submitted_sample(&submitted);

        release_preemptor_and_resume(&mut flow, high_request, request, &mut records);
        let resumed = flow.work_progress(work).unwrap();
        assert_eq!(resumed.state, WorkState::Active);
        assert_eq!(resumed.useful_elapsed, SimDuration::from_ticks(3));
        assert_eq!(resumed.remaining, SimDuration::from_ticks(27));
        assert_eq!(
            flow.request(high_request).unwrap().state,
            RequestState::Released
        );
        let resumed_resource = flow.resource(resource).unwrap();
        assert_eq!(resumed_resource.active.len(), 1);
        assert_eq!(resumed_resource.allocations.len(), 1);
        assert_eq!(resumed_resource.allocations[0].request, request);
        let context = flow.work_context::<WorkContext>(work).unwrap();
        assert_eq!(context.template_marker, TEMPLATE_MARKER);
        assert_eq!(context.attempt_marker, ATTEMPT_MARKER);
        assert_eq!(context.resumed, 1);
        assert_eq!(context.restarted, 0);
        assert!(records.iter().any(|record| {
            record.request == request && record.transition == LifecycleTransition::Resumed
        }));
        assert_submitted_sample(&submitted);
        assert_eq!(
            flow.work_context::<TransitContext>(carrier_work)
                .unwrap()
                .progress_at(flow.now())
                .unwrap(),
            transit_before
        );

        run_until_state(&mut flow, request, RequestState::Completed, &mut records);
        let complete = flow.work_progress(work).unwrap();
        assert_eq!(complete.state, WorkState::Completed);
        assert_eq!(complete.useful_elapsed, SimDuration::from_ticks(30));
        assert_eq!(complete.remaining, SimDuration::ZERO);
        assert_eq!(flow.request(request).unwrap().work, Some(work));
        assert_eq!(flow.work(work).unwrap().request, Some(request));
        assert_submitted_sample(&submitted);

        assert_eq!(
            flow.actor_domain_context(carrier_actor).unwrap(),
            carrier_work
        );
        let carrier_after = flow.work_context::<TransitContext>(carrier_work).unwrap();
        assert_eq!(
            carrier_after.phase(),
            kairo_ecs_abm::spatial::TransitPhase::Arrived
        );
        assert_eq!(
            carrier_after.progress_at(flow.now()).unwrap(),
            transit_before
        );
        assert_eq!(carrier_after.service_work(), work);
    }

    #[test]
    fn restart_preemption_rebuilds_original_template_without_resampling_or_transit_reset() {
        let mut flow = FlowRuntime::new();
        register_work(&mut flow);
        register_transit(&mut flow);
        let owner = flow.spawn_actor().unwrap();
        let preemptor_owner = flow.spawn_actor().unwrap();
        let carrier_actor = flow.spawn_actor().unwrap();
        let resource = flow.create_resource(1).unwrap();
        let (provider, mut adapter, input) = provider_inputs(
            owner,
            resource,
            PreemptionStrategy::Restart,
            FidelityMode::Micro,
            route_request(carrier_actor),
        );

        let prepared = must(input.prepare(&flow, &mut adapter, &provider));
        assert_eq!(prepared.sampled_duration(), SimDuration::from_ticks(30));
        assert_eq!(prepared.service_draw_position(), 1);
        let created = must(prepared.create(&mut flow));
        let work = created.work();
        let bound = must(created.bind(&flow));
        assert_eq!(bound.service_draw_position(), 1);
        let submitted = complete_transit(&mut flow, bound);
        assert_submitted_sample(&submitted);
        let (actual_work, request) = actual_task(&submitted);
        assert_eq!(actual_work, work);
        assert_eq!(flow.work(work).unwrap().request, Some(request));
        assert_flow_work(&flow, work);
        let carrier_work = flow.actor_domain_context(carrier_actor).unwrap();
        let carrier = flow.work_context::<TransitContext>(carrier_work).unwrap();
        assert_eq!(carrier.service_work(), work);
        assert_eq!(
            carrier.phase(),
            kairo_ecs_abm::spatial::TransitPhase::Arrived
        );
        let transit_before = carrier.progress_at(flow.now()).unwrap();
        assert_eq!(transit_before.remaining, SimDuration::ZERO);

        let mut records = Vec::new();
        run_until_state(&mut flow, request, RequestState::Active, &mut records);
        schedule_context_mutation(&mut flow, work);
        let high_request = submit_high_priority_preemptor(
            &mut flow,
            resource,
            preemptor_owner,
            request,
            &mut records,
        );

        let suspended = flow.work_progress(work).unwrap();
        assert_eq!(suspended.state, WorkState::Suspended);
        assert_eq!(suspended.useful_elapsed, SimDuration::from_ticks(3));
        assert_eq!(suspended.remaining, SimDuration::from_ticks(27));
        assert_eq!(
            flow.actor_domain_context(carrier_actor).unwrap(),
            carrier_work
        );
        let resource_state = flow.resource(resource).unwrap();
        assert_eq!(resource_state.active.len(), 1);
        assert_eq!(resource_state.allocations.len(), 1);
        assert_eq!(resource_state.allocations[0].request, high_request);
        assert_eq!(
            flow.work_context::<WorkContext>(work)
                .unwrap()
                .attempt_marker,
            ATTEMPT_MARKER
        );
        assert_submitted_sample(&submitted);
        assert_eq!(
            flow.work_context::<TransitContext>(carrier_work)
                .unwrap()
                .progress_at(flow.now())
                .unwrap(),
            transit_before
        );

        release_preemptor_and_resume(&mut flow, high_request, request, &mut records);
        let restarted = flow.work_progress(work).unwrap();
        assert_eq!(restarted.state, WorkState::Active);
        assert_eq!(restarted.attempt_revision, 1);
        assert_eq!(restarted.useful_elapsed, SimDuration::ZERO);
        assert_eq!(restarted.remaining, SimDuration::from_ticks(30));
        assert_eq!(
            flow.request(high_request).unwrap().state,
            RequestState::Released
        );
        let restarted_resource = flow.resource(resource).unwrap();
        assert_eq!(restarted_resource.active.len(), 1);
        assert_eq!(restarted_resource.allocations.len(), 1);
        assert_eq!(restarted_resource.allocations[0].request, request);
        let context = flow.work_context::<WorkContext>(work).unwrap();
        assert_eq!(context.template_marker, TEMPLATE_MARKER);
        assert_eq!(context.attempt_marker, TEMPLATE_MARKER);
        assert_eq!(context.resumed, 0);
        assert_eq!(context.restarted, 1);
        assert!(records.iter().any(|record| {
            record.request == request && record.transition == LifecycleTransition::Restarted
        }));
        assert_submitted_sample(&submitted);
        assert_eq!(
            flow.actor_domain_context(carrier_actor).unwrap(),
            carrier_work
        );
        assert_eq!(
            flow.work_context::<TransitContext>(carrier_work)
                .unwrap()
                .progress_at(flow.now())
                .unwrap(),
            transit_before
        );

        run_until_state(&mut flow, request, RequestState::Completed, &mut records);
        let complete = flow.work_progress(work).unwrap();
        assert_eq!(complete.state, WorkState::Completed);
        assert_eq!(complete.useful_elapsed, SimDuration::from_ticks(30));
        assert_eq!(complete.cumulative_busy, SimDuration::from_ticks(33));
        assert_eq!(complete.attempt_revision, 1);
        assert_eq!(
            flow.work(work).unwrap().original_duration,
            SimDuration::from_ticks(30)
        );
        assert_eq!(flow.request(request).unwrap().work, Some(work));
        assert_submitted_sample(&submitted);
        assert_eq!(
            flow.actor_domain_context(carrier_actor).unwrap(),
            carrier_work
        );
        let carrier_after = flow.work_context::<TransitContext>(carrier_work).unwrap();
        assert_eq!(
            carrier_after.phase(),
            kairo_ecs_abm::spatial::TransitPhase::Arrived
        );
        assert_eq!(
            carrier_after.progress_at(flow.now()).unwrap(),
            transit_before
        );
        assert_eq!(carrier_after.service_work(), work);
    }
}
