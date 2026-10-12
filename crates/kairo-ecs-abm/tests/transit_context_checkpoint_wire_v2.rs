include!("transit_context_checkpoint_v2_native.rs");

use kairo_ecs_abm::spatial::{RoutePlanImageV1, RouteSegmentImageV1};
use kairo_ecs_abm::TransitContextCheckpointV1;
use kairo_ecs_abm::{register_transit_context_checkpoint_codec_v2, TransitContextWireError};
use kairo_ecs_des::FlowCheckpointError;

const WIRE_CODEC: &str = "test.transit-context-v2-wire";
const WIRE_LIMITS: TransitContextCheckpointLimitsV1 =
    TransitContextCheckpointLimitsV1::new(16, 4096, 64, 8192);

fn wire_codecs_with_limits(
    source: &FlowRuntime,
    graph: Arc<TransitGraphV1>,
    limits: TransitContextCheckpointLimitsV1,
) -> FlowCheckpointCodecs {
    let mut codecs = FlowCheckpointCodecs::new();
    register_transit_context_checkpoint_codec_v2(
        &mut codecs,
        WIRE_CODEC,
        source.identity(),
        graph,
        limits,
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

fn wire_codecs(source: &FlowRuntime, graph: Arc<TransitGraphV1>) -> FlowCheckpointCodecs {
    wire_codecs_with_limits(source, graph, WIRE_LIMITS)
}

fn wire_capture(
    flow: &FlowRuntime,
    graph: Arc<TransitGraphV1>,
) -> (
    kairo_ecs_des::FlowCheckpointV1,
    FlowCheckpointCodecs,
    Vec<u8>,
) {
    let codecs = wire_codecs(flow, graph);
    let image = flow
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    let store = image
        .context_stores
        .iter()
        .find(|store| store.codec_key == WIRE_CODEC)
        .unwrap();
    assert_eq!(store.rows.len(), 1);
    let bytes = store.rows[0].1.clone();
    (image, codecs, bytes)
}

fn acquire_fixture(lengths: &[u64]) -> Fixture {
    let mut flow = FlowRuntime::new();
    register_transit_context(&mut flow, "native-v2", KIND).unwrap();
    let owner = flow.spawn_actor().unwrap();
    let carrier_actor = flow.spawn_actor().unwrap();
    let resource = flow.create_resource(1).unwrap();
    let service = flow
        .create_work(owner, SimDuration::from_ticks(4), "service", ())
        .unwrap();
    let (graph, route) = graph_route(lengths);
    let context = TransitContext::new(
        &flow,
        route,
        FlowAcquireCommand {
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
        },
        SimTime::ZERO,
    )
    .unwrap();
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

fn restore_wire(
    image: kairo_ecs_des::FlowCheckpointV1,
    codecs: &FlowCheckpointCodecs,
) -> Result<(FlowRuntime, kairo_ecs_des::FlowCheckpointRebindV1), FlowCheckpointError> {
    FlowRuntime::restore_checkpoint_with_rebind(image, codecs, FlowCheckpointLimits::default())
}

fn hex_bytes(text: &str) -> Vec<u8> {
    let (pairs, remainder) = text.as_bytes().as_chunks::<2>();
    assert!(remainder.is_empty());
    pairs
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn literal_ready_dto() -> TransitContextCheckpointV2 {
    TransitContextCheckpointV2 {
        schema_version: 2,
        route: RoutePlanImageV1 {
            schema_version: 1,
            origin: 1,
            destination: 1,
            segments: Vec::new(),
            distance_mm: 0,
            duration_ticks: 0,
            movement_mode: "walk".to_owned(),
            speed_mm_per_second: 1,
            ticks_per_second: 1,
            graph_version: 1,
            graph_canonical_bytes: Vec::new(),
        },
        segment_index: 0,
        elapsed_in_segment_ticks: 0,
        service_work: EntityId::new(2, 0),
        service_owner: EntityId::new(1, 0),
        action: TransitArrivalActionCheckpointV2::Acquire {
            resource: EntityId::new(3, 0),
            owner: EntityId::new(1, 0),
            work: Some(EntityId::new(2, 0)),
            at_ticks: 0,
            priority_level: 0,
            deadline_ticks: None,
            scheduler_priority: 0,
            timed: true,
            can_preempt: false,
            preemptible: None,
        },
        arrival_witness: None,
        carrier: None,
        kind: None,
        start_at_ticks: 0,
        phase: TransitPhase::Ready,
        paused_from: None,
        last_advanced_at_ticks: 0,
        initial_start_pending: true,
        expected_event: None,
        expected_due_ticks: None,
        command_ticket: None,
        arrival_ticket: None,
        next_progress_ticket: None,
    }
}

#[test]
fn transit_v2_wire_actual_ready_moving_roundtrip() {
    let mut f = fixture(&[2, 1]);
    let ready = f.flow.work_context::<TransitContext>(f.carrier).unwrap();
    let ready_bytes = ready
        .checkpoint_bytes_v2(&f.flow.identity(), WIRE_LIMITS)
        .unwrap();
    assert_eq!(
        TransitContextCheckpointV2::decode_bytes_v2(&ready_bytes, WIRE_LIMITS).unwrap(),
        current_image(&f.flow, f.carrier)
    );
    let mut scheduled = fixture(&[2, 1]);
    schedule_transit_start(
        &mut scheduled.flow,
        scheduled.carrier,
        KIND,
        SimTime::ZERO,
        0,
    )
    .unwrap();
    let (scheduled_image, scheduled_codecs, scheduled_bytes) =
        wire_capture(&scheduled.flow, scheduled.graph.clone());
    assert!(
        TransitContextCheckpointV2::decode_bytes_v2(&scheduled_bytes, WIRE_LIMITS)
            .unwrap()
            .expected_event
            .is_some()
    );
    let scheduled_restored = restore_wire(scheduled_image, &scheduled_codecs).unwrap().0;
    let scheduled_carrier = scheduled_restored
        .actor_domain_context(f.carrier_actor)
        .unwrap();
    assert_eq!(
        current_image(&scheduled_restored, scheduled_carrier),
        current_image(&scheduled.flow, scheduled.carrier)
    );

    let mut resumed_ready = acquire_fixture(&[2]);
    schedule_transit_start(
        &mut resumed_ready.flow,
        resumed_ready.carrier,
        KIND,
        SimTime::ZERO,
        0,
    )
    .unwrap();
    schedule_transit_control(
        &mut resumed_ready.flow,
        resumed_ready.carrier,
        KIND,
        FlowDomainControl::Pause,
        SimTime::ZERO,
        -1,
    )
    .unwrap();
    step_ok(&mut resumed_ready.flow);
    step_ok(&mut resumed_ready.flow);
    schedule_transit_control(
        &mut resumed_ready.flow,
        resumed_ready.carrier,
        KIND,
        FlowDomainControl::Resume,
        SimTime::from_ticks(5),
        -1,
    )
    .unwrap();
    step_ok(&mut resumed_ready.flow);
    let (resumed_image, resumed_codecs, resumed_bytes) =
        wire_capture(&resumed_ready.flow, resumed_ready.graph.clone());
    let resumed_dto =
        TransitContextCheckpointV2::decode_bytes_v2(&resumed_bytes, WIRE_LIMITS).unwrap();
    assert_eq!(resumed_dto.phase, TransitPhase::Ready);
    assert_eq!(resumed_dto.expected_due_ticks, Some(5));
    let resumed_restored = restore_wire(resumed_image, &resumed_codecs).unwrap().0;
    let resumed_carrier = resumed_restored
        .actor_domain_context(resumed_ready.carrier_actor)
        .unwrap();
    assert_eq!(
        current_image(&resumed_restored, resumed_carrier),
        resumed_dto
    );
    start(&mut f.flow, f.carrier);
    let (image, codecs, bytes) = wire_capture(&f.flow, f.graph.clone());
    assert_eq!(image.version, 1);
    assert_eq!(
        TransitContextCheckpointV2::decode_bytes_v2(&bytes, WIRE_LIMITS).unwrap(),
        current_image(&f.flow, f.carrier)
    );
    let (mut restored, _) = restore_wire(image, &codecs).unwrap();
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
}

#[test]
fn transit_v2_wire_paused_resume_suffix() {
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
    let (image, codecs, bytes) = wire_capture(&f.flow, f.graph.clone());
    assert_eq!(
        TransitContextCheckpointV2::decode_bytes_v2(&bytes, WIRE_LIMITS)
            .unwrap()
            .phase,
        TransitPhase::Paused
    );
    let (mut restored, _) = restore_wire(image, &codecs).unwrap();
    let restored_carrier = restored.actor_domain_context(f.carrier_actor).unwrap();
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
    let mut consumed = fixture(&[2]);
    start(&mut consumed.flow, consumed.carrier);
    schedule_transit_control(
        &mut consumed.flow,
        consumed.carrier,
        KIND,
        FlowDomainControl::Pause,
        SimTime::from_ticks(2),
        -2,
    )
    .unwrap();
    step_ok(&mut consumed.flow);
    step_ok(&mut consumed.flow); // The old Progress event is consumed while paused.
    let (consumed_image, consumed_codecs, consumed_bytes) =
        wire_capture(&consumed.flow, consumed.graph.clone());
    let consumed_dto =
        TransitContextCheckpointV2::decode_bytes_v2(&consumed_bytes, WIRE_LIMITS).unwrap();
    assert_eq!(consumed_dto.phase, TransitPhase::Paused);
    assert_eq!(consumed_dto.expected_event, None);
    assert_eq!(consumed_dto.next_progress_ticket, None);
    let (mut consumed_restored, _) = restore_wire(consumed_image, &consumed_codecs).unwrap();
    let consumed_carrier = consumed_restored
        .actor_domain_context(consumed.carrier_actor)
        .unwrap();
    let source_suffix = resumed_suffix(
        &mut consumed.flow,
        consumed.carrier,
        consumed.service,
        SimTime::from_ticks(5),
        -1,
    );
    let restored_suffix = resumed_suffix(
        &mut consumed_restored,
        consumed_carrier,
        consumed.service,
        SimTime::from_ticks(5),
        -1,
    );
    assert_eq!(source_suffix, restored_suffix);
    assert_eq!(
        current_image(&consumed.flow, consumed.carrier),
        current_image(&consumed_restored, consumed_carrier)
    );
}

#[test]
fn transit_v2_wire_arrived_binding_and_suppression() {
    let mut f = fixture(&[1]);
    let arrival = arrive(&mut f.flow, f.carrier);
    let resource = f.flow.create_resource(1).unwrap();
    let (unadmitted_image, unadmitted_codecs, _) = wire_capture(&f.flow, f.graph.clone());
    let (mut unadmitted_restored, _) = restore_wire(unadmitted_image, &unadmitted_codecs).unwrap();
    let unadmitted_carrier = unadmitted_restored
        .actor_domain_context(f.carrier_actor)
        .unwrap();
    assert!(
        pending_transit_arrival_handoff(&unadmitted_restored, unadmitted_carrier)
            .unwrap()
            .is_some()
    );
    assert!(
        observe_transit_arrival_handoff(&unadmitted_restored, unadmitted_carrier, &arrival)
            .unwrap()
            .is_some()
    );
    let source_request =
        submit_arrived_service(&mut f.flow, resource, f.owner, f.service, arrival.at);
    let unadmitted_request = submit_arrived_service(
        &mut unadmitted_restored,
        resource,
        f.owner,
        f.service,
        arrival.at,
    );
    assert_eq!(source_request, unadmitted_request);

    let (admitted_image, admitted_codecs, admitted_bytes) = wire_capture(&f.flow, f.graph.clone());
    let (mut admitted_restored, _) = restore_wire(admitted_image, &admitted_codecs).unwrap();
    let admitted_carrier = admitted_restored
        .actor_domain_context(f.carrier_actor)
        .unwrap();
    assert_eq!(
        TransitContextCheckpointV2::decode_bytes_v2(&admitted_bytes, WIRE_LIMITS)
            .unwrap()
            .phase,
        TransitPhase::Arrived
    );
    assert_eq!(
        admitted_restored.work(f.service).unwrap().request,
        Some(source_request)
    );
    assert_eq!(
        admitted_restored.request(source_request).unwrap().state,
        RequestState::Pending
    );
    for (flow, carrier) in [
        (&f.flow, f.carrier),
        (&unadmitted_restored, unadmitted_carrier),
        (&admitted_restored, admitted_carrier),
    ] {
        assert!(pending_transit_arrival_handoff(flow, carrier)
            .unwrap()
            .is_none());
        assert!(observe_transit_arrival_handoff(flow, carrier, &arrival)
            .unwrap()
            .is_none());
    }
    let source_suffix = service_completion_suffix(&mut f.flow, source_request, resource, f.service);
    let unadmitted_suffix = service_completion_suffix(
        &mut unadmitted_restored,
        unadmitted_request,
        resource,
        f.service,
    );
    let admitted_suffix =
        service_completion_suffix(&mut admitted_restored, source_request, resource, f.service);
    assert_eq!(source_suffix, unadmitted_suffix);
    assert_eq!(source_suffix, admitted_suffix);
    assert_eq!(source_suffix.len(), 2);
    assert_eq!(source_suffix[0].2, RequestState::Active);
    assert_eq!(source_suffix[1].2, RequestState::Completed);
    for (flow, carrier) in [
        (&f.flow, f.carrier),
        (&unadmitted_restored, unadmitted_carrier),
        (&admitted_restored, admitted_carrier),
    ] {
        assert!(pending_transit_arrival_handoff(flow, carrier)
            .unwrap()
            .is_none());
        assert!(observe_transit_arrival_handoff(flow, carrier, &arrival)
            .unwrap()
            .is_none());
    }
}

#[test]
fn transit_v2_wire_resume_witness_live_stale_noop() {
    let mut f = fixture(&[5]);
    start(&mut f.flow, f.carrier);
    let old_live = current_image(&f.flow, f.carrier).expected_event.unwrap();
    schedule_transit_control(
        &mut f.flow,
        f.carrier,
        KIND,
        FlowDomainControl::Pause,
        SimTime::from_ticks(5),
        -2,
    )
    .unwrap();
    let pause_dispatch = step_ok(&mut f.flow);
    assert_eq!(pause_dispatch.at, SimTime::from_ticks(5));
    let resume = schedule_transit_control(
        &mut f.flow,
        f.carrier,
        KIND,
        FlowDomainControl::Resume,
        SimTime::from_ticks(5),
        -1,
    )
    .unwrap();
    let resume_dispatch = step_ok(&mut f.flow);
    assert_eq!(resume_dispatch.event, resume);
    assert_eq!(resume_dispatch.at, SimTime::from_ticks(5));
    assert_eq!(
        current_image(&f.flow, f.carrier).phase,
        TransitPhase::Arrived
    );
    let source_arrived = current_image(&f.flow, f.carrier);
    assert_eq!(source_arrived.arrival_witness.unwrap().at_ticks, 5);
    assert_eq!(source_arrived.last_advanced_at_ticks, 5);
    let (image, codecs, _) = wire_capture(&f.flow, f.graph.clone());
    let (mut restored, _) = restore_wire(image, &codecs).unwrap();
    let carrier = restored.actor_domain_context(f.carrier_actor).unwrap();
    let before = current_image(&restored, carrier);
    assert_eq!(before.arrival_witness.unwrap().at_ticks, 5);
    assert_eq!(before.last_advanced_at_ticks, 5);
    assert_eq!(
        before.arrival_witness.unwrap().cause,
        TransitArrivalCauseCheckpointV2::Resume
    );
    let service_before = restored.work(f.service).unwrap();
    let progress_before = restored.work_progress(f.service).unwrap();
    let source_before = current_image(&f.flow, f.carrier);
    let source_service_before = f.flow.work(f.service).unwrap();
    let source_progress_before = f.flow.work_progress(f.service).unwrap();
    let stale_restored = step_ok(&mut restored);
    let stale_source = step_ok(&mut f.flow);
    assert_eq!(
        (stale_restored.event.index, stale_restored.event.generation),
        old_live
    );
    assert_eq!(
        (stale_source.event.index, stale_source.event.generation),
        old_live
    );
    assert_eq!(current_image(&restored, carrier), before);
    assert_eq!(restored.work(f.service).unwrap(), service_before);
    assert_eq!(restored.work_progress(f.service).unwrap(), progress_before);
    assert_eq!(current_image(&f.flow, f.carrier), source_before);
    assert_eq!(f.flow.work(f.service).unwrap(), source_service_before);
    assert_eq!(
        f.flow.work_progress(f.service).unwrap(),
        source_progress_before
    );
}

#[test]
fn transit_v2_wire_literal_bytes_and_v1_compatibility() {
    const GOLDEN: &str = "4b544358435056320100000002000000010000000100000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000040000000000000077616c6b0100000000000000010000000000000001000000000000000000000000000000000000000000000000000000000000000000000002000000000000000000000001000000000000000000000000030000000000000000000000010000000000000000000000010200000000000000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000";
    let golden = hex_bytes(GOLDEN);
    assert_eq!(golden.len(), 273);
    assert_eq!(
        literal_ready_dto().encode_bytes_v2(WIRE_LIMITS).unwrap(),
        golden
    );
    assert_eq!(
        TransitContextCheckpointV2::decode_bytes_v2(&golden, WIRE_LIMITS).unwrap(),
        literal_ready_dto()
    );
    let f = acquire_fixture(&[1]);
    let context = f.flow.work_context::<TransitContext>(f.carrier).unwrap();
    let v1 = context
        .checkpoint_bytes_v1(&f.flow.identity(), WIRE_LIMITS)
        .unwrap();
    let v2 = context
        .checkpoint_bytes_v2(&f.flow.identity(), WIRE_LIMITS)
        .unwrap();
    assert_ne!(v1, v2);
    let legacy = TransitContextCheckpointV1::decode_bytes_v1(&v1, WIRE_LIMITS).unwrap();
    assert_eq!(legacy.schema_version, 1);
    let mut arrived = fixture(&[1]);
    let _ = arrive(&mut arrived.flow, arrived.carrier);
    assert!(matches!(
        arrived
            .flow
            .work_context::<TransitContext>(arrived.carrier)
            .unwrap()
            .checkpoint_bytes_v1(&arrived.flow.identity(), WIRE_LIMITS),
        Err(TransitContextWireError::Native(
            TransitContextCheckpointError::InvalidState
        ))
    ));
}

#[test]
fn transit_v2_wire_exact_total_budget_real_rebind() {
    let mut f = fixture(&[1]);
    let context = f.flow.work_context::<TransitContext>(f.carrier).unwrap();
    let bytes = context
        .checkpoint_bytes_v2(&f.flow.identity(), WIRE_LIMITS)
        .unwrap();
    assert!(
        bytes.len() < 512,
        "small Handoff image should fit below native envelope estimate"
    );
    let exact = TransitContextCheckpointLimitsV1::new(16, 4096, 64, bytes.len());
    let outer_limits = FlowCheckpointLimits {
        max_payload_bytes: bytes.len(),
        ..FlowCheckpointLimits::default()
    };
    let codecs = wire_codecs_with_limits(&f.flow, f.graph.clone(), exact);
    let outer = f.flow.capture_checkpoint(&codecs, outer_limits).unwrap();
    let row = outer
        .context_stores
        .iter()
        .find(|store| store.codec_key == WIRE_CODEC)
        .unwrap();
    assert_eq!(row.rows[0].1, bytes);
    let (mut rebound, rebind) =
        FlowRuntime::restore_checkpoint_with_rebind(outer, &codecs, outer_limits).unwrap();
    assert_eq!(rebound.now(), SimTime::ZERO);
    let dto = TransitContextCheckpointV2::decode_bytes_v2(&bytes, exact).unwrap();
    let rebound_carrier = rebound.actor_domain_context(f.carrier_actor).unwrap();
    let direct_restored = TransitContextCheckpointV2::restore_bytes_v2(
        &bytes,
        &f.graph,
        &rebind,
        rebound_carrier.entity_id(),
        exact,
    )
    .unwrap();
    assert_eq!(
        direct_restored
            .checkpoint_v2(rebind.identity(), WIRE_LIMITS)
            .unwrap(),
        current_image(&rebound, rebound_carrier)
    );
    assert_eq!(dto.encode_bytes_v2(exact).unwrap(), bytes);
    let short = TransitContextCheckpointLimitsV1::new(16, 4096, 64, bytes.len() - 1);
    assert_eq!(
        dto.encode_bytes_v2(short),
        Err(TransitContextWireError::LimitExceeded)
    );
    assert_eq!(
        TransitContextCheckpointV2::decode_bytes_v2(&bytes, exact).unwrap(),
        dto
    );
    assert_eq!(
        TransitContextCheckpointV2::decode_bytes_v2(&bytes, short),
        Err(TransitContextWireError::LimitExceeded)
    );
    assert!(matches!(
        TransitContextCheckpointV2::restore_bytes_v2(
            &bytes,
            &f.graph,
            &rebind,
            rebound_carrier.entity_id(),
            short,
        ),
        Err(TransitContextWireError::LimitExceeded)
    ));
    assert_eq!(
        context.checkpoint_bytes_v2(&f.flow.identity(), short),
        Err(TransitContextWireError::LimitExceeded)
    );
    let short_outer = FlowCheckpointLimits {
        max_payload_bytes: bytes.len() - 1,
        ..FlowCheckpointLimits::default()
    };
    let short_codecs = wire_codecs_with_limits(&f.flow, f.graph.clone(), short);
    assert!(f
        .flow
        .capture_checkpoint(&short_codecs, short_outer)
        .is_err());
    start(&mut f.flow, f.carrier);
    start(&mut rebound, rebound_carrier);
    let source_arrival = arrive_after_start(&mut f.flow, f.carrier);
    let rebound_arrival = arrive_after_start(&mut rebound, rebound_carrier);
    assert_eq!(
        (source_arrival.event, source_arrival.at),
        (rebound_arrival.event, rebound_arrival.at)
    );
    assert_eq!(
        current_image(&f.flow, f.carrier),
        current_image(&rebound, rebound_carrier)
    );
    let overflow = TransitContextCheckpointLimitsV1::new(16, 4096, 64, usize::MAX);
    assert!(matches!(
        TransitContextCheckpointV2::restore_bytes_v2(
            &bytes,
            &f.graph,
            &rebind,
            rebound_carrier.entity_id(),
            overflow,
        ),
        Err(TransitContextWireError::LimitExceeded)
    ));
    assert_eq!(
        f.flow
            .work_context::<TransitContext>(f.carrier)
            .unwrap()
            .checkpoint_bytes_v2(&f.flow.identity(), overflow),
        Err(TransitContextWireError::LimitExceeded)
    );
}

#[test]
fn transit_v2_wire_count_text_and_checked_size_limits() {
    let mut dto = literal_ready_dto();
    dto.route.segments.push(RouteSegmentImageV1 {
        edge_id: 1,
        from: 1,
        to: 2,
        length_mm: 2,
        start_offset_ticks: 0,
        end_offset_ticks: 2,
    });
    dto.route.graph_canonical_bytes.push(0x5a);
    let encoded = dto.encode_bytes_v2(WIRE_LIMITS).unwrap();
    let exact = TransitContextCheckpointLimitsV1::new(1, 1, 4, 8192);
    assert_eq!(
        TransitContextCheckpointV2::decode_bytes_v2(&encoded, exact).unwrap(),
        dto
    );
    for limits in [
        TransitContextCheckpointLimitsV1::new(0, 1, 4, 8192),
        TransitContextCheckpointLimitsV1::new(1, 0, 4, 8192),
        TransitContextCheckpointLimitsV1::new(1, 1, 3, 8192),
    ] {
        assert!(matches!(
            TransitContextCheckpointV2::decode_bytes_v2(&encoded, limits),
            Err(TransitContextWireError::LimitExceeded)
        ));
    }
    let mut oversized_count = encoded.clone();
    oversized_count[36..44].copy_from_slice(&u64::MAX.to_le_bytes());
    let huge = TransitContextCheckpointLimitsV1::new(usize::MAX, 4096, 64, usize::MAX);
    assert_eq!(
        TransitContextCheckpointV2::decode_bytes_v2(&oversized_count, huge),
        Err(TransitContextWireError::LimitExceeded)
    );
    assert_eq!(
        TransitContextCheckpointV2::decode_bytes_v2(&encoded, WIRE_LIMITS).unwrap(),
        dto
    );
}

#[test]
fn transit_v2_wire_malformed_tags_truncation_and_trailing() {
    let encoded = literal_ready_dto().encode_bytes_v2(WIRE_LIMITS).unwrap();
    for end in 0..encoded.len() {
        assert!(
            TransitContextCheckpointV2::decode_bytes_v2(&encoded[..end], WIRE_LIMITS).is_err(),
            "accepted truncation at {end}"
        );
    }
    let mut trailing = encoded.clone();
    trailing.push(0);
    assert_eq!(
        TransitContextCheckpointV2::decode_bytes_v2(&trailing, WIRE_LIMITS),
        Err(TransitContextWireError::TrailingBytes)
    );
    let mut magic = encoded.clone();
    magic[0] ^= 1;
    assert_eq!(
        TransitContextCheckpointV2::decode_bytes_v2(&magic, WIRE_LIMITS),
        Err(TransitContextWireError::InvalidData)
    );
    let mut wire_schema = encoded.clone();
    wire_schema[8..12].copy_from_slice(&2u32.to_le_bytes());
    assert_eq!(
        TransitContextCheckpointV2::decode_bytes_v2(&wire_schema, WIRE_LIMITS),
        Err(TransitContextWireError::UnsupportedVersion(2))
    );
    let mut native_schema = encoded.clone();
    native_schema[12..16].copy_from_slice(&3u32.to_le_bytes());
    assert_eq!(
        TransitContextCheckpointV2::decode_bytes_v2(&native_schema, WIRE_LIMITS),
        Err(TransitContextWireError::UnsupportedVersion(3))
    );
    let mut mode = encoded.clone();
    mode[84] = 0xff;
    assert_eq!(
        TransitContextCheckpointV2::decode_bytes_v2(&mode, WIRE_LIMITS),
        Err(TransitContextWireError::InvalidUtf8)
    );
    let mut action = encoded.clone();
    action[164] = 9;
    assert_eq!(
        TransitContextCheckpointV2::decode_bytes_v2(&action, WIRE_LIMITS),
        Err(TransitContextWireError::InvalidData)
    );
    let mut phase = encoded.clone();
    phase[249] = 9;
    assert_eq!(
        TransitContextCheckpointV2::decode_bytes_v2(&phase, WIRE_LIMITS),
        Err(TransitContextWireError::InvalidData)
    );
    let mut option = encoded.clone();
    option[250] = 2;
    assert_eq!(
        TransitContextCheckpointV2::decode_bytes_v2(&option, WIRE_LIMITS),
        Err(TransitContextWireError::InvalidData)
    );
    let mut bad_last = encoded;
    bad_last[272] = 2;
    assert_eq!(
        TransitContextCheckpointV2::decode_bytes_v2(&bad_last, WIRE_LIMITS),
        Err(TransitContextWireError::InvalidData)
    );
    let mut bad_timed = literal_ready_dto().encode_bytes_v2(WIRE_LIMITS).unwrap();
    bad_timed[227] = 2;
    assert_eq!(
        TransitContextCheckpointV2::decode_bytes_v2(&bad_timed, WIRE_LIMITS),
        Err(TransitContextWireError::InvalidData)
    );
    let mut bad_strategy = literal_ready_dto().encode_bytes_v2(WIRE_LIMITS).unwrap();
    bad_strategy[229] = 9;
    assert_eq!(
        TransitContextCheckpointV2::decode_bytes_v2(&bad_strategy, WIRE_LIMITS),
        Err(TransitContextWireError::InvalidData)
    );
    let base = literal_ready_dto().encode_bytes_v2(WIRE_LIMITS).unwrap();
    let mut bad_purpose = base[..270].to_vec();
    bad_purpose.push(1);
    bad_purpose.extend_from_slice(&0u64.to_le_bytes());
    bad_purpose.extend_from_slice(&0u64.to_le_bytes());
    bad_purpose.push(9);
    bad_purpose.extend_from_slice(&[0, 0]);
    assert_eq!(
        TransitContextCheckpointV2::decode_bytes_v2(&bad_purpose, WIRE_LIMITS),
        Err(TransitContextWireError::InvalidData)
    );

    let mut arrived = fixture(&[1]);
    let _ = arrive(&mut arrived.flow, arrived.carrier);
    let arrived_dto = current_image(&arrived.flow, arrived.carrier);
    let arrived_bytes = arrived_dto.encode_bytes_v2(WIRE_LIMITS).unwrap();
    let route_len = 16
        + 28
        + arrived_dto.route.segments.len() * 64
        + 32
        + 8
        + arrived_dto.route.movement_mode.len()
        + 28
        + arrived_dto.route.graph_canonical_bytes.len();
    assert_eq!(
        arrived_dto.arrival_witness.unwrap().cause,
        TransitArrivalCauseCheckpointV2::Domain
    );
    let witness_cause = route_len + 48 + 5 + 1 + 12 + 16;
    assert_eq!(arrived_bytes[witness_cause], 0);
    let mut invalid_cause = arrived_bytes;
    invalid_cause[witness_cause] = 9;
    assert_eq!(
        TransitContextCheckpointV2::decode_bytes_v2(&invalid_cause, WIRE_LIMITS),
        Err(TransitContextWireError::InvalidData)
    );
}

#[test]
fn transit_v2_wire_owner_reference_ticket_rejection() {
    let mut f = fixture(&[2]);
    start(&mut f.flow, f.carrier);
    let (image, codecs, bytes) = wire_capture(&f.flow, f.graph.clone());
    let (restored, rebind) = restore_wire(image, &codecs).unwrap();
    let carrier = restored.actor_domain_context(f.carrier_actor).unwrap();
    assert_eq!(restored.work(f.service).unwrap().owner, f.owner);
    assert_eq!(
        current_image(&restored, carrier).phase,
        TransitPhase::Moving
    );
    let dto = TransitContextCheckpointV2::decode_bytes_v2(&bytes, WIRE_LIMITS).unwrap();
    let native_limits = TransitContextCheckpointLimitsV1::new(16, 4096, 64, 8192 + 512);
    assert!(matches!(
        dto.restore_for_owner(&f.graph, &rebind, f.owner, native_limits),
        Err(TransitContextCheckpointError::InvalidReference)
    ));
    let valid = dto
        .restore_for_owner(&f.graph, &rebind, carrier.entity_id(), native_limits)
        .unwrap();
    assert_eq!(valid.phase(), TransitPhase::Moving);
    let mut bad_service = dto.clone();
    bad_service.service_work = EntityId::new(u64::MAX, 0);
    assert!(matches!(
        bad_service.restore_for_owner(&f.graph, &rebind, carrier.entity_id(), native_limits),
        Err(TransitContextCheckpointError::InvalidReference)
    ));
    let mut bad_ticket = dto;
    bad_ticket.next_progress_ticket = Some((u64::MAX, usize::MAX));
    assert!(matches!(
        bad_ticket.restore_for_owner(&f.graph, &rebind, carrier.entity_id(), native_limits),
        Err(TransitContextCheckpointError::InvalidTicket)
    ));

    let mut historical = fixture(&[1]);
    historical.flow.despawn_actor(historical.owner).unwrap();
    step_ok(&mut historical.flow);
    let (historical_image, historical_codecs, _) =
        wire_capture(&historical.flow, historical.graph.clone());
    assert!(restore_wire(historical_image, &historical_codecs).is_err());

    let mut legacy_flow = FlowRuntime::new();
    register_transit_context(&mut legacy_flow, "native-v2", KIND).unwrap();
    let legacy_owner = legacy_flow.spawn_actor().unwrap();
    let legacy_carrier_actor = legacy_flow.spawn_actor().unwrap();
    let resource = legacy_flow.create_resource(1).unwrap();
    let service = legacy_flow
        .create_work(legacy_owner, SimDuration::from_ticks(4), "service", ())
        .unwrap();
    let (graph, route) = graph_route(&[2]);
    let context = TransitContext::new(
        &legacy_flow,
        route,
        FlowAcquireCommand {
            resource,
            owner: legacy_owner,
            work: Some(service),
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
    let _carrier = legacy_flow
        .create_actor_domain_context(legacy_carrier_actor, "native-v2", KIND, context)
        .unwrap();
    let request = submit_arrived_service(
        &mut legacy_flow,
        resource,
        legacy_owner,
        service,
        SimTime::ZERO,
    );
    for _ in 0..4 {
        if legacy_flow.request(request).unwrap().state == RequestState::Completed {
            break;
        }
        step_ok(&mut legacy_flow);
    }
    assert_eq!(
        legacy_flow.request(request).unwrap().state,
        RequestState::Completed
    );
    legacy_flow.despawn_actor(legacy_owner).unwrap();
    for _ in 0..8 {
        if legacy_flow.work(service).is_err() {
            break;
        }
        step_ok(&mut legacy_flow);
    }
    let (legacy_image, legacy_codecs, _) = wire_capture(&legacy_flow, Arc::new(graph));
    let legacy_restored = restore_wire(legacy_image, &legacy_codecs).unwrap().0;
    let legacy_carrier = legacy_restored
        .actor_domain_context(legacy_carrier_actor)
        .unwrap();
    assert_eq!(
        current_image(&legacy_restored, legacy_carrier).phase,
        TransitPhase::Ready
    );
}

#[test]
fn transit_v2_wire_codec_schema_and_remaining_budget() {
    let mut f = fixture(&[1, 1]);
    start(&mut f.flow, f.carrier);
    let (image, codecs, bytes) = wire_capture(&f.flow, f.graph.clone());
    assert_eq!(&bytes[..8], b"KTCXCPV2");
    assert_eq!(u32::from_le_bytes(bytes[8..12].try_into().unwrap()), 1);
    assert_eq!(u32::from_le_bytes(bytes[12..16].try_into().unwrap()), 2);
    let store = image
        .context_stores
        .iter()
        .find(|store| store.codec_key == WIRE_CODEC)
        .unwrap();
    assert_eq!(store.rows.len(), 1);
    assert_eq!(store.rows[0].1, bytes);
    let mut schema_one_fallback = image;
    schema_one_fallback
        .context_stores
        .iter_mut()
        .find(|store| store.codec_key == WIRE_CODEC)
        .unwrap()
        .version = 1;
    assert!(restore_wire(schema_one_fallback, &codecs).is_err());
    let exact_outer = FlowCheckpointLimits {
        max_payload_bytes: bytes.len(),
        ..FlowCheckpointLimits::default()
    };
    let exact_codecs = wire_codecs_with_limits(&f.flow, f.graph.clone(), WIRE_LIMITS);
    let exact_image = f
        .flow
        .capture_checkpoint(&exact_codecs, exact_outer)
        .unwrap();
    let (restored, rebind) =
        FlowRuntime::restore_checkpoint_with_rebind(exact_image, &exact_codecs, exact_outer)
            .unwrap();
    assert!(rebind.resolve_work(f.carrier.entity_id()).is_ok());
    assert_eq!(restored.now(), f.flow.now());
    let too_small = FlowCheckpointLimits {
        max_payload_bytes: bytes.len() - 1,
        ..FlowCheckpointLimits::default()
    };
    assert!(f.flow.capture_checkpoint(&exact_codecs, too_small).is_err());
    let mut bad = bytes;
    bad[12..16].copy_from_slice(&1u32.to_le_bytes());
    assert_eq!(
        TransitContextCheckpointV2::decode_bytes_v2(&bad, WIRE_LIMITS),
        Err(TransitContextWireError::UnsupportedVersion(1))
    );
}
