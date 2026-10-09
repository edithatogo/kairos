use kairo_ecs_des::{
    FlowBatchReceipt, FlowCallbackCodeV1, FlowCallbackSnapshot, FlowCheckpointCodecs,
    FlowCheckpointLimits, FlowCheckpointWireError, FlowCheckpointWireLimits, FlowCommandSink,
    FlowCommandV1, FlowDomainControl, FlowError, FlowHandlerCodeIds, FlowNotificationV1,
    FlowOwnedCommand, FlowRuntime, FlowWorldView, LifecycleTransition, WorkHandlers,
};
use kairo_ecs_types::{EventKind, SimDuration, SimTime};

const DOMAIN: EventKind = EventKind::custom(771);

fn encode_u64(
    value: &u64,
    remaining: usize,
) -> Result<Vec<u8>, kairo_ecs_des::FlowCheckpointCodecError> {
    if remaining < 8 {
        return Err(kairo_ecs_des::FlowCheckpointCodecError(
            "payload budget".into(),
        ));
    }
    Ok(value.to_le_bytes().to_vec())
}

fn decode_u64(
    bytes: &[u8],
    _: &kairo_ecs_des::FlowCheckpointRebindV1,
) -> Result<u64, kairo_ecs_des::FlowCheckpointCodecError> {
    let bytes: [u8; 8] = bytes
        .try_into()
        .map_err(|_| kairo_ecs_des::FlowCheckpointCodecError("u64 payload".into()))?;
    Ok(u64::from_le_bytes(bytes))
}

fn encode_empty(_: &(), _: usize) -> Result<Vec<u8>, kairo_ecs_des::FlowCheckpointCodecError> {
    Ok(Vec::new())
}

fn decode_empty(
    bytes: &[u8],
    _: &kairo_ecs_des::FlowCheckpointRebindV1,
) -> Result<(), kairo_ecs_des::FlowCheckpointCodecError> {
    if bytes.is_empty() {
        Ok(())
    } else {
        Err(kairo_ecs_des::FlowCheckpointCodecError(
            "empty payload expected".into(),
        ))
    }
}

fn make_empty(_: &()) {}

fn codecs() -> FlowCheckpointCodecs {
    let mut codecs = FlowCheckpointCodecs::new();
    codecs
        .register_context::<u64>("unit.context.v1", 1, encode_u64, decode_u64)
        .unwrap();
    codecs
        .register_restart_template::<u64, u64>("unit.restart.v1", 1, encode_u64, decode_u64)
        .unwrap();
    codecs
        .register_restart_factory::<u64, u64>("unit.restart.v1", "factory.v1", make_context)
        .unwrap();
    codecs
}

fn make_context(template: &u64) -> u64 {
    *template + 1
}

fn emit_domain(context: &mut u64, snapshot: &FlowCallbackSnapshot, sink: &mut FlowCommandSink) {
    *context += 1;
    let at = snapshot
        .delivery
        .at
        .checked_add(SimDuration::from_ticks(1))
        .unwrap();
    sink.emit(FlowOwnedCommand::Domain {
        work: snapshot.work,
        kind: DOMAIN,
        at,
        scheduler_priority: 3,
    })
    .unwrap();
    sink.emit(FlowOwnedCommand::DomainControl {
        work: snapshot.work,
        kind: DOMAIN,
        action: FlowDomainControl::Pause,
        at,
        scheduler_priority: 4,
    })
    .unwrap();
}

fn emit_domain_view<'a>(
    context: &'a mut u64,
    snapshot: &'a FlowCallbackSnapshot,
    _view: FlowWorldView<'a>,
    sink: &'a mut FlowCommandSink,
) {
    emit_domain(context, snapshot, sink)
}

fn wire_limits() -> FlowCheckpointWireLimits {
    FlowCheckpointWireLimits::default()
}

fn domain_codecs() -> FlowCheckpointCodecs {
    let mut codecs = codecs();
    codecs
        .register_work_handlers::<u64>(
            "unit",
            FlowHandlerCodeIds::default(),
            WorkHandlers::default(),
        )
        .unwrap();
    codecs
        .register_domain_view_hook(
            "unit",
            DOMAIN,
            emit_domain_view,
            FlowCallbackCodeV1 {
                stable_id: "domain.emit".into(),
                version: 1,
            },
        )
        .unwrap();
    codecs
}

#[test]
fn complete_native_image_roundtrips_and_restores_suffix_state() {
    let mut source = FlowRuntime::new();
    let owner = source.spawn_actor().unwrap();
    let resource = source.create_resource(1).unwrap();
    let work = source
        .create_restartable_work(owner, SimDuration::from_ticks(4), "unit", 90, make_context)
        .unwrap();
    source
        .acquire(resource)
        .owner(owner)
        .timed_work(work)
        .submit()
        .unwrap();
    let first = source.step().unwrap().unwrap();
    let codecs = codecs();
    let image = source
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    let bytes = image.encode_wire_v1(wire_limits()).unwrap();
    let decoded = kairo_ecs_des::FlowCheckpointV1::decode_wire_v1(&bytes, wire_limits()).unwrap();
    assert_eq!(decoded, image);

    let (mut restored, _) = FlowRuntime::restore_checkpoint_with_rebind(
        decoded,
        &codecs,
        FlowCheckpointLimits::default(),
    )
    .unwrap();
    assert_eq!(source.step().unwrap(), restored.step().unwrap());
    let source_next = source
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    let restored_next = restored
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    assert_eq!(
        source_next.encode_wire_v1(wire_limits()).unwrap(),
        restored_next.encode_wire_v1(wire_limits()).unwrap()
    );

    let dispatch_bytes = first.encode_wire_v1(wire_limits()).unwrap();
    assert_eq!(
        kairo_ecs_des::FlowDispatch::decode_wire_v1(&dispatch_bytes, wire_limits()).unwrap(),
        first,
    );

    let mut too_few_payload_bytes = wire_limits();
    too_few_payload_bytes.flow.max_payload_bytes = 7;
    assert!(matches!(
        image.encode_wire_v1(too_few_payload_bytes),
        Err(FlowCheckpointWireError::LimitExceeded(
            "aggregate payload byte"
        ))
    ));

    // Actor cleanup leaves the registered restart store materialized but empty.
    let mut empty_source = FlowRuntime::new();
    let actor = empty_source.spawn_actor().unwrap();
    empty_source
        .create_restartable_work(actor, SimDuration::from_ticks(1), "unit", 5, make_context)
        .unwrap();
    empty_source.despawn_actor(actor).unwrap();
    empty_source.step().unwrap();
    let empty_image = empty_source
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    assert_eq!(
        empty_image.restart_store_manifest,
        vec![("unit.restart.v1".into(), 1)]
    );
    assert!(empty_image.restart_stores[0].rows.is_empty());
    let empty_bytes = empty_image.encode_wire_v1(wire_limits()).unwrap();
    assert_eq!(
        kairo_ecs_des::FlowCheckpointV1::decode_wire_v1(&empty_bytes, wire_limits()).unwrap(),
        empty_image
    );
}

#[test]
fn short_empty_context_and_restart_rows_roundtrip_at_minimum_wire_width() {
    let mut codecs = FlowCheckpointCodecs::new();
    codecs
        .register_context("z", 1, encode_empty, decode_empty)
        .unwrap();
    codecs
        .register_restart_template::<(), ()>("s", 1, encode_empty, decode_empty)
        .unwrap();
    codecs
        .register_restart_factory::<(), ()>("s", "f", make_empty)
        .unwrap();

    let mut source = FlowRuntime::new();
    let owner = source.spawn_actor().unwrap();
    for _ in 0..8 {
        source
            .create_work(owner, SimDuration::from_ticks(1), "z", ())
            .unwrap();
        source
            .create_restartable_work(owner, SimDuration::from_ticks(1), "z", (), make_empty)
            .unwrap();
    }
    let image = source
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    assert_eq!(image.context_stores[0].rows.len(), 16);
    assert_eq!(image.restart_stores[0].rows.len(), 8);
    assert!(image.restart_stores[0]
        .rows
        .iter()
        .all(|(_, factory, payload)| factory == "f" && payload.is_empty()));

    let bytes = image.encode_wire_v1(wire_limits()).unwrap();
    let decoded = kairo_ecs_des::FlowCheckpointV1::decode_wire_v1(&bytes, wire_limits()).unwrap();
    assert_eq!(decoded, image);

    let registration_total = image.contexts.len()
        + image.context_stores.len()
        + image.restart_store_manifest.len()
        + image.restart_stores.len();
    let mut exact_registration_cap = wire_limits();
    exact_registration_cap.flow.max_registrations = registration_total;
    let exact_bytes = image.encode_wire_v1(exact_registration_cap).unwrap();
    assert_eq!(
        kairo_ecs_des::FlowCheckpointV1::decode_wire_v1(&exact_bytes, exact_registration_cap)
            .unwrap(),
        image
    );
    let mut short_registration_cap = exact_registration_cap;
    short_registration_cap.flow.max_registrations = registration_total - 1;
    assert!(matches!(
        image.encode_wire_v1(short_registration_cap),
        Err(FlowCheckpointWireError::LimitExceeded("Flow collection"))
    ));
    assert!(matches!(
        kairo_ecs_des::FlowCheckpointV1::decode_wire_v1(&exact_bytes, short_registration_cap),
        Err(FlowCheckpointWireError::LimitExceeded(
            "aggregate registrations"
        ))
    ));

    let _restored = FlowRuntime::restore_checkpoint_with_rebind(
        decoded,
        &codecs,
        FlowCheckpointLimits::default(),
    )
    .unwrap();
}

#[test]
fn decoder_rejects_schema_truncation_trailing_invalid_tags_and_wire_caps() {
    let mut source = FlowRuntime::new();
    let owner = source.spawn_actor().unwrap();
    source
        .create_restartable_work(owner, SimDuration::from_ticks(2), "unit", 42, make_context)
        .unwrap();
    let codecs = codecs();
    let image = source
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    let bytes = image.encode_wire_v1(wire_limits()).unwrap();

    let mut unsupported = bytes.clone();
    unsupported[8..10].copy_from_slice(&2_u16.to_le_bytes());
    assert!(matches!(
        kairo_ecs_des::FlowCheckpointV1::decode_wire_v1(&unsupported, wire_limits()),
        Err(FlowCheckpointWireError::UnsupportedSchema(2))
    ));
    assert_eq!(
        kairo_ecs_des::FlowCheckpointV1::decode_wire_v1(&bytes[..bytes.len() - 1], wire_limits()),
        Err(FlowCheckpointWireError::Truncated)
    );
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert_eq!(
        kairo_ecs_des::FlowCheckpointV1::decode_wire_v1(&trailing, wire_limits()),
        Err(FlowCheckpointWireError::TrailingBytes)
    );
    let mut invalid_bool = bytes.clone();
    invalid_bool[134] = 2; // First world-slot `alive` flag in the fixed schema.
    assert_eq!(
        kairo_ecs_des::FlowCheckpointV1::decode_wire_v1(&invalid_bool, wire_limits()),
        Err(FlowCheckpointWireError::InvalidBoolean)
    );
    let mut invalid_utf8 = bytes.clone();
    let key_offset = invalid_utf8.windows(4).position(|w| w == b"unit").unwrap();
    invalid_utf8[key_offset] = 0xff;
    assert_eq!(
        kairo_ecs_des::FlowCheckpointV1::decode_wire_v1(&invalid_utf8, wire_limits()),
        Err(FlowCheckpointWireError::InvalidUtf8)
    );
    let mut oversized_count = bytes.clone();
    oversized_count[110..118].copy_from_slice(&u64::MAX.to_le_bytes()); // Scheduler entry count.
    assert!(matches!(
        kairo_ecs_des::FlowCheckpointV1::decode_wire_v1(&oversized_count, wire_limits()),
        Err(FlowCheckpointWireError::LimitExceeded("scheduler entry"))
    ));
    let mut too_many_short_records = bytes.clone();
    too_many_short_records[110..118].copy_from_slice(&1_000_u64.to_le_bytes());
    let mut enough_scheduler_capacity = wire_limits();
    enough_scheduler_capacity.flow.max_scheduler_entries = 1_000;
    assert_eq!(
        kairo_ecs_des::FlowCheckpointV1::decode_wire_v1(
            &too_many_short_records,
            enough_scheduler_capacity,
        ),
        Err(FlowCheckpointWireError::Truncated)
    );
    let mut too_few_keys = wire_limits();
    too_few_keys.flow.max_key_bytes = 0;
    assert!(matches!(
        kairo_ecs_des::FlowCheckpointV1::decode_wire_v1(&bytes, too_few_keys),
        Err(FlowCheckpointWireError::LimitExceeded("aggregate key byte"))
    ));
    let mut too_few_payload_bytes = wire_limits();
    too_few_payload_bytes.flow.max_payload_bytes = 7;
    assert!(matches!(
        kairo_ecs_des::FlowCheckpointV1::decode_wire_v1(&bytes, too_few_payload_bytes),
        Err(FlowCheckpointWireError::LimitExceeded(
            "aggregate payload byte"
        ))
    ));
    let mut short = wire_limits();
    short.max_wire_bytes = bytes.len() - 1;
    assert!(matches!(
        kairo_ecs_des::FlowCheckpointV1::decode_wire_v1(&bytes, short),
        Err(FlowCheckpointWireError::LimitExceeded("total byte"))
    ));
}

#[test]
fn domain_dispatch_tickets_and_every_flow_error_variant_roundtrip() {
    let mut source = FlowRuntime::new();
    let actor = source.spawn_actor().unwrap();
    source
        .register_work_handlers::<u64>("unit", WorkHandlers::default())
        .unwrap();
    source
        .register_domain_view_hook("unit", DOMAIN, emit_domain_view)
        .unwrap();
    let work = source
        .create_actor_domain_context(actor, "unit", DOMAIN, 10_u64)
        .unwrap();
    source
        .schedule_domain(work, DOMAIN, SimTime::from_ticks(1), 0)
        .unwrap();
    let dispatch = source.step().unwrap().unwrap();
    let FlowBatchReceipt::Accepted(admissions) = &dispatch.callback_batches[0] else {
        panic!("expected accepted callback batch")
    };
    assert_eq!(admissions.len(), 2);

    let mut all_errors = vec![
        FlowError::InvalidEntity,
        FlowError::InvalidResource,
        FlowError::InvalidRequest,
        FlowError::TerminalRequest,
        FlowError::InvalidLease,
        FlowError::CapacityInUse,
        FlowError::ResourceInUse,
        FlowError::PastCommand,
        FlowError::CounterOverflow,
        FlowError::InvalidState,
        FlowError::InvalidWork,
        FlowError::DuplicateActorDomainContext,
        FlowError::DuplicateActorDespawn,
        FlowError::ReservedEventKind,
        FlowError::UnregisteredDomainEvent,
        FlowError::InvalidCommandTicket,
        FlowError::CallbackBatchLimitExceeded,
        FlowError::SameTickBudgetExceeded {
            at_ticks: 17,
            limit: 9,
        },
        FlowError::RunHalted,
    ];
    let mut receipt = dispatch.clone();
    receipt.callback_batches.push(FlowBatchReceipt::Rejected(
        kairo_ecs_des::FlowBatchRejection {
            failed_ticket: Some(admissions[0].ticket),
            error: FlowError::InvalidState,
        },
    ));
    for error in all_errors.drain(..) {
        receipt.error = Some(error);
        let bytes = receipt.encode_wire_v1(wire_limits()).unwrap();
        assert_eq!(
            kairo_ecs_des::FlowDispatch::decode_wire_v1(&bytes, wire_limits()).unwrap(),
            receipt
        );
    }

    let codecs = domain_codecs();
    let image = source
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    let bytes = image.encode_wire_v1(wire_limits()).unwrap();
    let decoded = kairo_ecs_des::FlowCheckpointV1::decode_wire_v1(&bytes, wire_limits()).unwrap();
    let (mut restored, _) = FlowRuntime::restore_checkpoint_with_rebind(
        decoded,
        &codecs,
        FlowCheckpointLimits::default(),
    )
    .unwrap();
    assert_eq!(source.step().unwrap(), restored.step().unwrap());
    let source_suffix = source
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap()
        .encode_wire_v1(wire_limits())
        .unwrap();
    let restored_suffix = restored
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap()
        .encode_wire_v1(wire_limits())
        .unwrap();
    assert_eq!(source_suffix, restored_suffix);
}

#[test]
fn checkpoint_wire_covers_each_command_variant_and_notification_payload() {
    let mut source = FlowRuntime::new();
    let owner = source.spawn_actor().unwrap();
    let resource = source.create_resource(2).unwrap();
    let work = source
        .create_work(owner, SimDuration::from_ticks(7), "unit", 55_u64)
        .unwrap();
    let request = source
        .acquire(resource)
        .owner(owner)
        .timed_work(work)
        .submit()
        .unwrap();
    let grant = source.step().unwrap().unwrap();
    let lease = grant
        .records
        .iter()
        .find_map(|r| r.lease)
        .expect("grant lease");
    let codecs = codecs();
    let mut image = source
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    image.commands = vec![
        (
            kairo_ecs_types::EventId::new(100, 100),
            FlowCommandV1::Submit(request),
        ),
        (
            kairo_ecs_types::EventId::new(101, 101),
            FlowCommandV1::Release(lease),
        ),
        (
            kairo_ecs_types::EventId::new(102, 102),
            FlowCommandV1::Capacity(resource, 3),
        ),
        (
            kairo_ecs_types::EventId::new(103, 103),
            FlowCommandV1::Remove(resource),
        ),
        (
            kairo_ecs_types::EventId::new(104, 104),
            FlowCommandV1::Despawn(owner),
        ),
        (
            kairo_ecs_types::EventId::new(105, 105),
            FlowCommandV1::Deadline(request),
        ),
        (
            kairo_ecs_types::EventId::new(106, 106),
            FlowCommandV1::Cancel(request),
        ),
        (
            kairo_ecs_types::EventId::new(107, 107),
            FlowCommandV1::Reprioritize(request, 9),
        ),
        (
            kairo_ecs_types::EventId::new(108, 108),
            FlowCommandV1::Completion(request, lease, lease.revision(), SimTime::from_ticks(8)),
        ),
        (
            kairo_ecs_types::EventId::new(109, 109),
            FlowCommandV1::Notify,
        ),
        (
            kairo_ecs_types::EventId::new(110, 110),
            FlowCommandV1::Domain(work, DOMAIN),
        ),
        (
            kairo_ecs_types::EventId::new(111, 111),
            FlowCommandV1::DomainControl(work, DOMAIN, FlowDomainControl::Resume),
        ),
    ];
    let progress = image.builtins.work_progress.as_ref().unwrap().rows[0]
        .1
        .clone();
    image.notifications = vec![(
        kairo_ecs_types::EventId::new(112, 112),
        FlowNotificationV1 {
            kind: 1,
            work,
            transition: LifecycleTransition::Restarted,
            progress,
            origin: kairo_ecs_types::EventId::new(99, 99),
            ordinal: 7,
        },
    )];
    let bytes = image.encode_wire_v1(wire_limits()).unwrap();
    assert_eq!(
        kairo_ecs_des::FlowCheckpointV1::decode_wire_v1(&bytes, wire_limits()).unwrap(),
        image
    );
    let mut unknown_command = bytes.clone();
    let notify_event = 109_u64.to_le_bytes();
    let event_at = unknown_command
        .windows(8)
        .position(|w| w == notify_event)
        .unwrap();
    unknown_command[event_at + 12] = 0xff;
    assert_eq!(
        kairo_ecs_des::FlowCheckpointV1::decode_wire_v1(&unknown_command, wire_limits()),
        Err(FlowCheckpointWireError::InvalidTag)
    );
    let mut duplicate = image.clone();
    duplicate.resources.push(duplicate.resources[0]);
    assert!(matches!(
        duplicate.encode_wire_v1(wire_limits()),
        Err(FlowCheckpointWireError::InvalidValue(
            "noncanonical order or duplicate entry"
        ))
    ));
}

#[test]
fn world_lifo_free_stack_and_component_dense_order_survive_wire_restore() {
    let mut source = FlowRuntime::new();
    let first = source.create_resource(1).unwrap();
    let removed = source.create_resource(2).unwrap();
    let last = source.create_resource(3).unwrap();
    source.remove_resource(removed).unwrap();
    source.step().unwrap();
    let codecs = FlowCheckpointCodecs::new();
    let image = source
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    assert_eq!(image.world.free_indices, vec![removed.entity_id().index]);
    let capacities = image.builtins.capacities.as_ref().unwrap();
    assert_eq!(capacities.sparse_slots, 3);
    assert_eq!(
        capacities
            .rows
            .iter()
            .map(|(id, _)| *id)
            .collect::<Vec<_>>(),
        vec![first.entity_id(), last.entity_id()]
    );
    let bytes = image.encode_wire_v1(wire_limits()).unwrap();
    let decoded = kairo_ecs_des::FlowCheckpointV1::decode_wire_v1(&bytes, wire_limits()).unwrap();
    assert_eq!(decoded, image);
    let mut restored =
        FlowRuntime::restore_checkpoint(decoded, &codecs, FlowCheckpointLimits::default()).unwrap();
    assert_eq!(
        source.create_resource(4).unwrap(),
        restored.create_resource(4).unwrap()
    );
    assert_eq!(
        source
            .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
            .unwrap()
            .encode_wire_v1(wire_limits())
            .unwrap(),
        restored
            .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
            .unwrap()
            .encode_wire_v1(wire_limits())
            .unwrap()
    );
}
