use kairo_ecs_abm::spatial::{
    EdgeId, MovementModeId, MovementProfile, NodeId, TransitEdge, TransitGraphV1,
};
use kairo_ecs_abm::{
    register_transit_context, register_transit_context_checkpoint_domain, schedule_transit_control,
    schedule_transit_start, TransitContext, TransitContextCheckpointError,
    TransitContextCheckpointLimitsV1, TransitContextCheckpointV1, TransitPhase,
};
use kairo_ecs_des::{
    FlowAcquireCommand, FlowCallbackCodeV1, FlowCheckpointCodecError, FlowCheckpointCodecs,
    FlowCheckpointLimits, FlowDomainControl, FlowRuntime,
};
use kairo_ecs_types::{EntityId, EventKind, SimDuration, SimTime};
use std::cell::RefCell;

const KIND: EventKind = EventKind::custom(0x7c21);
const LIMITS: TransitContextCheckpointLimitsV1 =
    TransitContextCheckpointLimitsV1::new(16, 4096, 64, 8192);

thread_local! {
    static SOURCE_IDENTITY: RefCell<Option<kairo_ecs_des::FlowRuntimeIdentity>> = const { RefCell::new(None) };
    static ROUTE_GRAPH: RefCell<Option<TransitGraphV1>> = const { RefCell::new(None) };
    static CONTEXT_IMAGE: RefCell<Option<TransitContextCheckpointV1>> = const { RefCell::new(None) };
    static OTHER_ACTOR: RefCell<Option<EntityId>> = const { RefCell::new(None) };
}

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

fn encode(
    context: &TransitContext,
    _remaining: usize,
) -> Result<Vec<u8>, FlowCheckpointCodecError> {
    let identity = SOURCE_IDENTITY.with(|value| value.borrow().clone().unwrap());
    let image = context
        .checkpoint_v1(&identity, LIMITS)
        .map_err(|error| FlowCheckpointCodecError(error.to_string()))?;
    CONTEXT_IMAGE.with(|value| *value.borrow_mut() = Some(image));
    Ok(vec![1])
}

fn decode(
    bytes: &[u8],
    row_owner: EntityId,
    rebind: &kairo_ecs_des::FlowCheckpointRebindV1,
) -> Result<TransitContext, FlowCheckpointCodecError> {
    if bytes != [1] {
        return Err(FlowCheckpointCodecError(
            "invalid test context marker".to_owned(),
        ));
    }
    let image = CONTEXT_IMAGE.with(|value| value.borrow_mut().take().unwrap());
    let trusted_graph = ROUTE_GRAPH.with(|value| value.borrow().clone().unwrap());
    image
        .restore_for_owner(&trusted_graph, rebind, row_owner, LIMITS)
        .map_err(|error| FlowCheckpointCodecError(error.to_string()))
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

fn codecs() -> FlowCheckpointCodecs {
    let mut codecs = FlowCheckpointCodecs::new();
    codecs
        .register_context_with_owner::<TransitContext>("transit-context-v1", 1, encode, decode)
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

fn capture_image(
    source: &FlowRuntime,
    trusted_graph: TransitGraphV1,
) -> kairo_ecs_des::FlowCheckpointV1 {
    SOURCE_IDENTITY.with(|value| *value.borrow_mut() = Some(source.identity()));
    ROUTE_GRAPH.with(|value| *value.borrow_mut() = Some(trusted_graph));
    source
        .capture_checkpoint(&codecs(), FlowCheckpointLimits::default())
        .unwrap()
}

fn reject_mutated_image(mutate: impl FnOnce(&mut TransitContextCheckpointV1)) {
    let (source, _, trusted_graph) = source_flow();
    let image = capture_image(&source, trusted_graph);
    CONTEXT_IMAGE.with(|value| mutate(value.borrow_mut().as_mut().unwrap()));
    assert!(
        FlowRuntime::restore_checkpoint(image, &codecs(), FlowCheckpointLimits::default()).is_err()
    );
}

fn source_flow() -> (FlowRuntime, kairo_ecs_des::WorkId, TransitGraphV1) {
    let mut flow = FlowRuntime::new();
    register_transit_context(&mut flow, "transit", KIND).unwrap();
    let actor = flow.spawn_actor().unwrap();
    let other_actor = flow.spawn_actor().unwrap();
    OTHER_ACTOR.with(|value| *value.borrow_mut() = Some(other_actor));
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
    SOURCE_IDENTITY.with(|value| *value.borrow_mut() = Some(source.identity()));
    ROUTE_GRAPH.with(|value| *value.borrow_mut() = Some(trusted_graph));

    let image = source
        .capture_checkpoint(&codecs(), FlowCheckpointLimits::default())
        .unwrap();
    let context_image = CONTEXT_IMAGE.with(|value| value.borrow().clone().unwrap());
    assert_eq!(context_image.phase, TransitPhase::Paused);
    assert_eq!(context_image.paused_from, Some(TransitPhase::Moving));
    assert_eq!(context_image.segment_index, 0);
    assert_eq!(context_image.elapsed_in_segment_ticks, 1);
    assert_eq!(context_image.acquire_priority_level, 4);
    assert_eq!(
        context_image.acquire_preemptible,
        Some(kairo_ecs_des::PreemptionStrategy::Suspend)
    );

    let mut restored =
        FlowRuntime::restore_checkpoint(image, &codecs(), FlowCheckpointLimits::default()).unwrap();
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
    let image = capture_image(&source, trusted_graph);
    let context_image = CONTEXT_IMAGE.with(|value| value.borrow().clone().unwrap());
    assert_eq!(context_image.phase, TransitPhase::Moving);
    assert!(context_image.expected_event.is_some());
    assert!(context_image.expected_due_ticks.is_some());

    let mut restored =
        FlowRuntime::restore_checkpoint(image, &codecs(), FlowCheckpointLimits::default()).unwrap();
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

    let image = capture_image(&source, trusted_graph.clone());
    CONTEXT_IMAGE.with(|value| {
        value.borrow_mut().as_mut().unwrap().segment_index = usize::MAX;
    });
    assert!(
        FlowRuntime::restore_checkpoint(image, &codecs(), FlowCheckpointLimits::default(),)
            .is_err()
    );
    assert_eq!(
        source_context
            .checkpoint_v1(&source.identity(), LIMITS)
            .unwrap(),
        before
    );

    let image = capture_image(&source, trusted_graph);
    CONTEXT_IMAGE.with(|value| {
        let mut stored = value.borrow_mut();
        let context_image = stored.as_mut().unwrap();
        let wrong = EntityId::new(
            context_image.service_work.index,
            context_image.service_work.generation.wrapping_add(1),
        );
        context_image.service_work = wrong;
    });
    assert!(
        FlowRuntime::restore_checkpoint(image, &codecs(), FlowCheckpointLimits::default(),)
            .is_err()
    );
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

    let image = capture_image(&source, trusted_graph.clone());
    CONTEXT_IMAGE.with(|value| {
        value.borrow_mut().as_mut().unwrap().schema_version = 2;
    });
    assert!(
        FlowRuntime::restore_checkpoint(image, &codecs(), FlowCheckpointLimits::default(),)
            .is_err()
    );

    let image = capture_image(&source, trusted_graph);
    ROUTE_GRAPH.with(|value| *value.borrow_mut() = Some(changed_graph()));
    assert!(
        FlowRuntime::restore_checkpoint(image, &codecs(), FlowCheckpointLimits::default(),)
            .is_err()
    );
}

#[test]
fn import_binds_service_owner_carrier_row_and_domain_kind() {
    let (source, _, trusted_graph) = source_flow();
    let image = capture_image(&source, trusted_graph.clone());
    CONTEXT_IMAGE.with(|value| {
        value.borrow_mut().as_mut().unwrap().acquire_owner =
            OTHER_ACTOR.with(|actor| actor.borrow().unwrap());
    });
    assert!(
        FlowRuntime::restore_checkpoint(image, &codecs(), FlowCheckpointLimits::default()).is_err()
    );

    let image = capture_image(&source, trusted_graph.clone());
    CONTEXT_IMAGE.with(|value| {
        let mut stored = value.borrow_mut();
        let checkpoint = stored.as_mut().unwrap();
        checkpoint.carrier = Some(checkpoint.service_work);
        checkpoint.kind = Some(KIND);
    });
    assert!(
        FlowRuntime::restore_checkpoint(image, &codecs(), FlowCheckpointLimits::default()).is_err()
    );

    let (mut moving, carrier, trusted_graph) = source_flow();
    schedule_transit_start(&mut moving, carrier, KIND, SimTime::ZERO, 0).unwrap();
    moving.step().unwrap().unwrap();
    let image = capture_image(&moving, trusted_graph);
    CONTEXT_IMAGE.with(|value| {
        value.borrow_mut().as_mut().unwrap().kind = Some(EventKind::custom(0x7c22));
    });
    assert!(
        FlowRuntime::restore_checkpoint(image, &codecs(), FlowCheckpointLimits::default()).is_err()
    );
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
    let image = capture_image(&moving, trusted_graph.clone());
    CONTEXT_IMAGE.with(|value| {
        value.borrow_mut().as_mut().unwrap().initial_start_pending = true;
    });
    assert!(
        FlowRuntime::restore_checkpoint(image, &codecs(), FlowCheckpointLimits::default()).is_err()
    );

    let image = capture_image(&moving, trusted_graph.clone());
    CONTEXT_IMAGE.with(|value| {
        let mut stored = value.borrow_mut();
        let image = stored.as_mut().unwrap();
        image.start_at_ticks = 1;
        image.acquire_at_ticks = 1;
        image.last_advanced_at_ticks = 0;
    });
    assert!(
        FlowRuntime::restore_checkpoint(image, &codecs(), FlowCheckpointLimits::default()).is_err()
    );

    let image = capture_image(&moving, trusted_graph.clone());
    CONTEXT_IMAGE.with(|value| {
        let mut stored = value.borrow_mut();
        let image = stored.as_mut().unwrap();
        image.segment_index = image.route.segments.len();
        image.elapsed_in_segment_ticks = 0;
    });
    assert!(
        FlowRuntime::restore_checkpoint(image, &codecs(), FlowCheckpointLimits::default()).is_err()
    );

    let image = capture_image(&moving, trusted_graph);
    CONTEXT_IMAGE.with(|value| {
        value.borrow_mut().as_mut().unwrap().carrier = None;
    });
    assert!(
        FlowRuntime::restore_checkpoint(image, &codecs(), FlowCheckpointLimits::default()).is_err()
    );

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
    assert_eq!(
        CONTEXT_IMAGE.with(|value| value.borrow().as_ref().unwrap().paused_from),
        Some(TransitPhase::Ready)
    );
    CONTEXT_IMAGE.with(|value| {
        value.borrow_mut().as_mut().unwrap().last_advanced_at_ticks = 1;
    });
    assert!(
        FlowRuntime::restore_checkpoint(image, &codecs(), FlowCheckpointLimits::default()).is_err()
    );

    let image = capture_image(&paused_ready, trusted_graph);
    CONTEXT_IMAGE.with(|value| {
        value.borrow_mut().as_mut().unwrap().segment_index = 1;
    });
    assert!(
        FlowRuntime::restore_checkpoint(image, &codecs(), FlowCheckpointLimits::default()).is_err()
    );

    let (mut arrived, carrier, trusted_graph) = source_flow();
    schedule_transit_start(&mut arrived, carrier, KIND, SimTime::ZERO, 0).unwrap();
    arrived.step().unwrap().unwrap();
    arrived.step().unwrap().unwrap();
    arrived.step().unwrap().unwrap();
    let image = capture_image(&arrived, trusted_graph);
    CONTEXT_IMAGE.with(|value| {
        value.borrow_mut().as_mut().unwrap().segment_index = 0;
    });
    assert!(
        FlowRuntime::restore_checkpoint(image, &codecs(), FlowCheckpointLimits::default()).is_err()
    );
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
    let checkpoint = CONTEXT_IMAGE.with(|value| value.borrow().clone().unwrap());
    assert_eq!(checkpoint.segment_index, 0);
    assert_eq!(checkpoint.carrier, None);
    assert_eq!(checkpoint.kind, None);
    let restored =
        FlowRuntime::restore_checkpoint(image, &codecs(), FlowCheckpointLimits::default()).unwrap();
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
    let checkpoint = CONTEXT_IMAGE.with(|value| value.borrow().clone().unwrap());
    assert_eq!(checkpoint.phase, TransitPhase::Paused);
    assert_eq!(checkpoint.paused_from, Some(TransitPhase::Moving));
    assert_eq!(checkpoint.segment_index, checkpoint.route.segments.len());
    let restored =
        FlowRuntime::restore_checkpoint(image, &codecs(), FlowCheckpointLimits::default()).unwrap();
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
    let context_image = CONTEXT_IMAGE.with(|value| value.borrow().clone().unwrap());
    assert_eq!(context_image.phase, TransitPhase::Arrived);
    assert!(context_image.arrival_ticket.is_some());
    assert_eq!(
        context_image.segment_index,
        context_image.route.segments.len()
    );
    assert_eq!(context_image.elapsed_in_segment_ticks, 0);

    let restored =
        FlowRuntime::restore_checkpoint(image, &codecs(), FlowCheckpointLimits::default()).unwrap();
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
    let context_image = CONTEXT_IMAGE.with(|value| value.borrow().clone().unwrap());
    let stale = context_image.expected_event.unwrap();
    assert_eq!(context_image.expected_due_ticks, None);

    let restored =
        FlowRuntime::restore_checkpoint(image, &codecs(), FlowCheckpointLimits::default()).unwrap();
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
