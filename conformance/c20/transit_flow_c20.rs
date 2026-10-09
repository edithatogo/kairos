use crate::flow_bridge::{
    AcquireIntent, BridgeError, PreparationIdentity, TransitObservation, TransitRequest,
    WorkPreparationInput,
};
use crate::seed_map::{CalibrationSeedMap, SeedPurpose};
use crate::work_duration::{IntrinsicDurationDistribution, IntrinsicWorkProvider};
use kairo_ecs_abm::spatial::{
    EdgeId, MovementModeId, MovementProfile, NodeId, TransitEdge, TransitGraphV1,
};
use kairo_ecs_abm::{register_transit_context, TransitContext, TransitPhase};
use kairo_ecs_des::fidelity::{FidelityAdapter, FidelityDecision, FidelityMode, FidelityPolicy};
use kairo_ecs_des::{
    FlowDomainControl, FlowError, FlowRuntime, LifecycleTransition, WorkId, WorkState,
};
use kairo_ecs_types::{EventKind, SimDuration, SimTime};
use std::sync::Arc;

const REGISTRATION: &str = "paired-transit-work";
const TASK: &str = "triage:1";
const KIND: EventKind = EventKind::custom(0xC20);

#[derive(Clone, Debug, Eq, PartialEq)]
struct Template(u64);

#[derive(Clone, Debug, Eq, PartialEq)]
struct Context(u64);

fn make_context(template: &Template) -> Context {
    Context(template.0)
}

fn transit_graph() -> Arc<TransitGraphV1> {
    let node = |value| NodeId::new(value);
    let mode = MovementModeId::new("walk").unwrap();
    let edges = (0..3)
        .map(|index| TransitEdge {
            id: EdgeId::new(index + 1),
            from: node(index + 1),
            to: node(index + 2),
            length_mm: 1,
            allowed_modes: vec![mode.clone()],
        })
        .collect();
    Arc::new(TransitGraphV1::new(1, vec![node(1), node(2), node(3), node(4)], edges).unwrap())
}

fn stream_and_provider() -> (
    crate::seed_map::CalibrationStream,
    crate::seed_map::CalibrationStreamKey,
    IntrinsicWorkProvider,
) {
    let mut seeds = CalibrationSeedMap::new(1, "study-α", 1234).unwrap();
    let expected = seeds
        .key_for("crn-v1", 7, "case-0001", TASK, SeedPurpose::Service)
        .unwrap();
    let stream = seeds
        .stream_for("crn-v1", 7, "case-0001", TASK, SeedPurpose::Service)
        .unwrap();
    let provider = IntrinsicWorkProvider::new(
        1,
        vec![(
            "triage".to_owned(),
            IntrinsicDurationDistribution::weighted_ticks(vec![(10, 3), (20, 2), (30, 5)]).unwrap(),
        )],
    )
    .unwrap();
    (stream, expected, provider)
}

fn accepted(dispatch: &kairo_ecs_des::FlowDispatch) -> Vec<kairo_ecs_des::FlowCommandAdmission> {
    match dispatch.callback_batches.as_slice() {
        [kairo_ecs_des::FlowBatchReceipt::Accepted(admissions)] => admissions.clone(),
        other => panic!("expected accepted transit batch, got {other:?}"),
    }
}

fn dispatch_and_observe(
    flow: &mut FlowRuntime,
    bound: &mut crate::flow_bridge::BoundIntrinsicWork<Template, Context>,
) -> kairo_ecs_des::FlowDispatch {
    let dispatch = flow.step().unwrap().expect("scheduled Flow event");
    let observation = bound
        .observe_transit_dispatch(flow, &dispatch)
        .unwrap_or_else(|_| panic!("actual transit dispatch observation failed"));
    assert!(!matches!(observation, TransitObservation::Rejected));
    dispatch
}

fn no_request_admissions(dispatch: &kairo_ecs_des::FlowDispatch) {
    for admission in accepted(dispatch) {
        assert_eq!(
            admission.request, None,
            "transit scheduling is not an acquire"
        );
    }
}

fn owned_control(
    flow: &mut FlowRuntime,
    bound: &mut crate::flow_bridge::BoundIntrinsicWork<Template, Context>,
    action: FlowDomainControl,
    at: SimTime,
    priority: i32,
) -> kairo_ecs_types::EventId {
    bound
        .schedule_transit_control(flow, action, at, priority)
        .unwrap_or_else(|_| panic!("bound transit control ingress failed"))
}

fn assert_invalid_dispatch(
    flow: &FlowRuntime,
    bound: &mut crate::flow_bridge::BoundIntrinsicWork<Template, Context>,
    dispatch: &kairo_ecs_des::FlowDispatch,
) {
    assert!(matches!(
        bound.observe_transit_dispatch(flow, dispatch),
        Err(BridgeError::InvalidDispatch)
    ));
}

fn assert_bound_retained(
    flow: &FlowRuntime,
    bound: &crate::flow_bridge::BoundIntrinsicWork<Template, Context>,
    work: WorkId,
    decision: FidelityDecision,
    duration: SimDuration,
    draw_position: u64,
) {
    assert_eq!(bound.decision(), decision);
    assert_eq!(flow.work(work).unwrap().original_duration, duration);
    assert!(bound.service_identity_matches());
    assert_eq!(bound.service_draw_position(), draw_position);
}

#[test]
fn actual_nonzero_route_consumes_stale_start_and_arrival_across_pause_resume_once() {
    let mut flow = FlowRuntime::new();
    register_transit_context(&mut flow, "paired-transit-plan", KIND).unwrap();
    flow.register_work_continuations(
        REGISTRATION,
        kairo_ecs_des::FlowContinuations::<Context>::default(),
    )
    .unwrap();
    let owner = flow.spawn_actor().unwrap();
    let carrier_actor = flow.spawn_actor().unwrap();
    let resource = flow.create_resource(1).unwrap();
    let (service, expected, provider) = stream_and_provider();
    let mut fidelity =
        FidelityAdapter::new(FidelityPolicy::new(1, Some(FidelityMode::Micro)).unwrap());
    let route = transit_graph()
        .route(
            NodeId::new(1),
            NodeId::new(4),
            &MovementProfile::new("walk", 3).unwrap(),
            1,
        )
        .unwrap_or_else(|_| panic!("canonical positive route construction failed"));
    assert_eq!(route.duration(), SimDuration::from_ticks(1));
    assert_eq!(
        route
            .segments()
            .iter()
            .map(|segment| segment.end_offset().ticks())
            .collect::<Vec<_>>(),
        vec![1, 1, 1]
    );
    let input = WorkPreparationInput::new(
        PreparationIdentity {
            owner,
            subsystem: "triage".to_owned(),
            registration: REGISTRATION.to_owned(),
            stratum: "triage".to_owned(),
        },
        service,
        expected,
        Template(0xC20),
        make_context,
        AcquireIntent {
            resource,
            owner,
            at: SimTime::from_ticks(5),
            priority_level: 0,
            deadline: None,
            scheduler_priority: 0,
            can_preempt: false,
            preemptible: None,
        },
        TransitRequest::Route {
            graph: transit_graph(),
            origin: NodeId::new(1),
            destination: NodeId::new(4),
            profile: MovementProfile::new("walk", 3).unwrap(),
            ticks_per_second: 1,
            carrier_actor,
            carrier_registration: "paired-transit-plan".to_owned(),
            kind: KIND,
        },
    );
    let prepared = input
        .prepare(&flow, &mut fidelity, &provider)
        .unwrap_or_else(|_| panic!("actual provider/Flow preparation failed"));
    assert_eq!(prepared.sampled_duration(), SimDuration::from_ticks(30));
    assert_eq!(prepared.service_draw_position(), 1);
    let frozen_decision = prepared.decision();
    let frozen_duration = prepared.sampled_duration();
    let frozen_draw_bounds = prepared.draw_bounds();
    assert_eq!(frozen_decision.mode, FidelityMode::Micro);
    assert_eq!(frozen_draw_bounds, (0, 1));
    let created = prepared
        .create(&mut flow)
        .unwrap_or_else(|_| panic!("actual restartable Flow work creation failed"));
    let work = created.work();
    assert_eq!(flow.work(work).unwrap().original_duration, frozen_duration);
    let mut bound = created
        .bind(&flow)
        .unwrap_or_else(|_| panic!("actual Flow work binding failed"));
    assert_eq!(bound.decision(), frozen_decision);
    assert!(bound.service_identity_matches());
    assert_eq!(bound.service_draw_position(), frozen_draw_bounds.1);
    assert_bound_retained(
        &flow,
        &bound,
        work,
        frozen_decision,
        frozen_duration,
        frozen_draw_bounds.1,
    );

    let first_start = bound.start_transit(&mut flow).unwrap();
    let mut foreign_flow = FlowRuntime::new();
    let owner_flow_before = flow.budget_snapshot();
    let foreign_before = foreign_flow.budget_snapshot();
    assert!(matches!(
        bound.schedule_transit_control(
            &mut foreign_flow,
            FlowDomainControl::Pause,
            SimTime::from_ticks(2),
            0
        ),
        Err(BridgeError::Fidelity(
            kairo_ecs_des::fidelity::FidelityError::InvalidWork
        ))
    ));
    assert_eq!(foreign_flow.budget_snapshot(), foreign_before);
    assert_eq!(flow.budget_snapshot(), owner_flow_before);

    let first_pause = owned_control(
        &mut flow,
        &mut bound,
        FlowDomainControl::Pause,
        SimTime::from_ticks(2),
        0,
    );
    let paused_ready = dispatch_and_observe(&mut flow, &mut bound);
    assert_eq!(paused_ready.event, first_pause);
    assert_bound_retained(
        &flow,
        &bound,
        work,
        frozen_decision,
        frozen_duration,
        frozen_draw_bounds.1,
    );
    no_request_admissions(&paused_ready);
    assert_invalid_dispatch(&flow, &mut bound, &paused_ready);

    let budget_while_paused = flow.budget_snapshot();
    assert!(matches!(
        bound.schedule_transit_control(
            &mut flow,
            FlowDomainControl::Pause,
            SimTime::from_ticks(1),
            0
        ),
        Err(BridgeError::Flow(FlowError::PastCommand))
    ));
    assert_eq!(flow.budget_snapshot(), budget_while_paused);
    let carrier_work = flow.actor_domain_context(carrier_actor).unwrap();
    assert_eq!(
        flow.work_context::<TransitContext>(carrier_work)
            .unwrap()
            .phase(),
        TransitPhase::Paused
    );
    let progress_before_repeated_pause = flow
        .work_context::<TransitContext>(carrier_work)
        .unwrap()
        .progress_at(flow.now())
        .unwrap();

    let unowned_event = flow
        .schedule_domain_control(
            carrier_work,
            KIND,
            FlowDomainControl::Pause,
            SimTime::from_ticks(3),
            0,
        )
        .unwrap();
    let unowned_pause = flow.step().unwrap().expect("external Flow control event");
    assert_eq!(unowned_pause.event, unowned_event);
    assert_bound_retained(
        &flow,
        &bound,
        work,
        frozen_decision,
        frozen_duration,
        frozen_draw_bounds.1,
    );
    assert_invalid_dispatch(&flow, &mut bound, &unowned_pause);
    assert!(matches!(
        unowned_pause.callback_batches.as_slice(),
        [kairo_ecs_des::FlowBatchReceipt::Rejected(rejection)]
            if rejection.error == FlowError::InvalidState && rejection.failed_ticket.is_none()
    ));
    assert!(unowned_pause.records.is_empty());
    assert_eq!(flow.work(work).unwrap().request, None);
    assert!(flow.resource(resource).unwrap().queued.is_empty());
    let progress_after_repeated_pause = flow
        .work_context::<TransitContext>(carrier_work)
        .unwrap()
        .progress_at(flow.now())
        .unwrap();
    assert_eq!(progress_before_repeated_pause.phase, TransitPhase::Paused);
    assert_eq!(progress_after_repeated_pause.phase, TransitPhase::Paused);
    assert_eq!(
        progress_after_repeated_pause,
        progress_before_repeated_pause
    );
    assert_eq!(
        flow.work_context::<TransitContext>(carrier_work)
            .unwrap()
            .phase(),
        TransitPhase::Paused
    );

    let stale_start = dispatch_and_observe(&mut flow, &mut bound);
    assert_bound_retained(
        &flow,
        &bound,
        work,
        frozen_decision,
        frozen_duration,
        frozen_draw_bounds.1,
    );
    assert_eq!(stale_start.event, first_start);
    no_request_admissions(&stale_start);
    let prestart_resume = owned_control(
        &mut flow,
        &mut bound,
        FlowDomainControl::Resume,
        SimTime::from_ticks(6),
        0,
    );
    let resumed_ready = dispatch_and_observe(&mut flow, &mut bound);
    assert_eq!(resumed_ready.event, prestart_resume);
    assert_bound_retained(
        &flow,
        &bound,
        work,
        frozen_decision,
        frozen_duration,
        frozen_draw_bounds.1,
    );
    no_request_admissions(&resumed_ready);
    let accepted_start = dispatch_and_observe(&mut flow, &mut bound);
    assert_bound_retained(
        &flow,
        &bound,
        work,
        frozen_decision,
        frozen_duration,
        frozen_draw_bounds.1,
    );
    no_request_admissions(&accepted_start);
    assert_eq!(flow.now(), SimTime::from_ticks(6));
    let context = flow.work_context::<TransitContext>(carrier_work).unwrap();
    assert_eq!(context.phase(), TransitPhase::Moving);
    assert_eq!(
        context.progress_at(flow.now()).unwrap().useful_elapsed,
        SimDuration::ZERO
    );
    assert_eq!(
        context.progress_at(flow.now()).unwrap().remaining,
        SimDuration::from_ticks(1)
    );

    let poststart_pause = owned_control(
        &mut flow,
        &mut bound,
        FlowDomainControl::Pause,
        SimTime::from_ticks(6),
        -1,
    );
    let paused_moving = dispatch_and_observe(&mut flow, &mut bound);
    assert_eq!(paused_moving.event, poststart_pause);
    assert_bound_retained(
        &flow,
        &bound,
        work,
        frozen_decision,
        frozen_duration,
        frozen_draw_bounds.1,
    );
    no_request_admissions(&paused_moving);
    let stale_arrival = dispatch_and_observe(&mut flow, &mut bound);
    assert_bound_retained(
        &flow,
        &bound,
        work,
        frozen_decision,
        frozen_duration,
        frozen_draw_bounds.1,
    );
    no_request_admissions(&stale_arrival);
    assert_eq!(flow.work(work).unwrap().request, None);
    let context = flow.work_context::<TransitContext>(carrier_work).unwrap();
    assert_eq!(context.phase(), TransitPhase::Paused);
    assert_eq!(
        context.progress_at(flow.now()).unwrap().useful_elapsed,
        SimDuration::ZERO
    );
    assert_eq!(
        context.progress_at(flow.now()).unwrap().remaining,
        SimDuration::from_ticks(1)
    );
    let poststart_resume = owned_control(
        &mut flow,
        &mut bound,
        FlowDomainControl::Resume,
        SimTime::from_ticks(8),
        0,
    );
    let resumed_moving = dispatch_and_observe(&mut flow, &mut bound);
    assert_eq!(resumed_moving.event, poststart_resume);
    assert_bound_retained(
        &flow,
        &bound,
        work,
        frozen_decision,
        frozen_duration,
        frozen_draw_bounds.1,
    );
    no_request_admissions(&resumed_moving);

    let mut arrivals = Vec::new();
    for _ in 0..8 {
        let Some(dispatch) = flow.step().unwrap() else {
            break;
        };
        let observation = bound
            .observe_transit_dispatch(&flow, &dispatch)
            .unwrap_or_else(|_| panic!("actual transit dispatch observation failed"));
        assert_bound_retained(
            &flow,
            &bound,
            work,
            frozen_decision,
            frozen_duration,
            frozen_draw_bounds.1,
        );
        if matches!(observation, TransitObservation::Arrived) {
            arrivals.push(dispatch);
            break;
        }
    }
    assert_eq!(arrivals.len(), 1);
    let arrival = &arrivals[0];
    assert_bound_retained(
        &flow,
        &bound,
        work,
        frozen_decision,
        frozen_duration,
        frozen_draw_bounds.1,
    );
    assert_eq!(arrival.at, SimTime::from_ticks(9));
    assert_invalid_dispatch(&flow, &mut bound, arrival);
    let admissions = accepted(arrival);
    assert_eq!(admissions.len(), 1);
    let request_id = admissions[0].request.expect("actual timed arrival claim");
    let request = flow.request(request_id).unwrap();
    assert_eq!(request.work, Some(work));
    assert_eq!(request.resource, resource);
    assert_eq!(request.owner, owner);
    assert!(request.timed);
    assert_eq!(flow.work(work).unwrap().request, Some(request_id));
    let submitted = bound
        .finish_transit(&flow)
        .unwrap_or_else(|_| panic!("accepted arrival did not yield submitted intrinsic work"));
    assert_eq!(submitted.work(), work);
    assert_eq!(submitted.decision(), frozen_decision);
    assert_eq!(submitted.sampled_duration(), frozen_duration);
    assert_eq!(submitted.service_draw_position(), frozen_draw_bounds.1);
    assert!(submitted.service_identity_matches());
    assert_eq!(submitted.request(), request_id);
    let context = flow.work_context::<TransitContext>(carrier_work).unwrap();
    assert_eq!(context.phase(), TransitPhase::Arrived);
    assert_eq!(
        context
            .progress_at(SimTime::from_ticks(9))
            .unwrap()
            .useful_elapsed,
        SimDuration::from_ticks(1)
    );
    let useful_travel = context
        .progress_at(SimTime::from_ticks(9))
        .unwrap()
        .useful_elapsed
        .ticks();
    assert_eq!(
        flow.request(request_id).unwrap().submitted_at,
        SimTime::from_ticks(9)
    );
    let mut completion_record = None;
    for _ in 0..32 {
        if flow.work_progress(work).unwrap().state == WorkState::Completed {
            break;
        }
        let dispatch = flow
            .step()
            .unwrap()
            .expect("intrinsic service completion event");
        if let Some(record) = dispatch.records.iter().find(|record| {
            record.request == request_id && record.transition == LifecycleTransition::Completed
        }) {
            completion_record = Some(record.clone());
        }
    }
    let completion_record = completion_record.expect("actual completed lifecycle record");
    assert_eq!(completion_record.at, SimTime::from_ticks(39));
    assert_eq!(
        flow.work_progress(work).unwrap().state,
        WorkState::Completed
    );
    let completed = flow.work_progress(work).unwrap();
    assert_eq!(completed.useful_elapsed, SimDuration::from_ticks(30));
    assert_eq!(completed.remaining, SimDuration::ZERO);
    assert_eq!(completed.cumulative_busy, SimDuration::from_ticks(30));
    assert_eq!(completed.completion_at, None);
    let completed_record_progress = completion_record
        .snapshot
        .progress
        .as_ref()
        .expect("completed lifecycle record retains work progress");
    assert_eq!(completed_record_progress.state, WorkState::Completed);
    assert_eq!(
        completed_record_progress.useful_elapsed,
        SimDuration::from_ticks(30)
    );
    assert_eq!(completed_record_progress.remaining, SimDuration::ZERO);
    assert_eq!(
        completed_record_progress.cumulative_busy,
        SimDuration::from_ticks(30)
    );
    assert_eq!(submitted.service_draw_position(), 1);
    let clock_start = SimTime::from_ticks(0).ticks();
    let first_pause_tick = paused_ready.at.ticks();
    let prestart_resume_tick = resumed_ready.at.ticks();
    let pause_start_tick = paused_moving.at.ticks();
    let resume_tick = resumed_moving.at.ticks();
    let arrival_tick = arrival.at.ticks();
    let submitted_tick = flow.request(request_id).unwrap().submitted_at.ticks();
    let completion_tick = completion_record.at.ticks();
    let idle_before_pause = first_pause_tick - clock_start;
    let paused_before_start = prestart_resume_tick - first_pause_tick;
    let paused_after_start = resume_tick - pause_start_tick;
    let queue_after_arrival = submitted_tick - arrival_tick;
    let service = completion_tick - submitted_tick;
    assert_eq!(paused_ready.at, SimTime::from_ticks(2));
    assert_eq!(resumed_ready.at, SimTime::from_ticks(6));
    assert_eq!(idle_before_pause, 2);
    assert_eq!(paused_before_start, 4);
    assert_eq!(useful_travel, 1);
    assert_eq!(paused_after_start, 2);
    assert_eq!(queue_after_arrival, 0);
    assert_eq!(service, 30);
    assert_eq!(
        completion_tick - clock_start,
        idle_before_pause
            + paused_before_start
            + useful_travel
            + paused_after_start
            + queue_after_arrival
            + service
    );
}
