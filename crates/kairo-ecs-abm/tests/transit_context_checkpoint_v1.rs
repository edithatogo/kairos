use kairo_ecs_abm::spatial::{
    EdgeId, MovementModeId, MovementProfile, NodeId, TransitEdge, TransitGraphV1,
};
use kairo_ecs_abm::{
    register_transit_context, register_transit_context_checkpoint_codec,
    register_transit_context_checkpoint_domain, schedule_transit_control, schedule_transit_start,
    TransitContext, TransitContextCheckpointError, TransitContextCheckpointLimitsV1,
    TransitContextCheckpointV1, TransitContextWireError, TransitPhase,
};
use kairo_ecs_des::{
    FlowAcquireCommand, FlowCallbackCodeV1, FlowCheckpointCodecError, FlowCheckpointCodecs,
    FlowCheckpointLimits, FlowDomainControl, FlowRuntime,
};
use kairo_ecs_types::{EntityId, EventKind, SimDuration, SimTime};
use std::sync::Arc;

const KIND: EventKind = EventKind::custom(0x7c21);
const LIMITS: TransitContextCheckpointLimitsV1 =
    TransitContextCheckpointLimitsV1::new(16, 4096, 64, 8192);

fn graph() -> TransitGraphV1 {
    let nodes = (1..=3).map(NodeId::new).collect::<Vec<_>>();
    let mode = MovementModeId::new("walk").unwrap();
    TransitGraphV1::new(
        1,
        nodes.clone(),
        vec![
            TransitEdge {
                id: EdgeId::new(1),
                from: nodes[0],
                to: nodes[1],
                length_mm: 2,
                allowed_modes: vec![mode.clone()],
            },
            TransitEdge {
                id: EdgeId::new(2),
                from: nodes[1],
                to: nodes[2],
                length_mm: 3,
                allowed_modes: vec![mode],
            },
        ],
    )
    .unwrap()
}

fn changed_graph() -> TransitGraphV1 {
    let nodes = (1..=3).map(NodeId::new).collect::<Vec<_>>();
    let mode = MovementModeId::new("walk").unwrap();
    TransitGraphV1::new(
        1,
        nodes.clone(),
        vec![
            TransitEdge {
                id: EdgeId::new(1),
                from: nodes[0],
                to: nodes[1],
                length_mm: 20,
                allowed_modes: vec![mode.clone()],
            },
            TransitEdge {
                id: EdgeId::new(2),
                from: nodes[1],
                to: nodes[2],
                length_mm: 3,
                allowed_modes: vec![mode],
            },
        ],
    )
    .unwrap()
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

fn codecs(
    source_identity: kairo_ecs_des::FlowRuntimeIdentity,
    trusted_graph: Arc<TransitGraphV1>,
) -> FlowCheckpointCodecs {
    let mut codecs = FlowCheckpointCodecs::new();
    register_transit_context_checkpoint_codec(
        &mut codecs,
        "transit-context-v1",
        source_identity,
        trusted_graph,
        LIMITS,
    )
    .unwrap();
    codecs
        .register_context::<()>("unit-v1", 1, encode_unit, decode_unit)
        .unwrap();
    register_transit_context_checkpoint_domain(
        &mut codecs,
        "transit",
        KIND,
        FlowCallbackCodeV1 {
            stable_id: "test.transit.planner".to_owned(),
            version: 1,
        },
        FlowCallbackCodeV1 {
            stable_id: "test.transit.receipt".to_owned(),
            version: 1,
        },
    )
    .unwrap();
    codecs
}

struct CapturedCheckpoint {
    image: kairo_ecs_des::FlowCheckpointV1,
    graph: Arc<TransitGraphV1>,
}

impl CapturedCheckpoint {
    fn context(&self) -> TransitContextCheckpointV1 {
        let store = self
            .image
            .context_stores
            .iter()
            .find(|store| store.codec_key == "transit-context-v1")
            .unwrap();
        TransitContextCheckpointV1::decode_bytes_v1(&store.rows[0].1, LIMITS).unwrap()
    }

    fn set_context(&mut self, context: &TransitContextCheckpointV1) {
        let store = self
            .image
            .context_stores
            .iter_mut()
            .find(|store| store.codec_key == "transit-context-v1")
            .unwrap();
        store.rows[0].1 = context.encode_bytes_v1(LIMITS).unwrap();
    }

    fn mutate_context_bytes(&mut self, mutate: impl FnOnce(&mut Vec<u8>)) {
        let store = self
            .image
            .context_stores
            .iter_mut()
            .find(|store| store.codec_key == "transit-context-v1")
            .unwrap();
        mutate(&mut store.rows[0].1);
    }

    fn restore(self) -> Result<FlowRuntime, kairo_ecs_des::FlowCheckpointError> {
        let codecs = codecs(FlowRuntime::new().identity(), self.graph);
        FlowRuntime::restore_checkpoint(self.image, &codecs, FlowCheckpointLimits::default())
    }
}

fn mutate_captured(
    mut captured: CapturedCheckpoint,
    mutate: impl FnOnce(&mut TransitContextCheckpointV1),
) -> CapturedCheckpoint {
    let mut context = captured.context();
    mutate(&mut context);
    captured.set_context(&context);
    captured
}

fn capture_image(source: &FlowRuntime, trusted_graph: TransitGraphV1) -> CapturedCheckpoint {
    let graph = Arc::new(trusted_graph);
    let codecs = codecs(source.identity(), graph.clone());
    let image = source
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    CapturedCheckpoint { image, graph }
}

#[test]
fn wire_v1_roundtrips_exactly_and_preflights_corruption_and_limits() {
    let (source, carrier, trusted_graph) = source_flow();
    let original_context = source.work_context::<TransitContext>(carrier).unwrap();
    let original = original_context
        .checkpoint_v1(&source.identity(), LIMITS)
        .unwrap();
    let captured = capture_image(&source, trusted_graph);
    let wire = captured.image.context_stores[0].rows[0].1.clone();
    assert_eq!(captured.context(), original);
    assert_eq!(
        original_context
            .checkpoint_bytes_v1(
                &source.identity(),
                TransitContextCheckpointLimitsV1::new(
                    LIMITS.max_segments,
                    LIMITS.max_graph_bytes,
                    LIMITS.max_mode_bytes,
                    wire.len(),
                )
            )
            .unwrap(),
        wire
    );
    let unrelated = FlowRuntime::new();
    assert_eq!(
        original_context.checkpoint_bytes_v1(&unrelated.identity(), LIMITS),
        Err(TransitContextWireError::Native(
            TransitContextCheckpointError::LineageMismatch
        ))
    );
    assert_eq!(
        original_context.checkpoint_bytes_v1(
            &source.identity(),
            TransitContextCheckpointLimitsV1::new(
                LIMITS.max_segments,
                LIMITS.max_graph_bytes,
                LIMITS.max_mode_bytes,
                wire.len() - 1,
            ),
        ),
        Err(TransitContextWireError::LimitExceeded)
    );

    let tight = TransitContextCheckpointLimitsV1::new(
        LIMITS.max_segments,
        LIMITS.max_graph_bytes,
        LIMITS.max_mode_bytes,
        wire.len(),
    );
    assert_eq!(
        TransitContextCheckpointV1::decode_bytes_v1(&wire, tight).unwrap(),
        original
    );
    assert_eq!(
        TransitContextCheckpointV1::decode_bytes_v1(
            &wire,
            TransitContextCheckpointLimitsV1::new(
                LIMITS.max_segments,
                LIMITS.max_graph_bytes,
                LIMITS.max_mode_bytes,
                wire.len() - 1,
            )
        ),
        Err(TransitContextWireError::LimitExceeded),
    );
    assert_eq!(
        TransitContextCheckpointV1::decode_bytes_v1(&wire[..wire.len() - 1], LIMITS),
        Err(TransitContextWireError::Truncated),
    );
    let mut trailing = wire.clone();
    trailing.push(0);
    assert_eq!(
        TransitContextCheckpointV1::decode_bytes_v1(&trailing, LIMITS),
        Err(TransitContextWireError::TrailingBytes)
    );

    let mut bad_schema = wire.clone();
    bad_schema[8..12].copy_from_slice(&2u32.to_le_bytes());
    assert_eq!(
        TransitContextCheckpointV1::decode_bytes_v1(&bad_schema, LIMITS),
        Err(TransitContextWireError::UnsupportedVersion(2))
    );
    let mut bad_route_schema = wire.clone();
    bad_route_schema[16..20].copy_from_slice(&2u32.to_le_bytes());
    assert_eq!(
        TransitContextCheckpointV1::decode_bytes_v1(&bad_route_schema, LIMITS),
        Err(TransitContextWireError::UnsupportedVersion(2))
    );
    let mut oversized_segments = wire.clone();
    oversized_segments[36..44].copy_from_slice(&u64::MAX.to_le_bytes());
    assert_eq!(
        TransitContextCheckpointV1::decode_bytes_v1(&oversized_segments, LIMITS),
        Err(TransitContextWireError::LimitExceeded)
    );

    let route_bytes = 16
        + 96
        + original.route.segments.len() * 64
        + original.route.movement_mode.len()
        + original.route.graph_canonical_bytes.len();
    let mut bad_option = wire.clone();
    bad_option[route_bytes + 60] = 2;
    assert_eq!(
        TransitContextCheckpointV1::decode_bytes_v1(&bad_option, LIMITS),
        Err(TransitContextWireError::InvalidData)
    );
    let mut bad_bool = wire.clone();
    bad_bool[route_bytes + 114] = 2;
    assert_eq!(
        TransitContextCheckpointV1::decode_bytes_v1(&bad_bool, LIMITS),
        Err(TransitContextWireError::InvalidData)
    );

    let mut bad_utf8 = wire.clone();
    let mode_offset = 16 + 68 + original.route.segments.len() * 64;
    bad_utf8[mode_offset] = 0xff;
    assert_eq!(
        TransitContextCheckpointV1::decode_bytes_v1(&bad_utf8, LIMITS),
        Err(TransitContextWireError::InvalidUtf8)
    );
    assert_eq!(
        TransitContextCheckpointV1::decode_bytes_v1(
            &wire,
            TransitContextCheckpointLimitsV1::new(0, 0, 0, LIMITS.max_total_bytes)
        ),
        Err(TransitContextWireError::LimitExceeded)
    );
    assert_eq!(
        TransitContextCheckpointV1::decode_bytes_v1(
            &wire,
            TransitContextCheckpointLimitsV1::new(
                LIMITS.max_segments,
                0,
                LIMITS.max_mode_bytes,
                LIMITS.max_total_bytes,
            )
        ),
        Err(TransitContextWireError::LimitExceeded)
    );
    assert_eq!(
        original_context
            .checkpoint_v1(&source.identity(), LIMITS)
            .unwrap(),
        original
    );
}

fn reject_mutated_image(mutate: impl FnOnce(&mut TransitContextCheckpointV1)) {
    let (source, _, trusted_graph) = source_flow();
    let mut captured = capture_image(&source, trusted_graph);
    let mut context = captured.context();
    mutate(&mut context);
    captured.set_context(&context);
    assert!(captured.restore().is_err());
}

fn source_flow() -> (FlowRuntime, kairo_ecs_des::WorkId, TransitGraphV1) {
    let mut flow = FlowRuntime::new();
    register_transit_context(&mut flow, "transit", KIND).unwrap();
    let actor = flow.spawn_actor().unwrap();
    let resource = flow.create_resource(1).unwrap();
    let service = flow
        .create_work(actor, SimDuration::from_ticks(3), "service", ())
        .unwrap();
    let graph = graph();
    let route = graph
        .route(
            NodeId::new(1),
            NodeId::new(3),
            &MovementProfile::new("walk", 1).unwrap(),
            1,
        )
        .unwrap();
    let acquire = FlowAcquireCommand {
        resource,
        owner: actor,
        work: Some(service),
        at: SimTime::ZERO,
        priority_level: 4,
        deadline: Some(SimTime::from_ticks(8)),
        scheduler_priority: 2,
        timed: true,
        can_preempt: true,
        preemptible: Some(kairo_ecs_des::PreemptionStrategy::Suspend),
    };
    let context = TransitContext::new(&flow, route, acquire, SimTime::ZERO).unwrap();
    let carrier = flow
        .create_actor_domain_context(actor, "transit", KIND, context)
        .unwrap();
    (flow, carrier, graph)
}

#[test]
fn paused_mid_edge_context_roundtrips_exact_cursor_and_runtime() {
    let (mut source, carrier, trusted_graph) = source_flow();
    schedule_transit_start(&mut source, carrier, KIND, SimTime::ZERO, 0).unwrap();
    source.step().unwrap().unwrap();
    schedule_transit_control(
        &mut source,
        carrier,
        KIND,
        FlowDomainControl::Pause,
        SimTime::from_ticks(1),
        0,
    )
    .unwrap();
    source.step().unwrap().unwrap();
    let original = source.work_context::<TransitContext>(carrier).unwrap();
    assert_eq!(original.phase(), TransitPhase::Paused);
    assert_eq!(
        original
            .progress_at(SimTime::from_ticks(1))
            .unwrap()
            .useful_elapsed,
        SimDuration::from_ticks(1)
    );
    let captured = capture_image(&source, trusted_graph);
    let context_image = captured.context();
    assert_eq!(
        context_image,
        original.checkpoint_v1(&source.identity(), LIMITS).unwrap()
    );
    assert_eq!(context_image.phase, TransitPhase::Paused);
    assert_eq!(context_image.paused_from, Some(TransitPhase::Moving));
    assert_eq!(context_image.segment_index, 0);
    assert_eq!(context_image.elapsed_in_segment_ticks, 1);
    assert_eq!(context_image.acquire_priority_level, 4);
    assert_eq!(
        context_image.acquire_preemptible,
        Some(kairo_ecs_des::PreemptionStrategy::Suspend)
    );

    let mut restored = captured.restore().unwrap();
    let rebound_carrier = restored
        .actor_domain_context(context_image.acquire_owner)
        .unwrap();
    let rebound = restored
        .work_context::<TransitContext>(rebound_carrier)
        .unwrap();
    assert_eq!(rebound.phase(), TransitPhase::Paused);
    assert_eq!(
        rebound.progress_at(SimTime::from_ticks(1)).unwrap(),
        original.progress_at(SimTime::from_ticks(1)).unwrap()
    );
    schedule_transit_control(
        &mut restored,
        rebound_carrier,
        KIND,
        FlowDomainControl::Pause,
        SimTime::from_ticks(1),
        0,
    )
    .unwrap();
}

#[test]
fn moving_context_roundtrips_scheduled_event_before_pause() {
    let (mut source, carrier, trusted_graph) = source_flow();
    schedule_transit_start(&mut source, carrier, KIND, SimTime::ZERO, 0).unwrap();
    source.step().unwrap().unwrap();
    let captured = capture_image(&source, trusted_graph);
    let context_image = captured.context();
    assert_eq!(
        context_image,
        source
            .work_context::<TransitContext>(carrier)
            .unwrap()
            .checkpoint_v1(&source.identity(), LIMITS)
            .unwrap()
    );
    assert_eq!(context_image.phase, TransitPhase::Moving);
    assert!(context_image.expected_event.is_some());
    assert!(context_image.expected_due_ticks.is_some());

    let mut restored = captured.restore().unwrap();
    let rebound_carrier = restored
        .actor_domain_context(context_image.acquire_owner)
        .unwrap();
    let rebound = restored
        .work_context::<TransitContext>(rebound_carrier)
        .unwrap();
    assert_eq!(rebound.phase(), TransitPhase::Moving);
    assert_eq!(
        rebound
            .progress_at(SimTime::from_ticks(1))
            .unwrap()
            .useful_elapsed,
        SimDuration::from_ticks(1)
    );
    schedule_transit_control(
        &mut restored,
        rebound_carrier,
        KIND,
        FlowDomainControl::Pause,
        SimTime::from_ticks(1),
        0,
    )
    .unwrap();
}

#[test]
fn capture_rejects_wrong_lineage_and_limits_without_changing_source() {
    let (source, carrier, _) = source_flow();
    let context = source.work_context::<TransitContext>(carrier).unwrap();
    let unrelated = FlowRuntime::new();
    assert_eq!(
        context.checkpoint_v1(&unrelated.identity(), LIMITS),
        Err(TransitContextCheckpointError::LineageMismatch)
    );
    let before = context.checkpoint_v1(&source.identity(), LIMITS).unwrap();
    let tiny = TransitContextCheckpointLimitsV1::new(0, 0, 0, 0);
    assert_eq!(
        context.checkpoint_v1(&source.identity(), tiny),
        Err(TransitContextCheckpointError::LimitExceeded)
    );
    assert_eq!(
        context.checkpoint_v1(&source.identity(), LIMITS).unwrap(),
        before
    );
}

#[test]
fn import_rejects_bad_cursor_and_unresolved_work_without_source_mutation() {
    let (source, carrier, trusted_graph) = source_flow();
    let source_context = source.work_context::<TransitContext>(carrier).unwrap();
    let before = source_context
        .checkpoint_v1(&source.identity(), LIMITS)
        .unwrap();

    let image = mutate_captured(capture_image(&source, trusted_graph.clone()), |context| {
        context.segment_index = usize::MAX;
    });
    assert!(image.restore().is_err());
    assert_eq!(
        source_context
            .checkpoint_v1(&source.identity(), LIMITS)
            .unwrap(),
        before
    );

    let image = mutate_captured(capture_image(&source, trusted_graph), |context_image| {
        let wrong = EntityId::new(
            context_image.service_work.index,
            context_image.service_work.generation.wrapping_add(1),
        );
        context_image.service_work = wrong;
    });
    assert!(image.restore().is_err());
    assert_eq!(
        source_context
            .checkpoint_v1(&source.identity(), LIMITS)
            .unwrap(),
        before
    );
}

#[test]
fn import_rejects_schema_and_unapproved_geometry() {
    let (source, _, trusted_graph) = source_flow();

    let mut image = capture_image(&source, trusted_graph.clone());
    image.mutate_context_bytes(|bytes| bytes[8..12].copy_from_slice(&2u32.to_le_bytes()));
    assert!(image.restore().is_err());

    let mut image = capture_image(&source, trusted_graph);
    image.graph = Arc::new(changed_graph());
    assert!(image.restore().is_err());
}

#[test]
fn import_binds_service_owner_carrier_row_and_domain_kind() {
    let (mut source, _, trusted_graph) = source_flow();
    let other_actor = source.spawn_actor().unwrap();
    let image = mutate_captured(capture_image(&source, trusted_graph.clone()), |context| {
        context.acquire_owner = other_actor;
    });
    assert!(image.restore().is_err());

    let image = mutate_captured(
        capture_image(&source, trusted_graph.clone()),
        |checkpoint| {
            checkpoint.carrier = Some(checkpoint.service_work);
            checkpoint.kind = Some(KIND);
        },
    );
    assert!(image.restore().is_err());

    let (mut moving, carrier, trusted_graph) = source_flow();
    schedule_transit_start(&mut moving, carrier, KIND, SimTime::ZERO, 0).unwrap();
    moving.step().unwrap().unwrap();
    let image = mutate_captured(capture_image(&moving, trusted_graph), |context| {
        context.kind = Some(EventKind::custom(0x7c22));
    });
    assert!(image.restore().is_err());
}

#[test]
fn import_rejects_transit_state_field_contradictions() {
    reject_mutated_image(|image| image.acquire_timed = false);
    reject_mutated_image(|image| image.acquire_at_ticks = 1);
    reject_mutated_image(|image| image.carrier = Some(image.service_work));
    reject_mutated_image(|image| image.kind = Some(KIND));
    reject_mutated_image(|image| image.paused_from = Some(TransitPhase::Ready));
    reject_mutated_image(|image| image.last_advanced_at_ticks = 1);
    reject_mutated_image(|image| image.segment_index = 1);

    let (mut moving, carrier, trusted_graph) = source_flow();
    schedule_transit_start(&mut moving, carrier, KIND, SimTime::ZERO, 0).unwrap();
    moving.step().unwrap().unwrap();
    let image = mutate_captured(capture_image(&moving, trusted_graph.clone()), |context| {
        context.initial_start_pending = true;
    });
    assert!(image.restore().is_err());

    let image = mutate_captured(capture_image(&moving, trusted_graph.clone()), |image| {
        image.start_at_ticks = 1;
        image.acquire_at_ticks = 1;
        image.last_advanced_at_ticks = 0;
    });
    assert!(image.restore().is_err());

    let image = mutate_captured(capture_image(&moving, trusted_graph.clone()), |image| {
        image.segment_index = image.route.segments.len();
        image.elapsed_in_segment_ticks = 0;
    });
    assert!(image.restore().is_err());

    let image = mutate_captured(capture_image(&moving, trusted_graph), |context| {
        context.carrier = None;
    });
    assert!(image.restore().is_err());

    let (mut paused_ready, carrier, trusted_graph) = source_flow();
    schedule_transit_control(
        &mut paused_ready,
        carrier,
        KIND,
        FlowDomainControl::Pause,
        SimTime::ZERO,
        0,
    )
    .unwrap();
    paused_ready.step().unwrap().unwrap();
    let image = capture_image(&paused_ready, trusted_graph.clone());
    assert_eq!(image.context().paused_from, Some(TransitPhase::Ready));
    let image = mutate_captured(image, |context| context.last_advanced_at_ticks = 1);
    assert!(image.restore().is_err());

    let image = mutate_captured(capture_image(&paused_ready, trusted_graph), |context| {
        context.segment_index = 1;
    });
    assert!(image.restore().is_err());

    let (mut arrived, carrier, trusted_graph) = source_flow();
    schedule_transit_start(&mut arrived, carrier, KIND, SimTime::ZERO, 0).unwrap();
    arrived.step().unwrap().unwrap();
    arrived.step().unwrap().unwrap();
    arrived.step().unwrap().unwrap();
    let image = mutate_captured(capture_image(&arrived, trusted_graph), |context| {
        context.segment_index = 0;
    });
    assert!(image.restore().is_err());
}

#[test]
fn ready_zero_route_and_paused_at_end_are_valid() {
    let mut source = FlowRuntime::new();
    register_transit_context(&mut source, "transit", KIND).unwrap();
    let actor = source.spawn_actor().unwrap();
    let resource = source.create_resource(1).unwrap();
    let service = source
        .create_work(actor, SimDuration::from_ticks(3), "service", ())
        .unwrap();
    let trusted_graph = graph();
    let zero_route = trusted_graph
        .route(
            NodeId::new(1),
            NodeId::new(1),
            &MovementProfile::new("walk", 1).unwrap(),
            1,
        )
        .unwrap();
    let acquire = FlowAcquireCommand {
        resource,
        owner: actor,
        work: Some(service),
        at: SimTime::ZERO,
        priority_level: 0,
        deadline: None,
        scheduler_priority: 0,
        timed: true,
        can_preempt: false,
        preemptible: None,
    };
    let context = TransitContext::new(&source, zero_route, acquire, SimTime::ZERO).unwrap();
    let carrier = source
        .create_actor_domain_context(actor, "transit", KIND, context)
        .unwrap();
    let image = capture_image(&source, trusted_graph.clone());
    let checkpoint = image.context();
    assert_eq!(checkpoint.segment_index, 0);
    assert_eq!(checkpoint.carrier, None);
    assert_eq!(checkpoint.kind, None);
    let restored = image.restore().unwrap();
    assert_eq!(
        restored
            .work_context::<TransitContext>(carrier)
            .unwrap()
            .phase(),
        TransitPhase::Ready
    );
    let rebound = restored.work_context::<TransitContext>(carrier).unwrap();
    let rebound_checkpoint = rebound.checkpoint_v1(&restored.identity(), LIMITS).unwrap();
    assert_eq!(rebound_checkpoint.carrier, None);
    assert_eq!(rebound_checkpoint.kind, None);

    let (mut source, carrier, trusted_graph) = source_flow();
    schedule_transit_start(&mut source, carrier, KIND, SimTime::ZERO, 0).unwrap();
    source.step().unwrap().unwrap();
    schedule_transit_control(
        &mut source,
        carrier,
        KIND,
        FlowDomainControl::Pause,
        SimTime::from_ticks(5),
        0,
    )
    .unwrap();
    for _ in 0..8 {
        source.step().unwrap();
        if source
            .work_context::<TransitContext>(carrier)
            .unwrap()
            .phase()
            == TransitPhase::Paused
        {
            break;
        }
    }
    let image = capture_image(&source, trusted_graph);
    let checkpoint = image.context();
    assert_eq!(checkpoint.phase, TransitPhase::Paused);
    assert_eq!(checkpoint.paused_from, Some(TransitPhase::Moving));
    assert_eq!(checkpoint.segment_index, checkpoint.route.segments.len());
    let restored = image.restore().unwrap();
    assert_eq!(
        restored
            .work_context::<TransitContext>(carrier)
            .unwrap()
            .phase(),
        TransitPhase::Paused
    );
}

#[test]
fn arrived_context_roundtrips_completed_cursor_and_arrival_ticket() {
    let (mut source, carrier, trusted_graph) = source_flow();
    schedule_transit_start(&mut source, carrier, KIND, SimTime::ZERO, 0).unwrap();
    source.step().unwrap().unwrap();
    source.step().unwrap().unwrap();
    source.step().unwrap().unwrap();
    let original = source.work_context::<TransitContext>(carrier).unwrap();
    assert_eq!(original.phase(), TransitPhase::Arrived);

    let image = capture_image(&source, trusted_graph);
    let context_image = image.context();
    assert_eq!(
        context_image,
        original.checkpoint_v1(&source.identity(), LIMITS).unwrap()
    );
    assert_eq!(context_image.phase, TransitPhase::Arrived);
    assert!(context_image.arrival_ticket.is_some());
    assert_eq!(
        context_image.segment_index,
        context_image.route.segments.len()
    );
    assert_eq!(context_image.elapsed_in_segment_ticks, 0);

    let restored = image.restore().unwrap();
    let rebound_carrier = restored
        .actor_domain_context(context_image.acquire_owner)
        .unwrap();
    let rebound = restored
        .work_context::<TransitContext>(rebound_carrier)
        .unwrap();
    assert_eq!(rebound.phase(), TransitPhase::Arrived);
    assert_eq!(rebound.arrival_ticket(), original.arrival_ticket());
    assert_eq!(
        rebound
            .progress_at(SimTime::from_ticks(5))
            .unwrap()
            .remaining,
        SimDuration::ZERO
    );
}

#[test]
fn arrived_context_rebinds_a_retained_already_dispatched_event_id() {
    let (mut source, carrier, trusted_graph) = source_flow();
    schedule_transit_start(&mut source, carrier, KIND, SimTime::ZERO, 0).unwrap();
    source.step().unwrap().unwrap();
    source.step().unwrap().unwrap();
    schedule_transit_control(
        &mut source,
        carrier,
        KIND,
        FlowDomainControl::Pause,
        SimTime::from_ticks(5),
        -1,
    )
    .unwrap();
    source.step().unwrap().unwrap();
    schedule_transit_control(
        &mut source,
        carrier,
        KIND,
        FlowDomainControl::Resume,
        SimTime::from_ticks(5),
        -2,
    )
    .unwrap();
    source.step().unwrap().unwrap();
    source.step().unwrap().unwrap();
    let original = source.work_context::<TransitContext>(carrier).unwrap();
    assert_eq!(original.phase(), TransitPhase::Arrived);

    let image = capture_image(&source, trusted_graph);
    let context_image = image.context();
    let stale = context_image.expected_event.unwrap();
    assert_eq!(context_image.expected_due_ticks, None);

    let restored = image.restore().unwrap();
    let rebound_carrier = restored
        .actor_domain_context(context_image.acquire_owner)
        .unwrap();
    let rebound = restored
        .work_context::<TransitContext>(rebound_carrier)
        .unwrap();
    assert_eq!(rebound.phase(), TransitPhase::Arrived);
    let rebound_image = rebound.checkpoint_v1(&restored.identity(), LIMITS).unwrap();
    assert_eq!(rebound_image.expected_event, Some(stale));
    assert_eq!(rebound_image.expected_due_ticks, None);
}
