use kairo_ecs_abm::spatial::{
    EdgeId, MovementModeId, MovementProfile, NodeId, RoutePlan, TransitEdge, TransitGraphV1,
};
use kairo_ecs_abm::{
    observe_transit_arrival_handoff, pending_transit_arrival_handoff, register_transit_context,
    register_transit_context_checkpoint_codec, register_transit_context_checkpoint_domain,
    schedule_transit_control, schedule_transit_start, TransitContext,
    TransitContextCheckpointError, TransitContextCheckpointLimitsV1, TransitContextWireError,
    TransitPhase,
};
use kairo_ecs_des::{
    FlowAcquireCommand, FlowBatchReceipt, FlowCallbackCodeV1, FlowCheckpointCodecError,
    FlowCheckpointCodecs, FlowCheckpointLimits, FlowConfig, FlowDomainControl, FlowRuntime,
    LifecycleTransition, RequestState, WorkId, WorkState,
};
use kairo_ecs_types::{EntityId, EventId, EventKind, SimDuration, SimTime};
use std::num::NonZeroU64;
use std::sync::Arc;

const KIND: EventKind = EventKind::custom(0x7c31);
const LIMITS: TransitContextCheckpointLimitsV1 =
    TransitContextCheckpointLimitsV1::new(16, 4096, 64, 8192);

fn graph_route(lengths: &[u64]) -> (TransitGraphV1, RoutePlan) {
    let nodes = (1..=lengths.len() + 1)
        .map(|id| NodeId::new(id as u64))
        .collect::<Vec<_>>();
    let mode = MovementModeId::new("walk").unwrap();
    let edges = lengths
        .iter()
        .enumerate()
        .map(|(i, length_mm)| TransitEdge {
            id: EdgeId::new((i + 1) as u64),
            from: nodes[i],
            to: nodes[i + 1],
            length_mm: *length_mm,
            allowed_modes: vec![mode.clone()],
        })
        .collect();
    let graph = TransitGraphV1::new(1, nodes.clone(), edges).unwrap();
    let route = graph
        .route(
            nodes[0],
            *nodes.last().unwrap(),
            &MovementProfile::new("walk", 1).unwrap(),
            1,
        )
        .unwrap();
    (graph, route)
}

fn handoff_flow(lengths: &[u64]) -> (FlowRuntime, WorkId, WorkId, EntityId, TransitGraphV1) {
    handoff_flow_with_config(lengths, None)
}

fn handoff_flow_with_config(
    lengths: &[u64],
    config: Option<FlowConfig>,
) -> (FlowRuntime, WorkId, WorkId, EntityId, TransitGraphV1) {
    let mut flow = config.map_or_else(FlowRuntime::new, FlowRuntime::with_config);
    register_transit_context(&mut flow, "native-transit", KIND).unwrap();
    let owner = flow.spawn_actor().unwrap();
    let carrier_actor = flow.spawn_actor().unwrap();
    let service = flow
        .create_work(owner, SimDuration::from_ticks(4), "service", ())
        .unwrap();
    let (graph, route) = graph_route(lengths);
    let context = TransitContext::new_handoff(&flow, route, service, SimTime::ZERO, 3).unwrap();
    let carrier = flow
        .create_actor_domain_context(carrier_actor, "native-transit", KIND, context)
        .unwrap();
    (flow, carrier, service, owner, graph)
}

fn start(flow: &mut FlowRuntime, carrier: WorkId) {
    schedule_transit_start(flow, carrier, KIND, SimTime::ZERO, 0).unwrap();
    let dispatch = flow.step().unwrap().unwrap();
    assert!(dispatch.error.is_none());
    assert!(
        matches!(dispatch.callback_batches.as_slice(), [FlowBatchReceipt::Accepted(rows)] if rows.len() == 1 && rows[0].request.is_none())
    );
}

fn run_to_handoff(flow: &mut FlowRuntime, carrier: WorkId) -> kairo_ecs_des::FlowDispatch {
    start(flow, carrier);
    for _ in 0..16 {
        let dispatch = flow.step().unwrap().unwrap();
        if flow
            .work_context::<TransitContext>(carrier)
            .unwrap()
            .phase()
            == TransitPhase::Arrived
        {
            assert!(dispatch.error.is_none());
            assert!(
                matches!(dispatch.callback_batches.as_slice(), [FlowBatchReceipt::Accepted(rows)] if rows.is_empty())
            );
            return dispatch;
        }
    }
    panic!("route did not arrive within the bounded fixture horizon")
}

fn codecs(source: &FlowRuntime, graph: Arc<TransitGraphV1>) -> FlowCheckpointCodecs {
    let mut codecs = FlowCheckpointCodecs::new();
    register_transit_context_checkpoint_codec(
        &mut codecs,
        "native-transit-v1",
        source.identity(),
        graph,
        LIMITS,
    )
    .unwrap();
    register_transit_context_checkpoint_domain(
        &mut codecs,
        "native-transit",
        KIND,
        FlowCallbackCodeV1 {
            stable_id: "test.native.transit.planner".to_owned(),
            version: 1,
        },
        FlowCallbackCodeV1 {
            stable_id: "test.native.transit.receipt".to_owned(),
            version: 1,
        },
    )
    .unwrap();
    codecs
        .register_context::<()>(
            "unit-v1",
            1,
            |_, _| Ok(Vec::new()),
            |bytes, _| {
                if bytes.is_empty() {
                    Ok(())
                } else {
                    Err(FlowCheckpointCodecError("invalid unit marker".to_owned()))
                }
            },
        )
        .unwrap();
    codecs
}

#[test]
fn transit_handoff_no_resource_start_arrival() {
    let (mut flow, carrier, service, owner, _graph) = handoff_flow(&[2]);
    assert!(pending_transit_arrival_handoff(&flow, carrier)
        .unwrap()
        .is_none());
    let foreign = FlowRuntime::new();
    assert_eq!(
        pending_transit_arrival_handoff(&foreign, carrier),
        Err(kairo_ecs_des::FlowError::InvalidWork)
    );
    assert_eq!(
        pending_transit_arrival_handoff(&flow, service),
        Err(kairo_ecs_des::FlowError::InvalidWork)
    );
    let cloned_context = flow
        .work_context::<TransitContext>(carrier)
        .unwrap()
        .clone();
    let task_owner = flow.spawn_actor().unwrap();
    let task_with_cloned_context = flow
        .create_work(
            task_owner,
            SimDuration::ZERO,
            "ordinary-task",
            cloned_context,
        )
        .unwrap();
    assert_eq!(
        pending_transit_arrival_handoff(&flow, task_with_cloned_context),
        Err(kairo_ecs_des::FlowError::InvalidWork)
    );
    assert_eq!(flow.work(service).unwrap().request, None);
    assert_eq!(
        flow.work_progress(service).unwrap().state,
        WorkState::Pending
    );
    let dispatch = run_to_handoff(&mut flow, carrier);
    assert_eq!(dispatch.at, SimTime::from_ticks(2));
    assert!(
        matches!(dispatch.callback_batches.as_slice(), [FlowBatchReceipt::Accepted(rows)] if rows.is_empty())
    );
    assert_eq!(flow.work(service).unwrap().owner, owner);
    assert_eq!(flow.work(service).unwrap().request, None);
    assert_eq!(
        flow.work_progress(service).unwrap().state,
        WorkState::Pending
    );
    assert_eq!(
        pending_transit_arrival_handoff(&flow, carrier)
            .unwrap()
            .unwrap()
            .service_work(),
        service
    );
    let observed = observe_transit_arrival_handoff(&flow, carrier, &dispatch)
        .unwrap()
        .unwrap();
    assert_eq!(observed.arrival_event(), dispatch.event);
    assert_eq!(observed.arrived_at(), dispatch.at);
    assert_eq!(observed.carrier(), carrier);

    let mut removed_owner = FlowRuntime::new();
    let dead_owner = removed_owner.spawn_actor().unwrap();
    let orphaned_service = removed_owner
        .create_work(dead_owner, SimDuration::from_ticks(1), "service", ())
        .unwrap();
    let (_, dead_route) = graph_route(&[1]);
    removed_owner.despawn_actor(dead_owner).unwrap();
    removed_owner.step().unwrap().unwrap();
    assert_eq!(
        removed_owner.work(orphaned_service),
        Err(kairo_ecs_des::FlowError::InvalidWork)
    );
    assert!(TransitContext::new_handoff(
        &removed_owner,
        dead_route,
        orphaned_service,
        removed_owner.now(),
        0,
    )
    .is_err());
}

#[test]
fn transit_handoff_resume_frontier_empty_commit() {
    let (mut flow, carrier, service, _, _) = handoff_flow(&[2]);
    start(&mut flow, carrier);
    let pause_at = SimTime::from_ticks(2);
    schedule_transit_control(
        &mut flow,
        carrier,
        KIND,
        FlowDomainControl::Pause,
        pause_at,
        -2,
    )
    .unwrap();
    let resume = schedule_transit_control(
        &mut flow,
        carrier,
        KIND,
        FlowDomainControl::Resume,
        pause_at,
        -1,
    )
    .unwrap();
    let paused = flow.step().unwrap().unwrap();
    assert_eq!(paused.at, pause_at);
    assert_eq!(
        flow.work_context::<TransitContext>(carrier)
            .unwrap()
            .phase(),
        TransitPhase::Paused
    );
    assert_eq!(
        flow.work_context::<TransitContext>(carrier)
            .unwrap()
            .progress_at(pause_at)
            .unwrap()
            .remaining,
        SimDuration::ZERO
    );
    let arrival = flow.step().unwrap().unwrap();
    assert_eq!(arrival.event, resume);
    assert_eq!(arrival.at, pause_at);
    assert!(arrival.error.is_none());
    assert!(
        matches!(arrival.callback_batches.as_slice(), [FlowBatchReceipt::Accepted(rows)] if rows.is_empty())
    );
    let handoff = observe_transit_arrival_handoff(&flow, carrier, &arrival)
        .unwrap()
        .unwrap();
    assert_eq!(handoff.arrival_event(), resume);
    assert_eq!(handoff.service_work(), service);
    assert_eq!(flow.work(service).unwrap().request, None);
}

#[test]
fn transit_handoff_service_admits_once() {
    let (mut flow, carrier, service, owner, _) = handoff_flow(&[1]);
    let initial_progress = flow.work_progress(service).unwrap().clone();
    assert_eq!(
        initial_progress.original_duration,
        SimDuration::from_ticks(4)
    );
    assert_eq!(initial_progress.useful_elapsed, SimDuration::ZERO);
    assert_eq!(initial_progress.remaining, SimDuration::from_ticks(4));
    let arrival = run_to_handoff(&mut flow, carrier);
    let after_transit = flow.work_progress(service).unwrap().clone();
    assert_eq!(
        after_transit.original_duration,
        initial_progress.original_duration
    );
    assert_eq!(after_transit.useful_elapsed, SimDuration::ZERO);
    assert_eq!(after_transit.remaining, SimDuration::from_ticks(4));
    let resource = flow.create_resource(1).unwrap();
    let request = flow
        .acquire(resource)
        .owner(owner)
        .at(arrival.at)
        .timed_work(service)
        .submit()
        .unwrap();
    assert_eq!(flow.work(service).unwrap().request, Some(request));
    assert_eq!(flow.request(request).unwrap().state, RequestState::Pending);
    assert!(pending_transit_arrival_handoff(&flow, carrier)
        .unwrap()
        .is_none());
    assert!(observe_transit_arrival_handoff(&flow, carrier, &arrival)
        .unwrap()
        .is_none());
    let before = flow.budget_snapshot();
    assert_eq!(
        flow.acquire(resource)
            .owner(owner)
            .at(arrival.at)
            .timed_work(service)
            .submit(),
        Err(kairo_ecs_des::FlowError::InvalidWork)
    );
    assert_eq!(flow.budget_snapshot(), before);
    assert_eq!(flow.work(service).unwrap().request, Some(request));

    let grant = flow.step().unwrap().unwrap();
    assert_eq!(grant.at, arrival.at);
    assert!(grant.records.iter().any(|record| {
        record.request == request
            && record.transition == LifecycleTransition::Granted
            && record.at == arrival.at
            && record.snapshot.work == Some(service)
    }));
    assert_eq!(flow.request(request).unwrap().state, RequestState::Active);
    let active = flow.work_progress(service).unwrap();
    assert_eq!(active.state, WorkState::Active);
    assert_eq!(active.useful_elapsed, SimDuration::ZERO);
    assert_eq!(active.remaining, SimDuration::from_ticks(4));
    assert!(pending_transit_arrival_handoff(&flow, carrier)
        .unwrap()
        .is_none());
    assert!(observe_transit_arrival_handoff(&flow, carrier, &arrival)
        .unwrap()
        .is_none());

    let completion = flow.step().unwrap().unwrap();
    assert_eq!(completion.at, SimTime::from_ticks(arrival.at.ticks() + 4));
    assert!(completion.records.iter().any(|record| {
        record.request == request
            && record.transition == LifecycleTransition::Completed
            && record.at == completion.at
            && record.snapshot.work == Some(service)
    }));
    assert_eq!(
        flow.request(request).unwrap().state,
        RequestState::Completed
    );
    let completed = flow.work_progress(service).unwrap();
    assert_eq!(completed.state, WorkState::Completed);
    assert_eq!(completed.useful_elapsed, SimDuration::from_ticks(4));
    assert_eq!(completed.remaining, SimDuration::ZERO);
    assert!(pending_transit_arrival_handoff(&flow, carrier)
        .unwrap()
        .is_none());
    assert!(observe_transit_arrival_handoff(&flow, carrier, &arrival)
        .unwrap()
        .is_none());
}

#[test]
fn transit_handoff_identity_and_error_receipts() {
    let (mut flow, carrier, service, _, _) = handoff_flow(&[1]);
    let arrival = run_to_handoff(&mut flow, carrier);
    let cloned_arrived_context = flow
        .work_context::<TransitContext>(carrier)
        .unwrap()
        .clone();
    let other_actor = flow.spawn_actor().unwrap();
    let other_carrier = flow
        .create_actor_domain_context(other_actor, "native-transit", KIND, cloned_arrived_context)
        .unwrap();
    assert_eq!(
        pending_transit_arrival_handoff(&flow, other_carrier),
        Err(kairo_ecs_des::FlowError::InvalidWork)
    );
    let before = flow.budget_snapshot();
    assert!(observe_transit_arrival_handoff(&flow, carrier, &arrival)
        .unwrap()
        .is_some());
    let mut wrong_time = arrival.clone();
    wrong_time.at = SimTime::from_ticks(arrival.at.ticks() + 1);
    assert_eq!(
        observe_transit_arrival_handoff(&flow, carrier, &wrong_time),
        Err(kairo_ecs_des::FlowError::InvalidWork)
    );
    let mut matching_error = arrival.clone();
    matching_error.error = Some(kairo_ecs_des::FlowError::InvalidState);
    assert_eq!(
        observe_transit_arrival_handoff(&flow, carrier, &matching_error),
        Err(kairo_ecs_des::FlowError::InvalidWork)
    );
    let mut rejected = arrival.clone();
    rejected.callback_batches = vec![FlowBatchReceipt::Rejected(
        kairo_ecs_des::FlowBatchRejection {
            failed_ticket: None,
            error: kairo_ecs_des::FlowError::InvalidState,
        },
    )];
    assert_eq!(
        observe_transit_arrival_handoff(&flow, carrier, &rejected),
        Err(kairo_ecs_des::FlowError::InvalidWork)
    );
    let mut unrelated_error = arrival.clone();
    unrelated_error.event = EventId::new(arrival.event.index + 1, arrival.event.generation);
    unrelated_error.error = Some(kairo_ecs_des::FlowError::InvalidState);
    assert!(
        observe_transit_arrival_handoff(&flow, carrier, &unrelated_error)
            .unwrap()
            .is_none()
    );
    let foreign = FlowRuntime::new();
    assert_eq!(
        observe_transit_arrival_handoff(&foreign, carrier, &arrival),
        Err(kairo_ecs_des::FlowError::InvalidWork)
    );
    assert_eq!(
        observe_transit_arrival_handoff(&flow, service, &arrival),
        Err(kairo_ecs_des::FlowError::InvalidWork)
    );
    assert_eq!(flow.budget_snapshot(), before);
    assert_eq!(flow.work(service).unwrap().request, None);
    assert_eq!(
        flow.work_context::<TransitContext>(carrier)
            .unwrap()
            .phase(),
        TransitPhase::Arrived
    );
}

#[test]
fn transit_handoff_pause_resume_same_clock() {
    let (mut flow, carrier, service, _, _) = handoff_flow(&[2]);
    start(&mut flow, carrier);
    schedule_transit_control(
        &mut flow,
        carrier,
        KIND,
        FlowDomainControl::Pause,
        SimTime::from_ticks(1),
        -1,
    )
    .unwrap();
    let pause = flow.step().unwrap().unwrap();
    assert_eq!(pause.at, SimTime::from_ticks(1));
    let paused = flow
        .work_context::<TransitContext>(carrier)
        .unwrap()
        .progress_at(pause.at)
        .unwrap();
    assert_eq!(paused.phase, TransitPhase::Paused);
    assert_eq!(paused.useful_elapsed, SimDuration::from_ticks(1));
    // The progress event scheduled before Pause remains queued and is a no-op.
    let stale_progress = flow.step().unwrap().unwrap();
    assert_eq!(stale_progress.at, SimTime::from_ticks(2));
    assert_eq!(stale_progress.error, None);
    assert_eq!(
        flow.work_context::<TransitContext>(carrier)
            .unwrap()
            .phase(),
        TransitPhase::Paused
    );
    schedule_transit_control(
        &mut flow,
        carrier,
        KIND,
        FlowDomainControl::Resume,
        SimTime::from_ticks(3),
        0,
    )
    .unwrap();
    let resume = flow.step().unwrap().unwrap();
    assert_eq!(resume.at, SimTime::from_ticks(3));
    assert!(resume.error.is_none());
    let arrival = flow.step().unwrap().unwrap();
    assert_eq!(arrival.at, SimTime::from_ticks(4));
    assert!(
        matches!(arrival.callback_batches.as_slice(), [FlowBatchReceipt::Accepted(rows)] if rows.is_empty())
    );
    let final_progress = flow
        .work_context::<TransitContext>(carrier)
        .unwrap()
        .progress_at(arrival.at)
        .unwrap();
    assert_eq!(final_progress.useful_elapsed, SimDuration::from_ticks(2));
    assert_eq!(final_progress.remaining, SimDuration::ZERO);
    assert_eq!(flow.work(service).unwrap().request, None);
}

#[test]
fn transit_handoff_budget_halt_pending() {
    let config = FlowConfig {
        max_same_tick_flow_transitions: NonZeroU64::new(1).unwrap(),
    };
    let (mut flow, carrier, service, _, _) = handoff_flow_with_config(&[1], Some(config));
    start(&mut flow, carrier);
    let at = SimTime::from_ticks(1);
    schedule_transit_control(&mut flow, carrier, KIND, FlowDomainControl::Pause, at, -2).unwrap();
    let resume =
        schedule_transit_control(&mut flow, carrier, KIND, FlowDomainControl::Resume, at, -1)
            .unwrap();
    let pause = flow.step().unwrap().unwrap();
    assert_eq!(pause.at, at);
    assert_eq!(
        flow.work_context::<TransitContext>(carrier)
            .unwrap()
            .phase(),
        TransitPhase::Paused
    );
    let phase = flow
        .work_context::<TransitContext>(carrier)
        .unwrap()
        .progress_at(at)
        .unwrap();
    assert_eq!(phase.remaining, SimDuration::ZERO);
    assert!(pending_transit_arrival_handoff(&flow, carrier)
        .unwrap()
        .is_none());
    let before = flow.budget_snapshot();
    let first = flow.step().unwrap_err();
    assert_eq!(
        first,
        kairo_ecs_des::FlowError::SameTickBudgetExceeded {
            at_ticks: 1,
            limit: 1
        }
    );
    let halted = flow.budget_snapshot();
    assert_eq!(halted.halted.unwrap().pending.id, resume);
    assert_eq!(flow.work(service).unwrap().request, None);
    for _ in 0..2 {
        assert_eq!(flow.step().unwrap_err(), first);
        assert_eq!(flow.budget_snapshot(), halted);
        assert_eq!(
            flow.work_context::<TransitContext>(carrier)
                .unwrap()
                .progress_at(at)
                .unwrap(),
            phase
        );
        assert!(pending_transit_arrival_handoff(&flow, carrier)
            .unwrap()
            .is_none());
        assert_eq!(flow.work(service).unwrap().request, None);
    }
    assert_ne!(before, halted);
}

#[test]
fn transit_handoff_legacy_and_v1_guard() {
    let (graph, route) = graph_route(&[1]);
    let mut flow = FlowRuntime::new();
    register_transit_context(&mut flow, "native-transit", KIND).unwrap();
    let owner = flow.spawn_actor().unwrap();
    let carrier_actor = flow.spawn_actor().unwrap();
    let service = flow
        .create_work(owner, SimDuration::from_ticks(2), "service", ())
        .unwrap();
    let handoff =
        TransitContext::new_handoff(&flow, route.clone(), service, SimTime::ZERO, 0).unwrap();
    assert_eq!(
        handoff.checkpoint_v1(&flow.identity(), LIMITS),
        Err(TransitContextCheckpointError::InvalidState)
    );
    assert_eq!(
        handoff.checkpoint_bytes_v1(&flow.identity(), LIMITS),
        Err(TransitContextWireError::Native(
            TransitContextCheckpointError::InvalidState
        ))
    );
    assert_eq!(flow.work(service).unwrap().request, None);

    let resource = flow.create_resource(1).unwrap();
    let legacy = TransitContext::new(
        &flow,
        route,
        FlowAcquireCommand {
            resource,
            owner,
            work: Some(service),
            at: SimTime::ZERO,
            priority_level: 4,
            deadline: None,
            scheduler_priority: 0,
            timed: true,
            can_preempt: false,
            preemptible: None,
        },
        SimTime::ZERO,
    )
    .unwrap();
    assert!(legacy
        .checkpoint_bytes_v1(&flow.identity(), LIMITS)
        .unwrap()
        .starts_with(b"KTCXCPV1"));
    let carrier = flow
        .create_actor_domain_context(carrier_actor, "native-transit", KIND, legacy)
        .unwrap();
    let codecs = codecs(&flow, Arc::new(graph));
    let image = flow
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    assert!(image
        .context_stores
        .iter()
        .any(|store| store.codec_key == "native-transit-v1"));
    let dispatch = run_legacy_to_arrival(&mut flow, carrier);
    assert!(dispatch.error.is_none());
    let FlowBatchReceipt::Accepted(admissions) = &dispatch.callback_batches[0] else {
        panic!("legacy arrival must accept its acquire");
    };
    assert_eq!(admissions.len(), 1);
    assert!(admissions[0].request.is_some());
    assert_eq!(flow.work(service).unwrap().request, admissions[0].request);
}

fn run_legacy_to_arrival(flow: &mut FlowRuntime, carrier: WorkId) -> kairo_ecs_des::FlowDispatch {
    schedule_transit_start(flow, carrier, KIND, SimTime::ZERO, 0).unwrap();
    assert!(flow.step().unwrap().unwrap().error.is_none());
    for _ in 0..8 {
        let dispatch = flow.step().unwrap().unwrap();
        if flow
            .work_context::<TransitContext>(carrier)
            .unwrap()
            .phase()
            == TransitPhase::Arrived
        {
            return dispatch;
        }
    }
    panic!("legacy route did not arrive")
}
