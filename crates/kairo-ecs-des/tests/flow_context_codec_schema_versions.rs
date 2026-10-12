use kairo_ecs_des::{
    FlowCheckpointCodecs, FlowCheckpointError, FlowCheckpointLimits, FlowCheckpointV1,
    FlowCheckpointV2, FlowCheckpointV3, FlowCheckpointV4, FlowHandlerCodeIds, FlowRuntime,
    RequestState, WorkHandlers,
};
use kairo_ecs_types::SimDuration;
use std::sync::atomic::{AtomicUsize, Ordering};

static OWNER_DECODE_CALLS: AtomicUsize = AtomicUsize::new(0);
static PLAIN_DECODE_CALLS: AtomicUsize = AtomicUsize::new(0);
static ENCODE_CALLS: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Debug, Eq, PartialEq)]
struct OwnerContext(u64);

#[derive(Clone, Debug, Eq, PartialEq)]
struct PlainContext(u64);

#[derive(Clone, Debug, Eq, PartialEq)]
struct FreshContext(u64);

fn encode_owner(
    value: &OwnerContext,
    remaining: usize,
) -> Result<Vec<u8>, kairo_ecs_des::FlowCheckpointCodecError> {
    encode_u64(value.0, remaining)
}

fn decode_owner(
    bytes: &[u8],
    _: &kairo_ecs_des::FlowCheckpointRebindV1,
) -> Result<OwnerContext, kairo_ecs_des::FlowCheckpointCodecError> {
    OWNER_DECODE_CALLS.fetch_add(1, Ordering::SeqCst);
    decode_u64(bytes).map(OwnerContext)
}

fn encode_plain(
    value: &PlainContext,
    remaining: usize,
) -> Result<Vec<u8>, kairo_ecs_des::FlowCheckpointCodecError> {
    encode_u64(value.0, remaining)
}

fn decode_plain(
    bytes: &[u8],
    _: &kairo_ecs_des::FlowCheckpointRebindV1,
) -> Result<PlainContext, kairo_ecs_des::FlowCheckpointCodecError> {
    PLAIN_DECODE_CALLS.fetch_add(1, Ordering::SeqCst);
    decode_u64(bytes).map(PlainContext)
}

fn encode_fresh(
    value: &FreshContext,
    remaining: usize,
) -> Result<Vec<u8>, kairo_ecs_des::FlowCheckpointCodecError> {
    encode_u64(value.0, remaining)
}

fn decode_fresh(
    bytes: &[u8],
    _: &kairo_ecs_des::FlowCheckpointRebindV1,
) -> Result<FreshContext, kairo_ecs_des::FlowCheckpointCodecError> {
    decode_u64(bytes).map(FreshContext)
}

fn encode_u64(
    value: u64,
    remaining: usize,
) -> Result<Vec<u8>, kairo_ecs_des::FlowCheckpointCodecError> {
    ENCODE_CALLS.fetch_add(1, Ordering::SeqCst);
    if remaining < 8 {
        return Err(kairo_ecs_des::FlowCheckpointCodecError(
            "context payload budget".into(),
        ));
    }
    Ok(value.to_le_bytes().to_vec())
}

fn decode_u64(bytes: &[u8]) -> Result<u64, kairo_ecs_des::FlowCheckpointCodecError> {
    let payload: [u8; 8] = bytes
        .try_into()
        .map_err(|_| kairo_ecs_des::FlowCheckpointCodecError("invalid context payload".into()))?;
    Ok(u64::from_le_bytes(payload))
}

fn encode_numeric(
    value: &u64,
    remaining: usize,
) -> Result<Vec<u8>, kairo_ecs_des::FlowCheckpointCodecError> {
    encode_u64(*value, remaining)
}

fn decode_numeric(
    bytes: &[u8],
    _: &kairo_ecs_des::FlowCheckpointRebindV1,
) -> Result<u64, kairo_ecs_des::FlowCheckpointCodecError> {
    decode_u64(bytes)
}

fn codecs_for_owner(version: u32) -> FlowCheckpointCodecs {
    let mut codecs = FlowCheckpointCodecs::new();
    codecs
        .register_context_with_owner::<OwnerContext>(
            "owner.ctx",
            version,
            encode_owner,
            |bytes, _owner, _view| {
                OWNER_DECODE_CALLS.fetch_add(1, Ordering::SeqCst);
                decode_u64(bytes).map(OwnerContext)
            },
        )
        .unwrap();
    codecs
        .register_work_handlers::<OwnerContext>(
            "owner.ctx",
            FlowHandlerCodeIds::default(),
            WorkHandlers::default(),
        )
        .unwrap();
    codecs
}

fn context_versions(image: &FlowCheckpointV1) -> Vec<u32> {
    image.contexts.iter().map(|row| row.version).collect()
}

fn context_versions_v2(image: &FlowCheckpointV2) -> Vec<u32> {
    image.contexts.iter().map(|row| row.version).collect()
}

fn context_versions_v3(image: &FlowCheckpointV3) -> Vec<u32> {
    image.contexts.iter().map(|row| row.version).collect()
}

fn context_versions_v4(image: &FlowCheckpointV4) -> Vec<u32> {
    image.contexts.iter().map(|row| row.version).collect()
}

#[test]
fn context_codec_nonzero_versions_roundtrip_actual_runtime() {
    OWNER_DECODE_CALLS.store(0, Ordering::SeqCst);
    PLAIN_DECODE_CALLS.store(0, Ordering::SeqCst);
    ENCODE_CALLS.store(0, Ordering::SeqCst);
    let mut codecs = FlowCheckpointCodecs::new();
    codecs
        .register_context_with_owner::<OwnerContext>(
            "owner.ctx",
            2,
            encode_owner,
            |bytes, owner, view| {
                OWNER_DECODE_CALLS.fetch_add(1, Ordering::SeqCst);
                assert_eq!(view.resolve_work(owner).unwrap().entity_id(), owner);
                decode_u64(bytes).map(OwnerContext)
            },
        )
        .unwrap();
    codecs
        .register_context::<PlainContext>("plain.ctx", u32::MAX, encode_plain, decode_plain)
        .unwrap();
    codecs
        .register_work_handlers::<OwnerContext>(
            "owner.ctx",
            FlowHandlerCodeIds::default(),
            WorkHandlers::default(),
        )
        .unwrap();
    codecs
        .register_work_handlers::<PlainContext>(
            "plain.ctx",
            FlowHandlerCodeIds::default(),
            WorkHandlers::default(),
        )
        .unwrap();

    let mut source = FlowRuntime::new();
    let actor = source.spawn_actor().unwrap();
    source
        .register_work_handlers::<OwnerContext>("owner.ctx", WorkHandlers::default())
        .unwrap();
    source
        .register_work_handlers::<PlainContext>("plain.ctx", WorkHandlers::default())
        .unwrap();
    let owner_work = source
        .create_work(
            actor,
            SimDuration::from_ticks(3),
            "owner.ctx",
            OwnerContext(41),
        )
        .unwrap();
    let plain_work = source
        .create_work(
            actor,
            SimDuration::from_ticks(4),
            "plain.ctx",
            PlainContext(99),
        )
        .unwrap();
    let owner_resource = source.create_resource(1).unwrap();
    let plain_resource = source.create_resource(1).unwrap();
    let owner_request = source
        .acquire(owner_resource)
        .owner(actor)
        .timed_work(owner_work)
        .submit()
        .unwrap();
    let plain_request = source
        .acquire(plain_resource)
        .owner(actor)
        .timed_work(plain_work)
        .submit()
        .unwrap();

    let captured = source
        .capture_checkpoint_v4(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    assert_eq!(context_versions_v4(&captured), vec![2, u32::MAX]);
    assert_eq!(captured.context_stores[0].version, 2);
    assert_eq!(captured.context_stores[1].version, u32::MAX);
    let (mut restored, rebind) = FlowRuntime::restore_checkpoint_v4_with_rebind(
        captured,
        &codecs,
        FlowCheckpointLimits::default(),
    )
    .unwrap();
    assert_eq!(
        rebind.resolve_work(owner_work.entity_id()).unwrap(),
        owner_work
    );
    assert_eq!(
        restored.work_context::<OwnerContext>(owner_work).unwrap(),
        &OwnerContext(41)
    );
    assert_eq!(
        restored.work_context::<PlainContext>(plain_work).unwrap(),
        &PlainContext(99)
    );

    let source_run = source.run_for(12).unwrap();
    let restored_run = restored.run_for(12).unwrap();
    assert_eq!(source_run.dispatches, restored_run.dispatches);
    assert!(source_run
        .dispatches
        .iter()
        .all(|dispatch| dispatch.error.is_none()));
    assert!(restored_run
        .dispatches
        .iter()
        .all(|dispatch| dispatch.error.is_none()));
    assert!(!source_run.budget_exhausted && !restored_run.budget_exhausted);
    assert_eq!(source.budget_snapshot(), restored.budget_snapshot());
    assert_eq!(
        restored.request(owner_request).unwrap(),
        source.request(owner_request).unwrap()
    );
    assert_eq!(
        restored.request(plain_request).unwrap(),
        source.request(plain_request).unwrap()
    );
    assert_eq!(
        restored.work(owner_work).unwrap(),
        source.work(owner_work).unwrap()
    );
    assert_eq!(
        restored.work(plain_work).unwrap(),
        source.work(plain_work).unwrap()
    );
    assert_eq!(
        restored.work_context::<OwnerContext>(owner_work).unwrap(),
        source.work_context::<OwnerContext>(owner_work).unwrap()
    );
    assert_eq!(
        restored.work_context::<PlainContext>(plain_work).unwrap(),
        source.work_context::<PlainContext>(plain_work).unwrap()
    );
    assert_eq!(
        source.request(owner_request).unwrap().state,
        RequestState::Completed
    );
    assert_eq!(
        source.request(plain_request).unwrap().state,
        RequestState::Completed
    );
    let source_suffix = source
        .capture_checkpoint_v4(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    let restored_suffix = restored
        .capture_checkpoint_v4(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    assert_eq!(restored_suffix, source_suffix);
    assert!(OWNER_DECODE_CALLS.load(Ordering::SeqCst) > 0);
    assert!(PLAIN_DECODE_CALLS.load(Ordering::SeqCst) > 0);
}

#[test]
fn context_codec_zero_and_duplicate_registration_atomic() {
    OWNER_DECODE_CALLS.store(0, Ordering::SeqCst);
    PLAIN_DECODE_CALLS.store(0, Ordering::SeqCst);
    ENCODE_CALLS.store(0, Ordering::SeqCst);
    let mut codecs = FlowCheckpointCodecs::new();
    assert!(matches!(
        codecs.register_context::<OwnerContext>("zero.simple", 0, encode_owner, decode_owner),
        Err(FlowCheckpointError::IncompatibleRegistration(key)) if key == "zero.simple"
    ));
    assert!(matches!(
        codecs.register_context_with_owner::<PlainContext>(
            "zero.owner", 0, encode_plain,
            |bytes, _, _| decode_u64(bytes).map(PlainContext),
        ),
        Err(FlowCheckpointError::IncompatibleRegistration(key)) if key == "zero.owner"
    ));

    codecs
        .register_context::<OwnerContext>("zero.simple", 1, encode_owner, decode_owner)
        .unwrap();
    codecs
        .register_context::<PlainContext>("plain.ctx", 2, encode_plain, decode_plain)
        .unwrap();
    assert!(matches!(
        codecs.register_context::<u64>("plain.ctx", 1, encode_numeric, decode_numeric),
        Err(FlowCheckpointError::IncompatibleRegistration(key)) if key == "plain.ctx"
    ));
    codecs
        .register_context::<u64>("numeric.ctx", 1, encode_numeric, decode_numeric)
        .unwrap();
    assert!(matches!(
        codecs.register_context::<PlainContext>("plain.other", 1, encode_plain, decode_plain),
        Err(FlowCheckpointError::IncompatibleRegistration(key)) if key == "plain.other"
    ));
    codecs
        .register_context::<FreshContext>("plain.other", 1, encode_fresh, decode_fresh)
        .unwrap();
    codecs
        .register_work_handlers::<OwnerContext>(
            "zero.simple",
            FlowHandlerCodeIds::default(),
            WorkHandlers::default(),
        )
        .unwrap();
    codecs
        .register_work_handlers::<PlainContext>(
            "plain.ctx",
            FlowHandlerCodeIds::default(),
            WorkHandlers::default(),
        )
        .unwrap();

    assert_eq!(ENCODE_CALLS.load(Ordering::SeqCst), 0);
    let mut runtime = FlowRuntime::new();
    let actor = runtime.spawn_actor().unwrap();
    runtime
        .register_work_handlers::<OwnerContext>("zero.simple", WorkHandlers::default())
        .unwrap();
    runtime
        .register_work_handlers::<PlainContext>("plain.ctx", WorkHandlers::default())
        .unwrap();
    runtime
        .create_work(actor, SimDuration::ZERO, "zero.simple", OwnerContext(5))
        .unwrap();
    runtime
        .create_work(actor, SimDuration::ZERO, "plain.ctx", PlainContext(6))
        .unwrap();
    let image = runtime
        .capture_checkpoint_v4(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    assert!(ENCODE_CALLS.load(Ordering::SeqCst) > 0);
    assert_eq!(
        image
            .contexts
            .iter()
            .map(|row| (row.codec_key.as_str(), row.version))
            .collect::<Vec<_>>(),
        vec![("plain.ctx", 2), ("zero.simple", 1)]
    );
    assert_eq!(image.context_stores.len(), 2);
    assert_eq!(OWNER_DECODE_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(PLAIN_DECODE_CALLS.load(Ordering::SeqCst), 0);
}

#[test]
fn context_codec_manifest_mismatch_rejects_before_decoder() {
    OWNER_DECODE_CALLS.store(0, Ordering::SeqCst);
    let mut runtime = FlowRuntime::new();
    let actor = runtime.spawn_actor().unwrap();
    runtime
        .register_work_handlers::<OwnerContext>("owner.ctx", WorkHandlers::default())
        .unwrap();
    let work = runtime
        .create_work(actor, SimDuration::ZERO, "owner.ctx", OwnerContext(77))
        .unwrap();
    let codecs_v2 = codecs_for_owner(2);
    let image_v2 = runtime
        .capture_checkpoint_v4(&codecs_v2, FlowCheckpointLimits::default())
        .unwrap();
    let (matching, _) = FlowRuntime::restore_checkpoint_v4_with_rebind(
        image_v2.clone(),
        &codecs_v2,
        FlowCheckpointLimits::default(),
    )
    .unwrap();
    assert_eq!(matching.work_context::<OwnerContext>(work).unwrap().0, 77);
    assert!(OWNER_DECODE_CALLS.load(Ordering::SeqCst) > 0);

    let mismatches = [
        {
            let mut image = image_v2.clone();
            image.contexts[0].version = 1;
            image
        },
        {
            let mut image = image_v2.clone();
            image.context_stores[0].version = 1;
            image
        },
    ];
    for image in mismatches {
        let original = image.clone();
        OWNER_DECODE_CALLS.store(0, Ordering::SeqCst);
        assert!(matches!(
            FlowRuntime::restore_checkpoint_v4(
                image.clone(),
                &codecs_v2,
                FlowCheckpointLimits::default()
            ),
            Err(FlowCheckpointError::IncompatibleRegistration(_))
        ));
        assert_eq!(OWNER_DECODE_CALLS.load(Ordering::SeqCst), 0);
        assert_eq!(image, original);
    }

    let codecs_v1 = codecs_for_owner(1);
    OWNER_DECODE_CALLS.store(0, Ordering::SeqCst);
    assert!(matches!(
        FlowRuntime::restore_checkpoint_v4(
            image_v2.clone(),
            &codecs_v1,
            FlowCheckpointLimits::default()
        ),
        Err(FlowCheckpointError::IncompatibleRegistration(_))
    ));
    assert_eq!(OWNER_DECODE_CALLS.load(Ordering::SeqCst), 0);

    let image_v1 = runtime
        .capture_checkpoint_v4(&codecs_v1, FlowCheckpointLimits::default())
        .unwrap();
    OWNER_DECODE_CALLS.store(0, Ordering::SeqCst);
    assert!(matches!(
        FlowRuntime::restore_checkpoint_v4(image_v1, &codecs_v2, FlowCheckpointLimits::default()),
        Err(FlowCheckpointError::IncompatibleRegistration(_))
    ));
    assert_eq!(OWNER_DECODE_CALLS.load(Ordering::SeqCst), 0);
}

#[test]
fn context_codec_legacy_wire_versions_preserve_schema_and_bytes() {
    PLAIN_DECODE_CALLS.store(0, Ordering::SeqCst);
    let mut codecs = FlowCheckpointCodecs::new();
    codecs
        .register_context::<PlainContext>("plain.ctx", 2, encode_plain, decode_plain)
        .unwrap();
    codecs
        .register_work_handlers::<PlainContext>(
            "plain.ctx",
            FlowHandlerCodeIds::default(),
            WorkHandlers::default(),
        )
        .unwrap();
    let mut runtime = FlowRuntime::new();
    let actor = runtime.spawn_actor().unwrap();
    runtime
        .register_work_handlers::<PlainContext>("plain.ctx", WorkHandlers::default())
        .unwrap();
    let work = runtime
        .create_work(actor, SimDuration::ZERO, "plain.ctx", PlainContext(1234))
        .unwrap();
    let limits = FlowCheckpointLimits::default();
    let wire_limits = kairo_ecs_des::FlowCheckpointWireLimits::default();

    let v1 = runtime.capture_checkpoint(&codecs, limits).unwrap();
    assert_eq!(context_versions(&v1), vec![2]);
    assert_eq!(v1.context_stores[0].version, 2);
    let b1 = v1.encode_wire_v1(wire_limits).unwrap();
    let d1 = FlowCheckpointV1::decode_wire_v1(&b1, wire_limits).unwrap();
    assert_eq!(d1, v1);
    assert_eq!(d1.encode_wire_v1(wire_limits).unwrap(), b1);
    let r1 = FlowRuntime::restore_checkpoint(d1, &codecs, limits).unwrap();
    assert_eq!(r1.work_context::<PlainContext>(work).unwrap().0, 1234);

    let v2 = runtime.capture_checkpoint_v2(&codecs, limits).unwrap();
    assert_eq!(context_versions_v2(&v2), vec![2]);
    assert_eq!(v2.context_stores[0].version, 2);
    let b2 = v2.encode_wire_v2(wire_limits).unwrap();
    let d2 = FlowCheckpointV2::decode_wire_v2(&b2, wire_limits).unwrap();
    assert_eq!(d2, v2);
    assert_eq!(d2.encode_wire_v2(wire_limits).unwrap(), b2);
    let r2 = FlowRuntime::restore_checkpoint_v2(d2, &codecs, limits).unwrap();
    assert_eq!(r2.work_context::<PlainContext>(work).unwrap().0, 1234);

    let v3 = runtime.capture_checkpoint_v3(&codecs, limits).unwrap();
    assert_eq!(context_versions_v3(&v3), vec![2]);
    assert_eq!(v3.context_stores[0].version, 2);
    let b3 = v3.encode_wire_v3(wire_limits).unwrap();
    let d3 = FlowCheckpointV3::decode_wire_v3(&b3, wire_limits).unwrap();
    assert_eq!(d3, v3);
    assert_eq!(d3.encode_wire_v3(wire_limits).unwrap(), b3);
    let r3 = FlowRuntime::restore_checkpoint_v3(d3, &codecs, limits).unwrap();
    assert_eq!(r3.work_context::<PlainContext>(work).unwrap().0, 1234);

    let v4 = runtime.capture_checkpoint_v4(&codecs, limits).unwrap();
    assert_eq!(context_versions_v4(&v4), vec![2]);
    assert_eq!(v4.context_stores[0].version, 2);
    let b4 = v4.encode_wire_v4(wire_limits).unwrap();
    let d4 = FlowCheckpointV4::decode_wire_v4(&b4, wire_limits).unwrap();
    assert_eq!(d4, v4);
    assert_eq!(d4.encode_wire_v4(wire_limits).unwrap(), b4);
    let r4 = FlowRuntime::restore_checkpoint_v4(d4, &codecs, limits).unwrap();
    assert_eq!(r4.work_context::<PlainContext>(work).unwrap().0, 1234);
    assert!(PLAIN_DECODE_CALLS.load(Ordering::SeqCst) >= 4);
}

#[test]
fn context_codec_restart_schema_guard_remains_v1() {
    fn encode_template(
        value: &u64,
        remaining: usize,
    ) -> Result<Vec<u8>, kairo_ecs_des::FlowCheckpointCodecError> {
        encode_u64(*value, remaining)
    }
    fn decode_template(
        bytes: &[u8],
        _: &kairo_ecs_des::FlowCheckpointRebindV1,
    ) -> Result<u64, kairo_ecs_des::FlowCheckpointCodecError> {
        decode_u64(bytes)
    }

    let mut codecs = FlowCheckpointCodecs::new();
    codecs
        .register_context::<PlainContext>("plain.ctx", 2, encode_plain, decode_plain)
        .unwrap();
    codecs
        .register_work_handlers::<PlainContext>(
            "plain.ctx",
            FlowHandlerCodeIds::default(),
            WorkHandlers::default(),
        )
        .unwrap();
    assert!(matches!(
        codecs.register_restart_template::<u64, PlainContext>("restart.zero", 0, encode_template, decode_template),
        Err(FlowCheckpointError::IncompatibleRegistration(key)) if key == "restart.zero"
    ));
    assert!(matches!(
        codecs.register_restart_template::<u64, PlainContext>("restart.two", 2, encode_template, decode_template),
        Err(FlowCheckpointError::IncompatibleRegistration(key)) if key == "restart.two"
    ));
    codecs
        .register_restart_template::<u64, PlainContext>(
            "restart.one",
            1,
            encode_template,
            decode_template,
        )
        .unwrap();

    let mut runtime = FlowRuntime::new();
    let actor = runtime.spawn_actor().unwrap();
    runtime
        .register_work_handlers::<PlainContext>("plain.ctx", WorkHandlers::default())
        .unwrap();
    runtime
        .create_work(actor, SimDuration::ZERO, "plain.ctx", PlainContext(9))
        .unwrap();
    let image = runtime
        .capture_checkpoint_v4(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    assert_eq!(context_versions_v4(&image), vec![2]);
}
