use kairo_ecs_des::fidelity::{
    FidelityAdapter, FidelityAdapterCheckpointV1, FidelityCheckpointLimits,
    FidelityCheckpointWireError, FidelityCheckpointWireLimits, FidelityDecision, FidelityMode,
    FidelityPolicy, FidelityScope,
};
use kairo_ecs_des::{FlowCheckpointCodecs, FlowCheckpointLimits, FlowRuntime, WorkId};
use kairo_ecs_types::{EntityId, SimDuration};

fn encode_u64(
    value: &u64,
    remaining: usize,
) -> Result<Vec<u8>, kairo_ecs_des::FlowCheckpointCodecError> {
    if remaining < 8 {
        return Err(kairo_ecs_des::FlowCheckpointCodecError(
            "payload cap".into(),
        ));
    }
    Ok(value.to_le_bytes().to_vec())
}

fn decode_u64(
    bytes: &[u8],
    _: &kairo_ecs_des::FlowCheckpointRebindV1,
) -> Result<u64, kairo_ecs_des::FlowCheckpointCodecError> {
    let raw: [u8; 8] = bytes
        .try_into()
        .map_err(|_| kairo_ecs_des::FlowCheckpointCodecError("u64 payload".into()))?;
    Ok(u64::from_le_bytes(raw))
}

fn codecs() -> FlowCheckpointCodecs {
    let mut codecs = FlowCheckpointCodecs::new();
    codecs
        .register_context("fidelity.wire.v1", 1, encode_u64, decode_u64)
        .unwrap();
    codecs
}

fn fidelity_limits() -> FidelityCheckpointLimits {
    FidelityCheckpointLimits {
        max_admitted: 16,
        max_overrides: 24,
        max_subsystem_bytes: 4096,
    }
}

fn wire_limits() -> FidelityCheckpointWireLimits {
    FidelityCheckpointWireLimits {
        checkpoint: fidelity_limits(),
        max_wire_bytes: 64 * 1024,
        max_total_records: 128,
    }
}

fn create_work(flow: &mut FlowRuntime, duration: u128) -> WorkId {
    let actor = flow.spawn_actor().unwrap();
    flow.create_work(
        actor,
        SimDuration::from_ticks(duration),
        "fidelity.wire.v1",
        19_u64,
    )
    .unwrap()
}

fn restored_flow(source: &FlowRuntime) -> (FlowRuntime, kairo_ecs_des::FlowCheckpointRebindV1) {
    let image = source
        .capture_checkpoint(&codecs(), FlowCheckpointLimits::default())
        .unwrap();
    FlowRuntime::restore_checkpoint_with_rebind(image, &codecs(), FlowCheckpointLimits::default())
        .unwrap()
}

#[test]
fn wire_roundtrip_preserves_current_pending_overrides_and_frozen_admissions() {
    let mut source = FlowRuntime::new();
    let macro_work = create_work(&mut source, 5);
    let micro_work = create_work(&mut source, 7);
    let micro_actor = source.work(micro_work).unwrap().owner;
    let future_actor = EntityId::new(900, 17);
    let mut current = FidelityPolicy::new(1, Some(FidelityMode::Macro)).unwrap();
    current
        .set_entity_subsystem(micro_actor, "triage", FidelityMode::Micro)
        .unwrap();
    // Overrides need not refer to an actor that is live at this checkpoint.
    current
        .set_entity(future_actor, FidelityMode::Micro)
        .unwrap();
    current
        .set_subsystem("ward/β", FidelityMode::Macro)
        .unwrap();
    let mut adapter = FidelityAdapter::new(current);
    let frozen_macro = adapter.admit(&source, macro_work, "triage").unwrap();
    let frozen_micro = adapter.admit(&source, micro_work, "triage").unwrap();
    let mut pending = FidelityPolicy::new(1, Some(FidelityMode::Micro)).unwrap();
    pending
        .set_entity_subsystem(micro_actor, "triage", FidelityMode::Macro)
        .unwrap();
    pending
        .set_entity_subsystem(future_actor, "future", FidelityMode::Macro)
        .unwrap();
    adapter.stage_policy(pending);

    let resource = source.create_resource(2).unwrap();
    for work in [macro_work, micro_work] {
        let owner = source.work(work).unwrap().owner;
        source
            .acquire(resource)
            .owner(owner)
            .timed_work(work)
            .submit()
            .unwrap();
    }
    for _ in 0..2 {
        source.step().unwrap().unwrap();
    }
    for work in [macro_work, micro_work] {
        assert_eq!(
            source.work_progress(work).unwrap().state,
            kairo_ecs_des::WorkState::Active
        );
    }

    let image = adapter.checkpoint(&source, fidelity_limits()).unwrap();
    let bytes = image.encode_wire_v1(wire_limits()).unwrap();
    let (mut fresh_flow, view) = restored_flow(&source);
    assert_ne!(source.identity(), fresh_flow.identity());
    let decoded =
        FidelityAdapterCheckpointV1::decode_wire_v1(&bytes, &view, wire_limits()).unwrap();
    let work_mapping: Vec<_> = image
        .admitted
        .iter()
        .map(|(old, _)| (*old, view.resolve_work(old.entity_id()).unwrap()))
        .collect();
    let mut restored =
        FidelityAdapter::from_checkpoint(decoded, &fresh_flow, &work_mapping, fidelity_limits())
            .unwrap();

    assert_eq!(restored.decision(macro_work), Some(&frozen_macro));
    assert_eq!(restored.decision(micro_work), Some(&frozen_micro));
    assert_eq!(frozen_macro.mode, FidelityMode::Macro);
    assert_eq!(frozen_micro.mode, FidelityMode::Micro);
    assert_eq!(frozen_micro.scope, FidelityScope::EntitySubsystem);
    assert_eq!(
        restored.checkpoint(&fresh_flow, fidelity_limits()).unwrap(),
        image
    );
    assert_eq!(
        restored.apply_at_boundary(&fresh_flow),
        Err(kairo_ecs_des::fidelity::FidelityError::BusyBoundary)
    );
    assert_eq!(
        restored.decision(macro_work),
        Some(&FidelityDecision {
            mode: FidelityMode::Macro,
            scope: FidelityScope::Global,
            policy_version: 1,
        })
    );
    // A failed boundary operation must leave both the staged policy and Flow intact.
    assert!(restored
        .checkpoint(&fresh_flow, fidelity_limits())
        .unwrap()
        .pending
        .is_some());
    assert_eq!(source.step().unwrap(), fresh_flow.step().unwrap());
    assert_eq!(
        source
            .capture_checkpoint(&codecs(), FlowCheckpointLimits::default())
            .unwrap(),
        fresh_flow
            .capture_checkpoint(&codecs(), FlowCheckpointLimits::default())
            .unwrap()
    );
}

#[test]
fn unbound_empty_image_roundtrips_and_noncanonical_policy_is_rejected() {
    let empty = FidelityAdapter::new(FidelityPolicy::new(1, None).unwrap());
    let source = FlowRuntime::new();
    let image = empty.checkpoint(&source, fidelity_limits()).unwrap();
    assert!(!image.bound_runtime);
    let bytes = image.encode_wire_v1(wire_limits()).unwrap();
    let (target, view) = restored_flow(&source);
    let decoded =
        FidelityAdapterCheckpointV1::decode_wire_v1(&bytes, &view, wire_limits()).unwrap();
    let restored =
        FidelityAdapter::from_checkpoint(decoded, &target, &[], fidelity_limits()).unwrap();
    assert_eq!(
        restored.checkpoint(&target, fidelity_limits()).unwrap(),
        image
    );
    let mut impossible_binding = image.clone();
    impossible_binding.bound_runtime = true;
    assert_eq!(
        impossible_binding.encode_wire_v1(wire_limits()),
        Err(FidelityCheckpointWireError::InvalidBinding)
    );

    let mut noncanonical = image.clone();
    noncanonical.current.entity_overrides = vec![
        (EntityId::new(2, 1), FidelityMode::Macro),
        (EntityId::new(1, 1), FidelityMode::Micro),
    ];
    assert_eq!(
        noncanonical.encode_wire_v1(wire_limits()),
        Err(FidelityCheckpointWireError::NonCanonical)
    );
}

#[test]
fn decoder_rejects_bounds_tags_versions_utf8_truncation_and_unknown_work() {
    let mut source = FlowRuntime::new();
    let work = create_work(&mut source, 3);
    let mut adapter =
        FidelityAdapter::new(FidelityPolicy::new(1, Some(FidelityMode::Macro)).unwrap());
    adapter.admit(&source, work, "ed").unwrap();
    let image = adapter.checkpoint(&source, fidelity_limits()).unwrap();
    let bytes = image.encode_wire_v1(wire_limits()).unwrap();
    let (target, view) = restored_flow(&source);

    assert_eq!(
        FidelityAdapterCheckpointV1::decode_wire_v1(
            &bytes[..bytes.len() - 1],
            &view,
            wire_limits()
        ),
        Err(FidelityCheckpointWireError::Truncated)
    );
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert_eq!(
        FidelityAdapterCheckpointV1::decode_wire_v1(&trailing, &view, wire_limits()),
        Err(FidelityCheckpointWireError::TrailingBytes)
    );
    let mut bad_schema = bytes.clone();
    bad_schema[8..10].copy_from_slice(&2_u16.to_le_bytes());
    assert_eq!(
        FidelityAdapterCheckpointV1::decode_wire_v1(&bad_schema, &view, wire_limits()),
        Err(FidelityCheckpointWireError::UnsupportedSchema(2))
    );
    let mut bad_mode = bytes.clone();
    bad_mode[19] = 0xff;
    assert_eq!(
        FidelityAdapterCheckpointV1::decode_wire_v1(&bad_mode, &view, wire_limits()),
        Err(FidelityCheckpointWireError::InvalidTag)
    );
    let mut bad_boolean = bytes.clone();
    *bad_boolean.last_mut().unwrap() = 2;
    assert_eq!(
        FidelityAdapterCheckpointV1::decode_wire_v1(&bad_boolean, &view, wire_limits()),
        Err(FidelityCheckpointWireError::InvalidBoolean)
    );
    let mut wrong_generation = bytes.clone();
    let generation = u32::from_le_bytes(wrong_generation[61..65].try_into().unwrap());
    wrong_generation[61..65].copy_from_slice(&generation.wrapping_add(1).to_le_bytes());
    assert_eq!(
        FidelityAdapterCheckpointV1::decode_wire_v1(&wrong_generation, &view, wire_limits()),
        Err(FidelityCheckpointWireError::InvalidWorkReference)
    );
    let mut missing_binding = bytes.clone();
    *missing_binding.last_mut().unwrap() = 0;
    assert_eq!(
        FidelityAdapterCheckpointV1::decode_wire_v1(&missing_binding, &view, wire_limits()),
        Err(FidelityCheckpointWireError::InvalidBinding)
    );
    let mut tiny_wire = wire_limits();
    tiny_wire.max_wire_bytes = bytes.len() - 1;
    assert!(matches!(
        image.encode_wire_v1(tiny_wire),
        Err(FidelityCheckpointWireError::LimitExceeded(_))
    ));
    assert!(matches!(
        FidelityAdapterCheckpointV1::decode_wire_v1(&bytes, &view, tiny_wire),
        Err(FidelityCheckpointWireError::LimitExceeded("wire byte"))
    ));
    let mut tiny_records = wire_limits();
    tiny_records.max_total_records = 0;
    assert!(matches!(
        image.encode_wire_v1(tiny_records),
        Err(FidelityCheckpointWireError::LimitExceeded(_))
    ));

    let unrelated = FlowRuntime::new();
    let (_other_fresh, other_view) = restored_flow(&unrelated);
    assert_eq!(
        FidelityAdapterCheckpointV1::decode_wire_v1(&bytes, &other_view, wire_limits()),
        Err(FidelityCheckpointWireError::InvalidWorkReference)
    );
    assert!(target.work(work).is_ok());

    let mut keyed_policy = FidelityPolicy::new(1, None).unwrap();
    keyed_policy
        .set_subsystem("ed/β", FidelityMode::Micro)
        .unwrap();
    let keyed = FidelityAdapter::new(keyed_policy)
        .checkpoint(&source, fidelity_limits())
        .unwrap();
    let keyed_bytes = keyed.encode_wire_v1(wire_limits()).unwrap();
    let key_offset = keyed_bytes
        .windows("ed/β".len())
        .position(|window| window == "ed/β".as_bytes())
        .unwrap();
    let mut invalid_utf8 = keyed_bytes.clone();
    invalid_utf8[key_offset] = 0xff;
    assert_eq!(
        FidelityAdapterCheckpointV1::decode_wire_v1(&invalid_utf8, &view, wire_limits()),
        Err(FidelityCheckpointWireError::InvalidUtf8)
    );
    let mut too_few_subsystem_bytes = wire_limits();
    too_few_subsystem_bytes.checkpoint.max_subsystem_bytes = 4;
    assert!(matches!(
        keyed.encode_wire_v1(too_few_subsystem_bytes),
        Err(FidelityCheckpointWireError::LimitExceeded("subsystem byte"))
    ));
    assert!(matches!(
        FidelityAdapterCheckpointV1::decode_wire_v1(&keyed_bytes, &view, too_few_subsystem_bytes),
        Err(FidelityCheckpointWireError::LimitExceeded("subsystem byte"))
    ));

    let mut bad_native = image;
    bad_native.version = 2;
    assert_eq!(
        bad_native.encode_wire_v1(wire_limits()),
        Err(FidelityCheckpointWireError::UnsupportedVersion(2))
    );
}
