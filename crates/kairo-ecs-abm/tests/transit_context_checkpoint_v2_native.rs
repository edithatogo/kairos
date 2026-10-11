use kairo_ecs_abm::spatial::{
    EdgeId, MovementModeId, MovementProfile, NodeId, TransitEdge, TransitGraphV1,
};
use kairo_ecs_abm::{
    observe_transit_arrival_handoff, pending_transit_arrival_handoff, register_transit_context,
    register_transit_context_checkpoint_domain, schedule_transit_control, schedule_transit_start,
    TransitArrivalActionCheckpointV2, TransitArrivalCauseCheckpointV2, TransitContext,
    TransitContextCheckpointError, TransitContextCheckpointLimitsV1, TransitContextCheckpointV2,
    TransitPhase,
};
use kairo_ecs_des::{
    FlowAcquireCommand, FlowBatchReceipt, FlowCallbackCodeV1, FlowCheckpointCodecError,
    FlowCheckpointCodecs, FlowCheckpointLimits, FlowDomainControl, FlowRuntime, RequestState,
    WorkId,
};
use kairo_ecs_types::{EntityId, EventKind, SimDuration, SimTime};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

const KIND: EventKind = EventKind::custom(0x7c51);
const LIMITS: TransitContextCheckpointLimitsV1 =
    TransitContextCheckpointLimitsV1::new(16, 4096, 64, 8192);
const V2_CODEC: &str = "test.transit-context-v2-native";
const UNIT_CODEC: &str = "test.unit-v1";
const TOKEN: &[u8] = &[0xa2];

fn graph_route(lengths: &[u64]) -> (TransitGraphV1, kairo_ecs_abm::spatial::RoutePlan) {
    let nodes = (1..=lengths.len() + 1)
        .map(|id| NodeId::new(id as u64))
        .collect::<Vec<_>>();
    let mode = MovementModeId::new("walk").unwrap();
    let edges = lengths
        .iter()
        .enumerate()
        .map(|(index, length_mm)| TransitEdge {
            id: EdgeId::new((index + 1) as u64),
            from: nodes[index],
            to: nodes[index + 1],
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

struct Fixture {
    flow: FlowRuntime,
    carrier: WorkId,
    carrier_actor: EntityId,
    service: WorkId,
    owner: EntityId,
    graph: Arc<TransitGraphV1>,
}

fn fixture(lengths: &[u64]) -> Fixture {
    let mut flow = FlowRuntime::new();
    register_transit_context(&mut flow, "native-v2", KIND).unwrap();
    let owner = flow.spawn_actor().unwrap();
    let carrier_actor = flow.spawn_actor().unwrap();
    let service = flow
        .create_work(owner, SimDuration::from_ticks(4), "service", ())
        .unwrap();
    let (graph, route) = graph_route(lengths);
    let context = TransitContext::new_handoff(&flow, route, service, SimTime::ZERO, 3).unwrap();
    let carrier = flow
        .create_actor_domain_context(carrier_actor, "native-v2", KIND, context)
        .unwrap();
    Fixture {
        flow,
        carrier,
        carrier_actor,
        service,
        owner,
        graph: Arc::new(graph),
    }
}

fn encode_unit(_: &(), _: usize) -> Result<Vec<u8>, FlowCheckpointCodecError> {
    Ok(Vec::new())
}

fn decode_unit(
    bytes: &[u8],
    _: &kairo_ecs_des::FlowCheckpointRebindV1,
) -> Result<(), FlowCheckpointCodecError> {
    if bytes.is_empty() {
        Ok(())
    } else {
        Err(FlowCheckpointCodecError("invalid unit marker".to_owned()))
    }
}

struct Captured {
    image: kairo_ecs_des::FlowCheckpointV1,
    slot: Rc<RefCell<Option<TransitContextCheckpointV2>>>,
    restore_error: Rc<RefCell<Option<TransitContextCheckpointError>>>,
    codecs: FlowCheckpointCodecs,
    dto: TransitContextCheckpointV2,
}

fn codecs(
    source: &FlowRuntime,
    graph: Arc<TransitGraphV1>,
    slot: Rc<RefCell<Option<TransitContextCheckpointV2>>>,
    restore_error: Rc<RefCell<Option<TransitContextCheckpointError>>>,
) -> FlowCheckpointCodecs {
    let mut codecs = FlowCheckpointCodecs::new();
    let source_identity = source.identity();
    let encode_slot = slot.clone();
    let decode_slot = slot;
    let decode_error = restore_error;
    codecs
        .register_context_with_owner::<TransitContext>(
            V2_CODEC,
            1,
            move |context, _| {
                let image = context
                    .checkpoint_v2(&source_identity, LIMITS)
                    .map_err(|error| FlowCheckpointCodecError(error.to_string()))?;
                *encode_slot.borrow_mut() = Some(image);
                Ok(TOKEN.to_vec())
            },
            move |bytes, row_owner, rebind| {
                if bytes != TOKEN {
                    return Err(FlowCheckpointCodecError("invalid V2 test token".to_owned()));
                }
                let image = decode_slot
                    .borrow()
                    .clone()
                    .ok_or_else(|| FlowCheckpointCodecError("missing frozen DTO".to_owned()))?;
                match image.restore_for_owner(&graph, rebind, row_owner, LIMITS) {
                    Ok(context) => Ok(context),
                    Err(error) => {
                        *decode_error.borrow_mut() = Some(error.clone());
                        Err(FlowCheckpointCodecError(error.to_string()))
                    }
                }
            },
        )
        .unwrap();
    codecs
        .register_context::<()>(UNIT_CODEC, 1, encode_unit, decode_unit)
        .unwrap();
    register_transit_context_checkpoint_domain(
        &mut codecs,
        "native-v2",
        KIND,
        FlowCallbackCodeV1 {
            stable_id: "test.native.transit-v2.planner".to_owned(),
            version: 1,
        },
        FlowCallbackCodeV1 {
            stable_id: "test.native.transit-v2.receipt".to_owned(),
            version: 1,
        },
    )
    .unwrap();
    codecs
}

fn capture(flow: &FlowRuntime, graph: Arc<TransitGraphV1>) -> Captured {
    let slot = Rc::new(RefCell::new(None));
    let restore_error = Rc::new(RefCell::new(None));
    let codecs = codecs(flow, graph.clone(), slot.clone(), restore_error.clone());
    let image = flow
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    let dto = slot.borrow().clone().expect("V2 context encoder ran");
    let store = image
        .context_stores
        .iter()
        .find(|store| store.codec_key == V2_CODEC)
        .unwrap();
    assert_eq!(store.rows.len(), 1);
    assert_eq!(store.rows[0].1, TOKEN);
    Captured {
        image,
        slot,
        restore_error,
        codecs,
        dto,
    }
}

impl Captured {
    fn restore(self) -> Result<FlowRuntime, kairo_ecs_des::FlowCheckpointError> {
        FlowRuntime::restore_checkpoint(self.image, &self.codecs, FlowCheckpointLimits::default())
    }

    fn restore_with(
        self,
        dto: TransitContextCheckpointV2,
    ) -> (
        Result<FlowRuntime, kairo_ecs_des::FlowCheckpointError>,
        Option<TransitContextCheckpointError>,
    ) {
        *self.slot.borrow_mut() = Some(dto);
        let restored = FlowRuntime::restore_checkpoint(
            self.image,
            &self.codecs,
            FlowCheckpointLimits::default(),
        );
        let error = self.restore_error.borrow().clone();
        (restored, error)
    }
}

fn step_ok(flow: &mut FlowRuntime) -> kairo_ecs_des::FlowDispatch {
    let dispatch = flow.step().unwrap().unwrap();
    assert!(dispatch.error.is_none(), "unexpected Flow dispatch error");
    dispatch
}

fn start(flow: &mut FlowRuntime, carrier: WorkId) {
    schedule_transit_start(flow, carrier, KIND, SimTime::ZERO, 0).unwrap();
    let dispatch = step_ok(flow);
    assert!(
        matches!(dispatch.callback_batches.as_slice(), [FlowBatchReceipt::Accepted(rows)] if rows.len() == 1 && rows[0].request.is_none())
    );
}

fn arrive(flow: &mut FlowRuntime, carrier: WorkId) -> kairo_ecs_des::FlowDispatch {
    start(flow, carrier);
    for _ in 0..16 {
        let dispatch = step_ok(flow);
        if flow
            .work_context::<TransitContext>(carrier)
            .unwrap()
            .phase()
            == TransitPhase::Arrived
        {
            return dispatch;
        }
    }
    panic!("bounded route did not arrive")
}

fn current_image(flow: &FlowRuntime, carrier: WorkId) -> TransitContextCheckpointV2 {
    flow.work_context::<TransitContext>(carrier)
        .unwrap()
        .checkpoint_v2(&flow.identity(), LIMITS)
        .unwrap()
}

#[test]
fn transit_checkpoint_v2_native_ready_and_scheduled_start() {
    let mut unscheduled = fixture(&[2]);
    let ready = capture(&unscheduled.flow, unscheduled.graph.clone());
    assert_eq!(ready.dto.phase, TransitPhase::Ready);
    assert!(ready.dto.initial_start_pending);
    assert_eq!(ready.dto.expected_event, None);
    assert_eq!(ready.dto.carrier, None);
    assert!(matches!(
        ready.dto.action,
        TransitArrivalActionCheckpointV2::Handoff { .. }
    ));
    let restored = ready.restore().unwrap();
    let restored_carrier = restored
        .actor_domain_context(unscheduled.carrier_actor)
        .unwrap();
    assert_eq!(
        current_image(&restored, restored_carrier),
        current_image(&unscheduled.flow, unscheduled.carrier)
    );

    schedule_transit_start(
        &mut unscheduled.flow,
        unscheduled.carrier,
        KIND,
        SimTime::ZERO,
        0,
    )
    .unwrap();
    let scheduled = capture(&unscheduled.flow, unscheduled.graph.clone());
    assert_eq!(scheduled.dto.phase, TransitPhase::Ready);
    assert!(scheduled.dto.expected_event.is_some());
    assert_eq!(scheduled.dto.expected_due_ticks, Some(0));
    assert_eq!(scheduled.dto.carrier, None);
    let restored = scheduled.restore().unwrap();
    assert_eq!(restored.now(), SimTime::ZERO);
}

#[test]
fn transit_checkpoint_v2_native_resumed_ready_later_due() {
    let mut f = fixture(&[2]);
    schedule_transit_start(&mut f.flow, f.carrier, KIND, SimTime::ZERO, 0).unwrap();
    schedule_transit_control(
        &mut f.flow,
        f.carrier,
        KIND,
        FlowDomainControl::Pause,
        SimTime::ZERO,
        -1,
    )
    .unwrap();
    step_ok(&mut f.flow); // Pause before the scheduled Start.
    step_ok(&mut f.flow); // The old Start is consumed while paused.
    let paused = f.flow.work_context::<TransitContext>(f.carrier).unwrap();
    assert_eq!(paused.phase(), TransitPhase::Paused);
    schedule_transit_control(
        &mut f.flow,
        f.carrier,
        KIND,
        FlowDomainControl::Resume,
        SimTime::from_ticks(5),
        -1,
    )
    .unwrap();
    step_ok(&mut f.flow);
    let dto = current_image(&f.flow, f.carrier);
    assert_eq!(dto.phase, TransitPhase::Ready);
    assert!(!dto.initial_start_pending);
    assert_eq!(dto.start_at_ticks, 0);
    assert_eq!(dto.expected_due_ticks, Some(5));
    assert!(dto.expected_event.is_some());
    let restored = capture(&f.flow, f.graph.clone()).restore().unwrap();
    let restored_carrier = restored.actor_domain_context(f.carrier_actor).unwrap();
    assert_eq!(current_image(&restored, restored_carrier), dto);
}

#[test]
fn transit_checkpoint_v2_native_moving_restore_suffix() {
    let mut f = fixture(&[2, 1]);
    start(&mut f.flow, f.carrier);
    let captured = capture(&f.flow, f.graph.clone());
    assert_eq!(captured.dto.phase, TransitPhase::Moving);
    assert!(captured.dto.expected_event.is_some());
    assert!(captured.dto.next_progress_ticket.is_some());
    let mut restored = captured.restore().unwrap();
    let restored_carrier = restored.actor_domain_context(f.carrier_actor).unwrap();
    let source_arrival = arrive_after_start(&mut f.flow, f.carrier);
    let restored_arrival = arrive_after_start(&mut restored, restored_carrier);
    assert_eq!(
        (source_arrival.event, source_arrival.at),
        (restored_arrival.event, restored_arrival.at)
    );
    assert_eq!(
        current_image(&f.flow, f.carrier),
        current_image(&restored, restored_carrier)
    );
    assert_eq!(f.flow.work(f.service).unwrap().request, None);
    assert_eq!(restored.work(f.service).unwrap().request, None);
}

fn arrive_after_start(flow: &mut FlowRuntime, carrier: WorkId) -> kairo_ecs_des::FlowDispatch {
    for _ in 0..16 {
        let dispatch = step_ok(flow);
        if flow
            .work_context::<TransitContext>(carrier)
            .unwrap()
            .phase()
            == TransitPhase::Arrived
        {
            return dispatch;
        }
    }
    panic!("bounded route suffix did not arrive")
}

fn resumed_suffix(
    flow: &mut FlowRuntime,
    carrier: WorkId,
    service: WorkId,
    resume_at: SimTime,
    scheduler_priority: i32,
) -> Vec<(
    (u64, u32),
    SimTime,
    TransitContextCheckpointV2,
    kairo_ecs_des::WorkProgress,
)> {
    schedule_transit_control(
        flow,
        carrier,
        KIND,
        FlowDomainControl::Resume,
        resume_at,
        scheduler_priority,
    )
    .unwrap();
    let mut suffix = Vec::new();
    for _ in 0..4 {
        let dispatch = step_ok(flow);
        let state = current_image(flow, carrier);
        suffix.push((
            (dispatch.event.index, dispatch.event.generation),
            dispatch.at,
            state.clone(),
            flow.work_progress(service).unwrap(),
        ));
        if state.phase == TransitPhase::Arrived {
            return suffix;
        }
    }
    panic!("bounded resumed transit suffix did not arrive")
}

fn submit_arrived_service(
    flow: &mut FlowRuntime,
    resource: kairo_ecs_des::ResourceId,
    owner: EntityId,
    service: WorkId,
    at: SimTime,
) -> kairo_ecs_des::RequestId {
    flow.acquire(resource)
        .owner(owner)
        .at(at)
        .timed_work(service)
        .submit()
        .unwrap()
}

type ServiceCompletionSnapshot = (
    (u64, u32),
    SimTime,
    RequestState,
    Option<kairo_ecs_des::LeaseId>,
    kairo_ecs_des::WorkProgress,
    kairo_ecs_des::ResourceSnapshot,
);

fn service_completion_suffix(
    flow: &mut FlowRuntime,
    request: kairo_ecs_des::RequestId,
    resource: kairo_ecs_des::ResourceId,
    service: WorkId,
) -> Vec<ServiceCompletionSnapshot> {
    let mut suffix = Vec::new();
    for _ in 0..4 {
        let dispatch = step_ok(flow);
        let state = flow.request(request).unwrap();
        suffix.push((
            (dispatch.event.index, dispatch.event.generation),
            dispatch.at,
            state.state,
            state.lease,
            flow.work_progress(service).unwrap(),
            flow.resource(resource).unwrap(),
        ));
        if state.state == RequestState::Completed {
            return suffix;
        }
    }
    panic!("bounded service did not complete")
}

#[test]
fn transit_checkpoint_v2_native_paused_retained_and_consumed_event() {
    let mut f = fixture(&[2]);
    start(&mut f.flow, f.carrier);
    schedule_transit_control(
        &mut f.flow,
        f.carrier,
        KIND,
        FlowDomainControl::Pause,
        SimTime::from_ticks(2),
        -2,
    )
    .unwrap();
    step_ok(&mut f.flow);
    let retained = capture(&f.flow, f.graph.clone());
    assert_eq!(retained.dto.phase, TransitPhase::Paused);
    assert_eq!(retained.dto.paused_from, Some(TransitPhase::Moving));
    assert!(retained.dto.expected_event.is_some());
    assert!(retained.dto.next_progress_ticket.is_some());
    let mut retained_restored = retained.restore().unwrap();
    let retained_carrier = retained_restored
        .actor_domain_context(f.carrier_actor)
        .unwrap();
    assert_eq!(retained_restored.now(), SimTime::from_ticks(2));
    let retained_source_suffix = resumed_suffix(
        &mut f.flow,
        f.carrier,
        f.service,
        SimTime::from_ticks(2),
        -3,
    );
    let retained_restored_suffix = resumed_suffix(
        &mut retained_restored,
        retained_carrier,
        f.service,
        SimTime::from_ticks(2),
        -3,
    );
    assert_eq!(retained_source_suffix, retained_restored_suffix);
    assert_eq!(
        current_image(&f.flow, f.carrier),
        current_image(&retained_restored, retained_carrier)
    );
    assert_eq!(
        f.flow.work_progress(f.service).unwrap(),
        retained_restored.work_progress(f.service).unwrap()
    );

    // Build an independent consumed-event cut from a fresh paused source.
    let mut f = fixture(&[2]);
    start(&mut f.flow, f.carrier);
    schedule_transit_control(
        &mut f.flow,
        f.carrier,
        KIND,
        FlowDomainControl::Pause,
        SimTime::from_ticks(2),
        -2,
    )
    .unwrap();
    step_ok(&mut f.flow);
    step_ok(&mut f.flow); // Consume the stale Progress callback while paused.
    let consumed = capture(&f.flow, f.graph.clone());
    assert_eq!(consumed.dto.phase, TransitPhase::Paused);
    assert_eq!(consumed.dto.expected_event, None);
    assert_eq!(consumed.dto.expected_due_ticks, None);
    assert_eq!(consumed.dto.next_progress_ticket, None);
    let mut restored = consumed.restore().unwrap();
    let restored_carrier = restored.actor_domain_context(f.carrier_actor).unwrap();
    assert_eq!(restored_carrier, f.carrier);
    let source_suffix = resumed_suffix(
        &mut f.flow,
        f.carrier,
        f.service,
        SimTime::from_ticks(5),
        -1,
    );
    let restored_suffix = resumed_suffix(
        &mut restored,
        restored_carrier,
        f.service,
        SimTime::from_ticks(5),
        -1,
    );
    assert_eq!(source_suffix, restored_suffix);
    assert_eq!(
        current_image(&f.flow, f.carrier),
        current_image(&restored, restored_carrier)
    );
    assert_eq!(
        f.flow.work_progress(f.service).unwrap(),
        restored.work_progress(f.service).unwrap()
    );
}

#[test]
fn transit_checkpoint_v2_native_arrived_unadmitted() {
    let mut f = fixture(&[1]);
    let arrival = arrive(&mut f.flow, f.carrier);
    let source = current_image(&f.flow, f.carrier);
    assert_eq!(source.phase, TransitPhase::Arrived);
    assert!(source.arrival_witness.is_some());
    assert_eq!(f.flow.work(f.service).unwrap().request, None);
    let restored = capture(&f.flow, f.graph.clone()).restore().unwrap();
    let carrier = restored.actor_domain_context(f.carrier_actor).unwrap();
    assert_eq!(current_image(&restored, carrier), source);
    assert!(pending_transit_arrival_handoff(&restored, carrier)
        .unwrap()
        .is_some());
    let observed = observe_transit_arrival_handoff(&restored, carrier, &arrival).unwrap();
    assert!(observed.is_some());
}

#[test]
fn transit_checkpoint_v2_native_arrived_admitted_suppression() {
    let mut f = fixture(&[1]);
    let arrival = arrive(&mut f.flow, f.carrier);
    let resource = f.flow.create_resource(1).unwrap();
    let captured = capture(&f.flow, f.graph.clone());
    assert_eq!(captured.dto.phase, TransitPhase::Arrived);
    let arrived_dto = captured.dto.clone();
    let mut restored = captured.restore().unwrap();
    let carrier = restored.actor_domain_context(f.carrier_actor).unwrap();
    assert_eq!(current_image(&restored, carrier), arrived_dto);

    let source_request =
        submit_arrived_service(&mut f.flow, resource, f.owner, f.service, arrival.at);
    let restored_request =
        submit_arrived_service(&mut restored, resource, f.owner, f.service, arrival.at);
    assert_eq!(source_request, restored_request);
    assert_eq!(
        f.flow.work(f.service).unwrap().request,
        Some(source_request)
    );
    assert_eq!(
        restored.work(f.service).unwrap().request,
        Some(restored_request)
    );
    let admitted_capture = capture(&f.flow, f.graph.clone());
    assert_eq!(admitted_capture.dto, arrived_dto);
    let mut admitted_restored = admitted_capture.restore().unwrap();
    let admitted_carrier = admitted_restored
        .actor_domain_context(f.carrier_actor)
        .unwrap();
    assert_eq!(
        admitted_restored.work(f.service).unwrap().request,
        Some(source_request)
    );
    assert_eq!(
        admitted_restored.request(source_request).unwrap().state,
        RequestState::Pending
    );
    assert!(
        pending_transit_arrival_handoff(&admitted_restored, admitted_carrier)
            .unwrap()
            .is_none()
    );
    assert!(
        observe_transit_arrival_handoff(&admitted_restored, admitted_carrier, &arrival)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        admitted_restored.work_progress(f.service).unwrap(),
        f.flow.work_progress(f.service).unwrap()
    );
    let source_progress_before = f.flow.work_progress(f.service).unwrap();
    let restored_progress_before = restored.work_progress(f.service).unwrap();
    assert_eq!(source_progress_before, restored_progress_before);
    let source_budget_before_duplicate = f.flow.budget_snapshot();
    let restored_budget_before_duplicate = restored.budget_snapshot();
    assert_eq!(
        f.flow
            .acquire(resource)
            .owner(f.owner)
            .at(arrival.at)
            .timed_work(f.service)
            .submit(),
        Err(kairo_ecs_des::FlowError::InvalidWork)
    );
    assert_eq!(
        restored
            .acquire(resource)
            .owner(f.owner)
            .at(arrival.at)
            .timed_work(f.service)
            .submit(),
        Err(kairo_ecs_des::FlowError::InvalidWork)
    );
    assert_eq!(f.flow.budget_snapshot(), source_budget_before_duplicate);
    assert_eq!(restored.budget_snapshot(), restored_budget_before_duplicate);
    assert!(pending_transit_arrival_handoff(&f.flow, f.carrier)
        .unwrap()
        .is_none());
    assert!(
        observe_transit_arrival_handoff(&f.flow, f.carrier, &arrival)
            .unwrap()
            .is_none()
    );
    assert!(pending_transit_arrival_handoff(&restored, carrier)
        .unwrap()
        .is_none());
    assert!(
        observe_transit_arrival_handoff(&restored, carrier, &arrival)
            .unwrap()
            .is_none()
    );

    let source_suffix = service_completion_suffix(&mut f.flow, source_request, resource, f.service);
    let restored_suffix =
        service_completion_suffix(&mut restored, restored_request, resource, f.service);
    let admitted_restored_suffix =
        service_completion_suffix(&mut admitted_restored, source_request, resource, f.service);
    assert_eq!(source_suffix, restored_suffix);
    assert_eq!(source_suffix, admitted_restored_suffix);
    assert_eq!(source_suffix.len(), 2, "one grant and one timed completion");
    assert_eq!(source_suffix[0].1, arrival.at);
    assert_eq!(source_suffix[0].2, RequestState::Active);
    assert_eq!(source_suffix[1].2, RequestState::Completed);
    assert_eq!(
        source_suffix[1].1,
        arrival.at.checked_add(SimDuration::from_ticks(4)).unwrap()
    );
    assert_eq!(current_image(&f.flow, f.carrier), arrived_dto);
    assert_eq!(current_image(&restored, carrier), arrived_dto);
    assert_eq!(
        current_image(&admitted_restored, admitted_carrier),
        arrived_dto
    );
    assert!(pending_transit_arrival_handoff(&f.flow, f.carrier)
        .unwrap()
        .is_none());
    assert!(
        observe_transit_arrival_handoff(&f.flow, f.carrier, &arrival)
            .unwrap()
            .is_none()
    );
    assert!(pending_transit_arrival_handoff(&restored, carrier)
        .unwrap()
        .is_none());
    assert!(
        observe_transit_arrival_handoff(&restored, carrier, &arrival)
            .unwrap()
            .is_none()
    );
    assert!(
        pending_transit_arrival_handoff(&admitted_restored, admitted_carrier)
            .unwrap()
            .is_none()
    );
    assert!(
        observe_transit_arrival_handoff(&admitted_restored, admitted_carrier, &arrival)
            .unwrap()
            .is_none()
    );
}

#[test]
fn transit_checkpoint_v2_native_frontier_resume_live_domain_noop() {
    let mut f = fixture(&[2]);
    start(&mut f.flow, f.carrier);
    let old_live = current_image(&f.flow, f.carrier).expected_event.unwrap();
    let at = SimTime::from_ticks(2);
    schedule_transit_control(
        &mut f.flow,
        f.carrier,
        KIND,
        FlowDomainControl::Pause,
        at,
        -2,
    )
    .unwrap();
    let resume = schedule_transit_control(
        &mut f.flow,
        f.carrier,
        KIND,
        FlowDomainControl::Resume,
        at,
        -1,
    )
    .unwrap();
    step_ok(&mut f.flow);
    let arrival = step_ok(&mut f.flow);
    assert_eq!(arrival.event, resume);
    let captured = capture(&f.flow, f.graph.clone());
    assert_eq!(captured.dto.phase, TransitPhase::Arrived);
    assert_eq!(captured.dto.expected_event, None);
    assert_eq!(
        captured.dto.arrival_witness.unwrap().cause,
        TransitArrivalCauseCheckpointV2::Resume
    );
    assert_eq!(
        captured.dto.arrival_witness.unwrap().event,
        (resume.index, resume.generation)
    );
    let mut restored = captured.restore().unwrap();
    let carrier = restored.actor_domain_context(f.carrier_actor).unwrap();
    let before_stale_dto = current_image(&restored, carrier);
    let before_stale_service = restored.work(f.service).unwrap();
    let before_stale_progress = restored.work_progress(f.service).unwrap();
    let stale = step_ok(&mut restored);
    assert_eq!((stale.event.index, stale.event.generation), old_live);
    assert_eq!(
        restored
            .work_context::<TransitContext>(carrier)
            .unwrap()
            .phase(),
        TransitPhase::Arrived
    );
    let after_stale_dto = current_image(&restored, carrier);
    assert_eq!(after_stale_dto, before_stale_dto);
    assert_eq!(
        after_stale_dto.arrival_witness.unwrap().event,
        (resume.index, resume.generation)
    );
    assert_eq!(restored.work(f.service).unwrap(), before_stale_service);
    assert_eq!(
        restored.work_progress(f.service).unwrap(),
        before_stale_progress
    );
    assert!(after_stale_dto.expected_event.is_none());
    assert!(restored.work(f.service).unwrap().request.is_none());
}

#[test]
fn transit_checkpoint_v2_native_historical_only_handoff_binding_rejected() {
    let mut f = fixture(&[1]);
    f.flow.despawn_actor(f.owner).unwrap();
    step_ok(&mut f.flow);
    let captured = capture(&f.flow, f.graph.clone());
    assert_eq!(captured.dto.service_owner, f.owner);
    assert!(matches!(
        captured.dto.action,
        TransitArrivalActionCheckpointV2::Handoff { .. }
    ));
    let dto = captured.dto.clone();
    let (restore, native_error) = captured.restore_with(dto);
    assert!(restore.is_err());
    assert_eq!(
        native_error,
        Some(TransitContextCheckpointError::InvalidReference)
    );
}

#[test]
fn transit_checkpoint_v2_native_acquire_historical_refs_and_v1_bytes() {
    let mut flow = FlowRuntime::new();
    register_transit_context(&mut flow, "native-v2", KIND).unwrap();
    let owner = flow.spawn_actor().unwrap();
    let carrier_actor = flow.spawn_actor().unwrap();
    let resource = flow.create_resource(1).unwrap();
    let service = flow
        .create_work(owner, SimDuration::from_ticks(4), "service", ())
        .unwrap();
    let (graph, route) = graph_route(&[2]);
    let legacy = FlowAcquireCommand {
        resource,
        owner,
        work: Some(service),
        at: SimTime::ZERO,
        priority_level: 4,
        deadline: None,
        scheduler_priority: 1,
        timed: true,
        can_preempt: false,
        preemptible: None,
    };
    let context = TransitContext::new(&flow, route, legacy, SimTime::ZERO).unwrap();
    let carrier = flow
        .create_actor_domain_context(carrier_actor, "native-v2", KIND, context)
        .unwrap();
    let original = flow.work_context::<TransitContext>(carrier).unwrap();
    let v1 = original.checkpoint_v1(&flow.identity(), LIMITS).unwrap();
    assert_eq!(v1.schema_version, 1);
    assert!(original
        .checkpoint_bytes_v1(&flow.identity(), LIMITS)
        .unwrap()
        .starts_with(b"KTCXCPV1"));
    let v2 = original.checkpoint_v2(&flow.identity(), LIMITS).unwrap();
    assert!(
        matches!(v2.action, TransitArrivalActionCheckpointV2::Acquire { owner: saved_owner, work: Some(saved_work), .. } if saved_owner == owner && saved_work == service.entity_id())
    );

    // A real completed request retains historical owner/work evidence while
    // the legacy context stays at its original Ready cut.
    let request = submit_arrived_service(&mut flow, resource, owner, service, SimTime::ZERO);
    for _ in 0..4 {
        if flow.request(request).unwrap().state == RequestState::Completed {
            break;
        }
        step_ok(&mut flow);
    }
    assert_eq!(
        flow.request(request).unwrap().state,
        RequestState::Completed
    );
    flow.despawn_actor(owner).unwrap();
    for _ in 0..8 {
        if flow.work(service).is_err() {
            break;
        }
        step_ok(&mut flow);
    }
    assert!(flow.work(service).is_err());
    assert_eq!(
        flow.request(request).unwrap().state,
        RequestState::Completed
    );
    let captured = capture(&flow, Arc::new(graph));
    assert!(matches!(
        captured.dto.action,
        TransitArrivalActionCheckpointV2::Acquire { .. }
    ));
    let restored = captured.restore().unwrap();
    let restored_carrier = restored.actor_domain_context(carrier_actor).unwrap();
    assert_eq!(
        restored
            .work_context::<TransitContext>(restored_carrier)
            .unwrap()
            .phase(),
        TransitPhase::Ready
    );

    // A separate legacy context reaches the route frontier only when Resume
    // arrives at tick five, preserving V1 time and real Acquire admission.
    let mut late_flow = FlowRuntime::new();
    register_transit_context(&mut late_flow, "native-v2", KIND).unwrap();
    let late_owner = late_flow.spawn_actor().unwrap();
    let late_carrier_actor = late_flow.spawn_actor().unwrap();
    let late_resource = late_flow.create_resource(1).unwrap();
    let late_service = late_flow
        .create_work(late_owner, SimDuration::from_ticks(4), "service", ())
        .unwrap();
    let (_, late_route) = graph_route(&[2]);
    let late_context = TransitContext::new(
        &late_flow,
        late_route,
        FlowAcquireCommand {
            resource: late_resource,
            owner: late_owner,
            work: Some(late_service),
            at: SimTime::ZERO,
            priority_level: 4,
            deadline: None,
            scheduler_priority: 1,
            timed: true,
            can_preempt: false,
            preemptible: None,
        },
        SimTime::ZERO,
    )
    .unwrap();
    let late_carrier = late_flow
        .create_actor_domain_context(late_carrier_actor, "native-v2", KIND, late_context)
        .unwrap();
    schedule_transit_start(&mut late_flow, late_carrier, KIND, SimTime::ZERO, 0).unwrap();
    step_ok(&mut late_flow);
    schedule_transit_control(
        &mut late_flow,
        late_carrier,
        KIND,
        FlowDomainControl::Pause,
        SimTime::from_ticks(2),
        -2,
    )
    .unwrap();
    step_ok(&mut late_flow);
    step_ok(&mut late_flow); // Consume the stale Progress event while paused.
    schedule_transit_control(
        &mut late_flow,
        late_carrier,
        KIND,
        FlowDomainControl::Resume,
        SimTime::from_ticks(5),
        -1,
    )
    .unwrap();
    let late_arrival = step_ok(&mut late_flow);
    assert_eq!(late_arrival.at, SimTime::from_ticks(5));
    assert_eq!(
        late_flow
            .work_context::<TransitContext>(late_carrier)
            .unwrap()
            .phase(),
        TransitPhase::Arrived
    );
    let late_v1 = late_flow
        .work_context::<TransitContext>(late_carrier)
        .unwrap()
        .checkpoint_v1(&late_flow.identity(), LIMITS)
        .unwrap();
    assert_eq!(late_v1.last_advanced_at_ticks, 5);
    assert!(late_flow
        .work_context::<TransitContext>(late_carrier)
        .unwrap()
        .checkpoint_bytes_v1(&late_flow.identity(), LIMITS)
        .unwrap()
        .starts_with(b"KTCXCPV1"));
    let kairo_ecs_des::FlowBatchReceipt::Accepted(admissions) = &late_arrival.callback_batches[0]
    else {
        panic!("late Resume must accept the legacy Acquire");
    };
    assert_eq!(admissions.len(), 1);
    let late_request = admissions[0].request.unwrap();
    assert_eq!(
        late_flow.work(late_service).unwrap().request,
        Some(late_request)
    );
    for _ in 0..4 {
        if late_flow.request(late_request).unwrap().state == RequestState::Completed {
            break;
        }
        step_ok(&mut late_flow);
    }
    assert_eq!(
        late_flow.request(late_request).unwrap().state,
        RequestState::Completed
    );
}

#[test]
fn transit_checkpoint_v2_native_bounds_versions_and_atomic_restore() {
    let f = fixture(&[1]);
    let before = current_image(&f.flow, f.carrier);
    let segments = before.route.segments.len();
    let graph_bytes = before.route.graph_canonical_bytes.len();
    let mode_bytes = before.route.movement_mode.len();
    let total_bytes = 512 + segments * 64 + graph_bytes + mode_bytes;
    assert_eq!(
        f.flow
            .work_context::<TransitContext>(f.carrier)
            .unwrap()
            .checkpoint_v2(
                &f.flow.identity(),
                TransitContextCheckpointLimitsV1::new(
                    segments,
                    graph_bytes,
                    mode_bytes,
                    total_bytes
                )
            )
            .unwrap(),
        before
    );
    for limits in [
        TransitContextCheckpointLimitsV1::new(segments - 1, graph_bytes, mode_bytes, total_bytes),
        TransitContextCheckpointLimitsV1::new(segments, graph_bytes - 1, mode_bytes, total_bytes),
        TransitContextCheckpointLimitsV1::new(segments, graph_bytes, mode_bytes - 1, total_bytes),
        TransitContextCheckpointLimitsV1::new(segments, graph_bytes, mode_bytes, total_bytes - 1),
    ] {
        assert_eq!(
            f.flow
                .work_context::<TransitContext>(f.carrier)
                .unwrap()
                .checkpoint_v2(&f.flow.identity(), limits),
            Err(TransitContextCheckpointError::LimitExceeded)
        );
    }
    assert_eq!(
        f.flow
            .work_context::<TransitContext>(f.carrier)
            .unwrap()
            .checkpoint_v2(
                &f.flow.identity(),
                TransitContextCheckpointLimitsV1::new(0, 0, 0, 0)
            ),
        Err(TransitContextCheckpointError::LimitExceeded)
    );
    assert_eq!(current_image(&f.flow, f.carrier), before);
    let captured = capture(&f.flow, f.graph.clone());
    let mut unsupported = captured.dto.clone();
    unsupported.schema_version = 3;
    let (restored, error) = captured.restore_with(unsupported);
    assert!(restored.is_err());
    assert_eq!(
        error,
        Some(TransitContextCheckpointError::UnsupportedVersion(3))
    );

    let captured = capture(&f.flow, f.graph.clone());
    let mut malformed_pair = captured.dto.clone();
    malformed_pair.expected_due_ticks = Some(4);
    let (restored, error) = captured.restore_with(malformed_pair);
    assert!(restored.is_err());
    assert_eq!(error, Some(TransitContextCheckpointError::InvalidState));
    assert_eq!(current_image(&f.flow, f.carrier), before);

    let captured = capture(&f.flow, f.graph.clone());
    let mut command_ticket = captured.dto.clone();
    command_ticket.command_ticket = Some((
        1,
        0,
        kairo_ecs_abm::TransitCommandPurposeCheckpointV1::Arrival,
    ));
    let (restored, error) = captured.restore_with(command_ticket);
    assert!(restored.is_err());
    assert_eq!(error, Some(TransitContextCheckpointError::InvalidState));
    assert_eq!(current_image(&f.flow, f.carrier), before);

    let captured = capture(&f.flow, f.graph.clone());
    let mut wrong_reference = captured.dto.clone();
    wrong_reference.service_work = EntityId::new(u64::MAX, 0);
    let (restored, error) = captured.restore_with(wrong_reference);
    assert!(restored.is_err());
    assert_eq!(error, Some(TransitContextCheckpointError::InvalidReference));

    let mut moving = fixture(&[2]);
    start(&mut moving.flow, moving.carrier);
    let captured = capture(&moving.flow, moving.graph.clone());
    let mut invalid_ticket = captured.dto.clone();
    let (batch, index) = invalid_ticket.next_progress_ticket.unwrap();
    let max_callback_commands = captured.image.callback_config.max_callback_commands.get();
    assert!(index < max_callback_commands);
    invalid_ticket.next_progress_ticket = Some((batch, max_callback_commands));
    let (restored, error) = captured.restore_with(invalid_ticket);
    assert!(restored.is_err());
    assert_eq!(error, Some(TransitContextCheckpointError::InvalidTicket));
}
