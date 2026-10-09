use kairo_ecs_des::{
    FlowCallbackCodeV1, FlowCallbackSnapshot, FlowCheckpointCodecs, FlowCheckpointError,
    FlowCheckpointLimits, FlowCheckpointRebindV1, FlowCommandSink, FlowConfig,
    FlowContinuationCodeIds, FlowContinuations, FlowRuntime, FlowWorldView, PreemptionStrategy,
    RequestState, WorkState,
};
use kairo_ecs_types::{EventKind, SimDuration, SimTime};
use std::cell::{Cell, RefCell};
use std::num::NonZeroU64;

#[test]
fn owned_codec_environments_receive_dense_owner_and_return_same_validated_view() {
    let mut source = FlowRuntime::new();
    let owner = source.spawn_actor().unwrap();
    let other_owner = source.spawn_actor().unwrap();
    let work = source
        .create_restartable_work(
            owner,
            SimDuration::from_ticks(20),
            "service",
            Template { initial: 7 },
            template_context,
        )
        .unwrap();
    let offset = std::sync::Arc::new(19_u64);
    let mut codecs = FlowCheckpointCodecs::new();
    let encode_offset = offset.clone();
    let decode_offset = offset.clone();
    codecs
        .register_context_with_owner::<Context>(
            "context.v1",
            1,
            move |context, remaining| {
                if remaining < 8 {
                    return Err(kairo_ecs_des::FlowCheckpointCodecError("budget".into()));
                }
                Ok((context.value + *encode_offset).to_le_bytes().to_vec())
            },
            move |bytes, row, view| {
                assert_eq!(view.resolve_work(row).unwrap(), work);
                assert_eq!(view.resolve_work_owner(row).unwrap(), owner);
                assert_eq!(
                    view.resolve_work_binding(row).unwrap(),
                    (owner, "service", None)
                );
                let bytes: [u8; 8] = bytes
                    .try_into()
                    .map_err(|_| kairo_ecs_des::FlowCheckpointCodecError("payload".into()))?;
                Ok(Context {
                    value: u64::from_le_bytes(bytes) - *decode_offset,
                })
            },
        )
        .unwrap();
    let encode_offset = offset.clone();
    codecs
        .register_restart_template_with_owner::<Template, Context>(
            "template.v1",
            1,
            move |template, remaining| {
                if remaining < 8 {
                    return Err(kairo_ecs_des::FlowCheckpointCodecError("budget".into()));
                }
                Ok((template.initial + *encode_offset).to_le_bytes().to_vec())
            },
            move |bytes, row, view| {
                assert_eq!(view.resolve_work(row).unwrap(), work);
                let bytes: [u8; 8] = bytes
                    .try_into()
                    .map_err(|_| kairo_ecs_des::FlowCheckpointCodecError("payload".into()))?;
                Ok(Template {
                    initial: u64::from_le_bytes(bytes) - *offset,
                })
            },
        )
        .unwrap();
    codecs
        .register_restart_factory::<Template, Context>(
            "template.v1",
            "factory.v1",
            template_context,
        )
        .unwrap();
    let resource = source.create_resource(1).unwrap();
    source
        .acquire(resource)
        .owner(owner)
        .timed_work(work)
        .submit()
        .unwrap();
    let image = source
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    let mut mismatched_owner = image.clone();
    mismatched_owner.builtins.work_specs.as_mut().unwrap().rows[0]
        .1
        .owner = other_owner;
    assert!(matches!(
        FlowRuntime::restore_checkpoint_with_rebind(
            mismatched_owner,
            &codecs,
            FlowCheckpointLimits::default()
        ),
        Err(FlowCheckpointError::InvalidState(_))
    ));
    let (restored, view) = FlowRuntime::restore_checkpoint_with_rebind(
        image,
        &codecs,
        FlowCheckpointLimits::default(),
    )
    .unwrap();
    assert_ne!(restored.identity(), source.identity());
    assert_eq!(*view.identity(), restored.identity());
    assert_eq!(view.resolve_work(work.entity_id()).unwrap(), work);
    assert_eq!(restored.work_context::<Context>(work).unwrap().value, 7);
    assert!(view
        .resolve_work_owner(kairo_ecs_types::EntityId::new(u64::MAX, 0))
        .is_err());
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Context {
    value: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Template {
    initial: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RebindPayload {
    work: kairo_ecs_des::WorkId,
    resource: kairo_ecs_des::ResourceId,
    request: kairo_ecs_des::RequestId,
    actor: kairo_ecs_types::EntityId,
    event: kairo_ecs_types::EventId,
    identity: kairo_ecs_des::FlowRuntimeIdentity,
}

fn template_context(template: &Template) -> Context {
    Context {
        value: template.initial,
    }
}

fn encode_template(
    value: &Template,
    remaining: usize,
) -> Result<Vec<u8>, kairo_ecs_des::FlowCheckpointCodecError> {
    if remaining < 8 {
        return Err(kairo_ecs_des::FlowCheckpointCodecError("too small".into()));
    }
    Ok(value.initial.to_le_bytes().to_vec())
}

fn decode_template(
    bytes: &[u8],
    _: &FlowCheckpointRebindV1,
) -> Result<Template, kairo_ecs_des::FlowCheckpointCodecError> {
    let bytes: [u8; 8] = bytes
        .try_into()
        .map_err(|_| kairo_ecs_des::FlowCheckpointCodecError("invalid template payload".into()))?;
    Ok(Template {
        initial: u64::from_le_bytes(bytes),
    })
}

fn encode_context(
    value: &Context,
    remaining: usize,
) -> Result<Vec<u8>, kairo_ecs_des::FlowCheckpointCodecError> {
    if remaining < 8 {
        return Err(kairo_ecs_des::FlowCheckpointCodecError("too small".into()));
    }
    Ok(value.value.to_le_bytes().to_vec())
}

thread_local! { static DECODE_CALLS: Cell<usize> = const { Cell::new(0) }; }
thread_local! { static LAST_DECODE_IDENTITY: RefCell<Option<kairo_ecs_des::FlowRuntimeIdentity>> = const { RefCell::new(None) }; }
thread_local! { static REBIND_IDENTITIES: RefCell<Vec<kairo_ecs_des::FlowRuntimeIdentity>> = const { RefCell::new(Vec::new()) }; }
fn decode_context(
    bytes: &[u8],
    rebind: &FlowCheckpointRebindV1,
) -> Result<Context, kairo_ecs_des::FlowCheckpointCodecError> {
    DECODE_CALLS.with(|count| count.set(count.get() + 1));
    LAST_DECODE_IDENTITY.with(|identity| {
        *identity.borrow_mut() = Some(rebind.identity().clone());
    });
    let bytes: [u8; 8] = bytes
        .try_into()
        .map_err(|_| kairo_ecs_des::FlowCheckpointCodecError("invalid context payload".into()))?;
    Ok(Context {
        value: u64::from_le_bytes(bytes),
    })
}

fn encode_unit(_: &(), _: usize) -> Result<Vec<u8>, kairo_ecs_des::FlowCheckpointCodecError> {
    Ok(Vec::new())
}

fn decode_unit(
    _: &[u8],
    _: &FlowCheckpointRebindV1,
) -> Result<(), kairo_ecs_des::FlowCheckpointCodecError> {
    Ok(())
}

fn encode_rebind_payload(
    value: &RebindPayload,
    remaining: usize,
) -> Result<Vec<u8>, kairo_ecs_des::FlowCheckpointCodecError> {
    if remaining < 60 {
        return Err(kairo_ecs_des::FlowCheckpointCodecError("too small".into()));
    }
    let mut bytes = Vec::with_capacity(60);
    for id in [
        value.work.entity_id(),
        value.resource.entity_id(),
        value.request.entity_id(),
        value.actor,
    ] {
        bytes.extend_from_slice(&id.index.to_le_bytes());
        bytes.extend_from_slice(&id.generation.to_le_bytes());
    }
    bytes.extend_from_slice(&value.event.index.to_le_bytes());
    bytes.extend_from_slice(&value.event.generation.to_le_bytes());
    Ok(bytes)
}

fn decode_rebind_payload(
    bytes: &[u8],
    rebind: &FlowCheckpointRebindV1,
) -> Result<RebindPayload, kairo_ecs_des::FlowCheckpointCodecError> {
    if bytes.len() != 60 {
        return Err(kairo_ecs_des::FlowCheckpointCodecError(
            "invalid payload".into(),
        ));
    }
    let entity = |offset: usize| {
        let index = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
        let generation = u32::from_le_bytes(bytes[offset + 8..offset + 12].try_into().unwrap());
        kairo_ecs_types::EntityId::new(index, generation)
    };
    let event = kairo_ecs_types::EventId::new(
        u64::from_le_bytes(bytes[48..56].try_into().unwrap()),
        u32::from_le_bytes(bytes[56..60].try_into().unwrap()),
    );
    let payload = RebindPayload {
        work: rebind.resolve_work(entity(0))?,
        resource: rebind.resolve_resource(entity(12))?,
        request: rebind.resolve_request(entity(24))?,
        actor: rebind.resolve_actor(entity(36))?,
        event: rebind.resolve_event(event)?,
        identity: rebind.identity().clone(),
    };
    REBIND_IDENTITIES.with(|identities| identities.borrow_mut().push(payload.identity.clone()));
    Ok(payload)
}

fn copy_rebind_payload(value: &RebindPayload) -> RebindPayload {
    value.clone()
}

fn noop_rebind_callback(
    _: &mut RebindPayload,
    _: &FlowCallbackSnapshot,
    _: FlowWorldView<'_>,
    _: &mut FlowCommandSink,
) {
}

fn bind_rebind_event(value: &mut RebindPayload, event: kairo_ecs_types::EventId) {
    value.event = event;
}

fn mutate_context(
    context: &mut Context,
    _: &FlowCallbackSnapshot,
    _: FlowWorldView<'_>,
    _: &mut FlowCommandSink,
) {
    context.value += 11;
}

fn different_callback(
    context: &mut Context,
    _: &FlowCallbackSnapshot,
    _: FlowWorldView<'_>,
    _: &mut FlowCommandSink,
) {
    context.value += 12;
}

fn resume_context(context: &mut Context, _: &FlowCallbackSnapshot, _: &mut FlowCommandSink) {
    context.value += 50;
}

fn restart_context(context: &mut Context, _: &FlowCallbackSnapshot, _: &mut FlowCommandSink) {
    context.value += 100;
}

fn register_codec(
    callback: for<'a> fn(
        &'a mut Context,
        &'a FlowCallbackSnapshot,
        FlowWorldView<'a>,
        &'a mut FlowCommandSink,
    ),
) -> FlowCheckpointCodecs {
    register_codec_with_callback_id(callback, "model.context.callback", 1)
}

fn register_codec_with_callback_id(
    callback: for<'a> fn(
        &'a mut Context,
        &'a FlowCallbackSnapshot,
        FlowWorldView<'a>,
        &'a mut FlowCommandSink,
    ),
    stable_id: &str,
    version: u32,
) -> FlowCheckpointCodecs {
    let mut codecs = FlowCheckpointCodecs::new();
    codecs
        .register_context("context.v1", 1, encode_context, decode_context)
        .unwrap();
    codecs
        .register_domain_view_hook(
            "model.ctx",
            EventKind::custom(91),
            callback,
            FlowCallbackCodeV1 {
                stable_id: stable_id.into(),
                version,
            },
        )
        .unwrap();
    codecs
}

#[test]
fn context_and_template_decoders_rebind_only_validated_ids_to_fresh_runtime() {
    let kind = EventKind::custom(92);
    let mut codecs = FlowCheckpointCodecs::new();
    codecs
        .register_context(
            "payload.v1",
            1,
            encode_rebind_payload,
            decode_rebind_payload,
        )
        .unwrap();
    codecs
        .register_context("unit.v1", 1, encode_unit, decode_unit)
        .unwrap();
    codecs
        .register_restart_template::<RebindPayload, RebindPayload>(
            "payload.template.v1",
            1,
            encode_rebind_payload,
            decode_rebind_payload,
        )
        .unwrap();
    codecs
        .register_restart_factory(
            "payload.template.v1",
            "payload.copy.v1",
            copy_rebind_payload,
        )
        .unwrap();
    codecs
        .register_domain_view_hook(
            "rebind",
            kind,
            noop_rebind_callback,
            FlowCallbackCodeV1 {
                stable_id: "rebind.noop".into(),
                version: 1,
            },
        )
        .unwrap();

    let mut source = FlowRuntime::new();
    source
        .register_domain_view_hook("rebind", kind, noop_rebind_callback)
        .unwrap();
    let owner = source.spawn_actor().unwrap();
    let resource = source.create_resource(0).unwrap();
    let request = source.acquire(resource).owner(owner).submit().unwrap();
    source.step().unwrap();
    source.cancel(request, SimTime::from_ticks(1)).unwrap();
    source.step().unwrap();
    assert_eq!(
        source.request(request).unwrap().state,
        RequestState::Cancelled
    );
    let historical_resource = source.create_resource(0).unwrap();
    source.remove_resource(historical_resource).unwrap();
    source.step().unwrap();
    let anchor = source
        .create_work(owner, SimDuration::from_ticks(100), "anchor", ())
        .unwrap();
    let mut payload = RebindPayload {
        work: anchor,
        resource: historical_resource,
        request,
        actor: owner,
        event: kairo_ecs_types::EventId::new(u64::MAX, u32::MAX),
        identity: source.identity(),
    };
    let carrier = source
        .create_actor_domain_context(owner, "rebind", kind, payload.clone())
        .unwrap();
    let event = source
        .schedule_domain_and_bind(carrier, kind, SimTime::from_ticks(12), 0, bind_rebind_event)
        .unwrap();
    payload.event = event;
    let restartable = source
        .create_restartable_work(
            owner,
            SimDuration::from_ticks(20),
            "rebind",
            payload,
            copy_rebind_payload,
        )
        .unwrap();
    let mut image = source
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    image
        .builtins
        .requests
        .as_mut()
        .unwrap()
        .rows
        .iter_mut()
        .find(|(entity, _)| *entity == request.entity_id())
        .unwrap()
        .1
        .resource = historical_resource;

    REBIND_IDENTITIES.with(|identities| identities.borrow_mut().clear());
    let restored =
        FlowRuntime::restore_checkpoint(image.clone(), &codecs, FlowCheckpointLimits::default())
            .unwrap();
    assert_ne!(restored.identity(), source.identity());
    for work in [carrier, restartable] {
        let value = restored.work_context::<RebindPayload>(work).unwrap();
        assert_eq!(value.work, anchor);
        assert_eq!(value.resource, historical_resource);
        assert_eq!(value.request, request);
        assert_eq!(value.actor, owner);
        assert_eq!(value.event, event);
        assert_eq!(value.identity, restored.identity());
    }
    REBIND_IDENTITIES.with(|identities| {
        let identities = identities.borrow();
        assert_eq!(identities.len(), 3);
        assert!(identities
            .iter()
            .all(|identity| identity == &restored.identity()));
    });

    let mut bad_reference = image;
    let store = bad_reference
        .context_stores
        .iter_mut()
        .find(|store| store.codec_key == "payload.v1")
        .unwrap();
    store.rows[0].1[11] = store.rows[0].1[11].wrapping_add(1);
    REBIND_IDENTITIES.with(|identities| identities.borrow_mut().clear());
    assert!(matches!(
        FlowRuntime::restore_checkpoint(bad_reference, &codecs, FlowCheckpointLimits::default()),
        Err(FlowCheckpointError::Codec { .. })
    ));
    REBIND_IDENTITIES.with(|identities| assert!(identities.borrow().is_empty()));
}

#[test]
fn image_roundtrips_mutated_context_and_preserves_work_identity_with_fresh_runtime_identity() {
    let codecs = register_codec(mutate_context);
    let mut source = FlowRuntime::new();
    source
        .register_domain_view_hook("model.ctx", EventKind::custom(91), mutate_context)
        .unwrap();
    let actor = source.spawn_actor().unwrap();
    let work = source
        .create_actor_domain_context(
            actor,
            "model.ctx",
            EventKind::custom(91),
            Context { value: 4 },
        )
        .unwrap();
    source
        .schedule_domain(work, EventKind::custom(91), SimTime::from_ticks(1), 0)
        .unwrap();
    source.run_for(1).unwrap();
    assert_eq!(source.work_context::<Context>(work).unwrap().value, 15);

    let image = source
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    let restored =
        FlowRuntime::restore_checkpoint(image, &codecs, FlowCheckpointLimits::default()).unwrap();
    assert_ne!(source.identity(), restored.identity());
    LAST_DECODE_IDENTITY.with(|identity| {
        assert_eq!(identity.borrow().as_ref(), Some(&restored.identity()));
    });
    assert_eq!(restored.work(work).unwrap(), source.work(work).unwrap());
    assert_eq!(restored.work_context::<Context>(work).unwrap().value, 15);
}

#[test]
fn budget_halts_at_first_and_later_tick_roundtrip_with_source_counters() {
    let codecs = register_codec(mutate_context);
    let kind = EventKind::custom(91);
    let config = FlowConfig {
        max_same_tick_flow_transitions: NonZeroU64::new(1).unwrap(),
    };

    let mut first_tick = FlowRuntime::with_config(config);
    first_tick
        .register_domain_view_hook("model.ctx", kind, mutate_context)
        .unwrap();
    let actor = first_tick.spawn_actor().unwrap();
    let resource = first_tick.create_resource(1).unwrap();
    let work = first_tick
        .create_work(
            actor,
            SimDuration::from_ticks(10),
            "model.ctx",
            Context { value: 3 },
        )
        .unwrap();
    first_tick
        .acquire(resource)
        .owner(actor)
        .timed_work(work)
        .submit()
        .unwrap();
    assert!(matches!(
        first_tick.step(),
        Err(kairo_ecs_des::FlowError::SameTickBudgetExceeded { at_ticks: 0, .. })
    ));
    assert_eq!(first_tick.budget_snapshot().tick, None);
    assert_eq!(first_tick.budget_snapshot().consumed, 0);
    let image = first_tick
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    let restored = FlowRuntime::restore_checkpoint(image, &codecs, FlowCheckpointLimits::default())
        .expect("first-dispatch budget halt is a valid checkpoint");
    assert_eq!(restored.budget_snapshot(), first_tick.budget_snapshot());

    let mut later_tick = FlowRuntime::with_config(config);
    later_tick
        .register_domain_view_hook("model.ctx", kind, mutate_context)
        .unwrap();
    let actor = later_tick.spawn_actor().unwrap();
    let carrier = later_tick
        .create_actor_domain_context(actor, "model.ctx", kind, Context { value: 4 })
        .unwrap();
    later_tick
        .schedule_domain(carrier, kind, SimTime::from_ticks(0), 0)
        .unwrap();
    later_tick
        .schedule_domain(carrier, kind, SimTime::from_ticks(1), 100)
        .unwrap();
    let resource = later_tick.create_resource(1).unwrap();
    let work = later_tick
        .create_work(
            actor,
            SimDuration::from_ticks(10),
            "model.ctx",
            Context { value: 5 },
        )
        .unwrap();
    later_tick
        .acquire(resource)
        .owner(actor)
        .timed_work(work)
        .at(SimTime::from_ticks(1))
        .submit()
        .unwrap();
    later_tick.step().unwrap();
    assert_eq!(
        later_tick.budget_snapshot().tick,
        Some(SimTime::from_ticks(0))
    );
    assert_eq!(later_tick.budget_snapshot().consumed, 1);
    assert!(matches!(
        later_tick.step(),
        Err(kairo_ecs_des::FlowError::SameTickBudgetExceeded { at_ticks: 1, .. })
    ));
    assert_eq!(later_tick.budget_snapshot().consumed, 1);
    assert_eq!(later_tick.budget_snapshot().halted.unwrap().consumed, 0);
    let image = later_tick
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    let mut wrong_pending = image.clone();
    let non_head = wrong_pending
        .scheduler
        .entries
        .iter()
        .find(|entry| {
            entry.live
                && entry.request.at == SimTime::from_ticks(1)
                && entry.request.priority == 100
        })
        .unwrap();
    let halt = wrong_pending.budget_halt.as_mut().unwrap();
    halt.pending.id = non_head.id;
    halt.pending.at = non_head.request.at;
    halt.pending.priority = non_head.request.priority;
    halt.pending.sequence = non_head.sequence;
    halt.pending.entity = non_head.request.entity;
    halt.pending.kind = non_head.request.kind;
    assert!(matches!(
        FlowRuntime::restore_checkpoint(wrong_pending, &codecs, FlowCheckpointLimits::default()),
        Err(FlowCheckpointError::InvalidState(
            "invalid Flow counters or budget state"
        ))
    ));
    let restored = FlowRuntime::restore_checkpoint(image, &codecs, FlowCheckpointLimits::default())
        .expect("later-tick budget halt is a valid checkpoint");
    assert_eq!(restored.budget_snapshot(), later_tick.budget_snapshot());
}

#[test]
fn restart_template_and_queued_suffix_survive_separately_from_mutated_context() {
    let mut codecs = register_codec(mutate_context);
    codecs
        .register_restart_template::<Template, Context>(
            "template.v1",
            1,
            encode_template,
            decode_template,
        )
        .unwrap();
    codecs
        .register_restart_factory("template.v1", "template.factory.v1", template_context)
        .unwrap();

    let mut source = FlowRuntime::new();
    source
        .register_domain_view_hook("model.ctx", EventKind::custom(91), mutate_context)
        .unwrap();
    let actor = source.spawn_actor().unwrap();
    let work = source
        .create_restartable_work(
            actor,
            kairo_ecs_types::SimDuration::from_ticks(20),
            "model.ctx",
            Template { initial: 10 },
            template_context,
        )
        .unwrap();
    source
        .schedule_domain(work, EventKind::custom(91), SimTime::from_ticks(1), 0)
        .unwrap();
    source.run_for(1).unwrap();
    assert_eq!(source.work_context::<Context>(work).unwrap().value, 21);
    source
        .schedule_domain(work, EventKind::custom(91), SimTime::from_ticks(2), 0)
        .unwrap();

    let image = source
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    let mut restored =
        FlowRuntime::restore_checkpoint(image, &codecs, FlowCheckpointLimits::default()).unwrap();
    assert_eq!(restored.work_context::<Context>(work).unwrap().value, 21);
    source.run_for(1).unwrap();
    restored.run_for(1).unwrap();
    assert_eq!(
        source.work_context::<Context>(work).unwrap(),
        restored.work_context::<Context>(work).unwrap()
    );
    assert_eq!(source.work_context::<Context>(work).unwrap().value, 32);
}

#[test]
fn registered_empty_restart_store_roundtrips_and_missing_manifest_fails_before_decode() {
    let mut codecs = register_codec(mutate_context);
    codecs
        .register_restart_template::<Template, Context>(
            "template.v1",
            1,
            encode_template,
            decode_template,
        )
        .unwrap();
    codecs
        .register_restart_factory("template.v1", "template.factory.v1", template_context)
        .unwrap();
    let mut source = FlowRuntime::new();
    source
        .register_domain_view_hook("model.ctx", EventKind::custom(91), mutate_context)
        .unwrap();
    let actor = source.spawn_actor().unwrap();
    let resource = source.create_resource(1).unwrap();
    let work = source
        .create_restartable_work(
            actor,
            SimDuration::from_ticks(1),
            "model.ctx",
            Template { initial: 8 },
            template_context,
        )
        .unwrap();
    source
        .acquire(resource)
        .owner(actor)
        .timed_work(work)
        .submit()
        .unwrap();
    source.run_for(3).unwrap();
    source
        .despawn_actor_at_with_scheduler_priority(actor, source.now(), 0)
        .unwrap();
    source.step().unwrap();
    let image = source
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    let restart = image
        .restart_stores
        .iter()
        .find(|store| store.codec_key == "template.v1")
        .unwrap();
    assert!(restart.rows.is_empty());
    FlowRuntime::restore_checkpoint(image.clone(), &codecs, FlowCheckpointLimits::default())
        .unwrap();

    let long_codec_key = "t".repeat(6_000);
    let mut long_key_codecs = register_codec(mutate_context);
    long_key_codecs
        .register_restart_template::<Template, Context>(
            long_codec_key.clone(),
            1,
            encode_template,
            decode_template,
        )
        .unwrap();
    long_key_codecs
        .register_restart_factory(&long_codec_key, "f", template_context)
        .unwrap();
    assert!(matches!(
        source.capture_checkpoint(
            &long_key_codecs,
            FlowCheckpointLimits {
                max_key_bytes: 10_000,
                ..FlowCheckpointLimits::default()
            }
        ),
        Err(FlowCheckpointError::LimitExceeded("key bytes"))
    ));

    let mut oversized_manifest = image.clone();
    oversized_manifest.restart_store_manifest[0].0 = "m".repeat(1_024);
    DECODE_CALLS.with(|count| count.set(0));
    assert!(matches!(
        FlowRuntime::restore_checkpoint(
            oversized_manifest,
            &codecs,
            FlowCheckpointLimits {
                max_key_bytes: 1_024,
                ..FlowCheckpointLimits::default()
            }
        ),
        Err(FlowCheckpointError::LimitExceeded("key bytes"))
    ));
    DECODE_CALLS.with(|count| assert_eq!(count.get(), 0));

    let mut missing = image;
    missing.restart_stores.clear();
    DECODE_CALLS.with(|count| count.set(0));
    assert!(matches!(
        FlowRuntime::restore_checkpoint(missing, &codecs, FlowCheckpointLimits::default()),
        Err(FlowCheckpointError::InvalidState(
            "restart store manifest is incomplete"
        ))
    ));
    DECODE_CALLS.with(|count| assert_eq!(count.get(), 0));
}

#[test]
fn many_work_restore_preserves_registration_and_context_mapping() {
    let codecs = register_codec(mutate_context);
    let mut source = FlowRuntime::new();
    source
        .register_domain_view_hook("model.ctx", EventKind::custom(91), mutate_context)
        .unwrap();
    let actor = source.spawn_actor().unwrap();
    let works: Vec<_> = (0..512)
        .map(|value| {
            source
                .create_work(
                    actor,
                    SimDuration::from_ticks(10),
                    "model.ctx",
                    Context { value },
                )
                .unwrap()
        })
        .collect();
    let image = source
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    let restored =
        FlowRuntime::restore_checkpoint(image, &codecs, FlowCheckpointLimits::default()).unwrap();
    for (value, work) in works.into_iter().enumerate() {
        assert_eq!(
            restored.work_context::<Context>(work).unwrap().value,
            value as u64
        );
        assert_eq!(restored.work(work).unwrap(), source.work(work).unwrap());
    }
}

#[test]
fn stale_release_reservation_survives_preemption_and_suffix_matches_source() {
    let codecs = register_codec(mutate_context);
    let mut source = FlowRuntime::new();
    source
        .register_domain_view_hook("model.ctx", EventKind::custom(91), mutate_context)
        .unwrap();
    let actor = source.spawn_actor().unwrap();
    let resource = source.create_resource(1).unwrap();
    let low = source
        .create_work(
            actor,
            SimDuration::from_ticks(20),
            "model.ctx",
            Context { value: 1 },
        )
        .unwrap();
    let low_request = source
        .acquire(resource)
        .owner(actor)
        .timed_work(low)
        .priority(9)
        .preemptible(PreemptionStrategy::Suspend)
        .submit()
        .unwrap();
    source.step().unwrap();
    let stale_lease = source.request(low_request).unwrap().lease.unwrap();
    source
        .release(stale_lease, SimTime::from_ticks(10))
        .unwrap();
    let active_release = source
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    let mut missing_active_reservation = active_release;
    missing_active_reservation.pending_releases.clear();
    DECODE_CALLS.with(|count| count.set(0));
    assert!(matches!(
        FlowRuntime::restore_checkpoint(
            missing_active_reservation,
            &codecs,
            FlowCheckpointLimits::default()
        ),
        Err(FlowCheckpointError::InvalidState(
            "pending operation reservation mismatch"
        ))
    ));
    DECODE_CALLS.with(|count| assert_eq!(count.get(), 0));

    let urgent = source
        .create_work(
            actor,
            SimDuration::from_ticks(2),
            "model.ctx",
            Context { value: 2 },
        )
        .unwrap();
    let high_request = source
        .acquire(resource)
        .owner(actor)
        .timed_work(urgent)
        .priority(1)
        .can_preempt(true)
        .at(SimTime::from_ticks(5))
        .submit()
        .unwrap();
    source.step().unwrap();
    assert_eq!(
        source.request(low_request).unwrap().state,
        RequestState::Suspended
    );
    let image = source
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    assert!(image.pending_releases.contains(&stale_lease));
    let mut restored =
        FlowRuntime::restore_checkpoint(image.clone(), &codecs, FlowCheckpointLimits::default())
            .unwrap();
    assert_eq!(source.run_for(10).unwrap(), restored.run_for(10).unwrap());
    assert_eq!(
        source.request(high_request).unwrap(),
        restored.request(high_request).unwrap()
    );
    assert_eq!(
        source.resource(resource).unwrap(),
        restored.resource(resource).unwrap()
    );
}

#[test]
fn actor_cleanup_can_clear_release_reservation_while_stale_command_remains() {
    let mut source = FlowRuntime::new();
    let actor = source.spawn_actor().unwrap();
    let resource = source.create_resource(1).unwrap();
    let request = source.acquire(resource).owner(actor).submit().unwrap();
    source.step().unwrap();
    let lease = source.request(request).unwrap().lease.unwrap();
    source.release(lease, SimTime::from_ticks(5)).unwrap();
    source
        .despawn_actor_at_with_scheduler_priority(actor, SimTime::from_ticks(2), 0)
        .unwrap();
    source.step().unwrap();

    let codecs = FlowCheckpointCodecs::new();
    let image = source
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    assert!(image.pending_releases.is_empty());
    assert!(image.commands.iter().any(|(_, command)| matches!(
        command,
        kairo_ecs_des::FlowCommandV1::Release(stale) if *stale == lease
    )));
    let mut restored =
        FlowRuntime::restore_checkpoint(image, &codecs, FlowCheckpointLimits::default()).unwrap();
    assert_eq!(source.run_for(8).unwrap(), restored.run_for(8).unwrap());
    assert_eq!(
        source.request(request).unwrap(),
        restored.request(request).unwrap()
    );
}

#[test]
fn local_capture_rejects_same_registration_name_with_different_callback_function() {
    let codecs = register_codec(different_callback);
    let mut source = FlowRuntime::new();
    source
        .register_domain_view_hook("model.ctx", EventKind::custom(91), mutate_context)
        .unwrap();
    let actor = source.spawn_actor().unwrap();
    source
        .create_actor_domain_context(
            actor,
            "model.ctx",
            EventKind::custom(91),
            Context { value: 4 },
        )
        .unwrap();
    assert!(matches!(
        source.capture_checkpoint(&codecs, FlowCheckpointLimits::default()),
        Err(FlowCheckpointError::IncompatibleRegistration(_))
    ));
}

#[test]
fn callback_id_mismatch_is_rejected_before_any_context_decoder_runs() {
    let codecs = register_codec(mutate_context);
    let mut source = FlowRuntime::new();
    source
        .register_domain_view_hook("model.ctx", EventKind::custom(91), mutate_context)
        .unwrap();
    let actor = source.spawn_actor().unwrap();
    source
        .create_actor_domain_context(
            actor,
            "model.ctx",
            EventKind::custom(91),
            Context { value: 4 },
        )
        .unwrap();
    let mut image = source
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    image.domains[0].callback_ids[0].1.stable_id = "other.model.code".into();
    DECODE_CALLS.with(|count| count.set(0));
    assert!(matches!(
        FlowRuntime::restore_checkpoint(image, &codecs, FlowCheckpointLimits::default()),
        Err(FlowCheckpointError::IncompatibleRegistration(_))
    ));
    DECODE_CALLS.with(|count| assert_eq!(count.get(), 0));
}

#[test]
fn alternate_callback_registration_id_or_version_rejects_before_decoder() {
    let codecs = register_codec(mutate_context);
    let alternate = register_codec_with_callback_id(mutate_context, "model.context.callback", 2);
    let mut source = FlowRuntime::new();
    source
        .register_domain_view_hook("model.ctx", EventKind::custom(91), mutate_context)
        .unwrap();
    let actor = source.spawn_actor().unwrap();
    source
        .create_actor_domain_context(
            actor,
            "model.ctx",
            EventKind::custom(91),
            Context { value: 4 },
        )
        .unwrap();
    let image = source
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    DECODE_CALLS.with(|count| count.set(0));
    assert!(matches!(
        FlowRuntime::restore_checkpoint(image, &alternate, FlowCheckpointLimits::default()),
        Err(FlowCheckpointError::IncompatibleRegistration(_))
    ));
    DECODE_CALLS.with(|count| assert_eq!(count.get(), 0));
}

#[test]
fn duplicate_context_row_is_rejected_before_any_context_decoder_runs() {
    let codecs = register_codec(mutate_context);
    let mut source = FlowRuntime::new();
    source
        .register_domain_view_hook("model.ctx", EventKind::custom(91), mutate_context)
        .unwrap();
    let actor = source.spawn_actor().unwrap();
    source
        .create_actor_domain_context(
            actor,
            "model.ctx",
            EventKind::custom(91),
            Context { value: 4 },
        )
        .unwrap();
    let mut image = source
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    let duplicate = image.context_stores[0].rows[0].clone();
    image.context_stores[0].rows.push(duplicate);
    DECODE_CALLS.with(|count| count.set(0));
    assert!(matches!(
        FlowRuntime::restore_checkpoint(image, &codecs, FlowCheckpointLimits::default()),
        Err(FlowCheckpointError::InvalidState(_))
    ));
    DECODE_CALLS.with(|count| assert_eq!(count.get(), 0));
}

#[test]
fn queued_deadline_and_pending_scheduler_suffix_continue_identically() {
    let codecs = FlowCheckpointCodecs::new();
    let mut source = FlowRuntime::new();
    let actor = source.spawn_actor().unwrap();
    let resource = source.create_resource(0).unwrap();
    let request = source
        .acquire(resource)
        .owner(actor)
        .deadline(SimTime::from_ticks(5))
        .submit()
        .unwrap();
    source.step().unwrap();
    assert_eq!(source.request(request).unwrap().state, RequestState::Queued);

    let image = source
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    let mut restored =
        FlowRuntime::restore_checkpoint(image, &codecs, FlowCheckpointLimits::default()).unwrap();
    assert_eq!(
        restored.request(request).unwrap(),
        source.request(request).unwrap()
    );
    source.run_for(16).unwrap();
    restored.run_for(16).unwrap();
    assert_eq!(
        restored.request(request).unwrap(),
        source.request(request).unwrap()
    );
}

#[test]
fn schema_codec_key_index_and_payload_limits_fail_before_decoder() {
    let codecs = register_codec(mutate_context);
    let mut source = FlowRuntime::new();
    source
        .register_domain_view_hook("model.ctx", EventKind::custom(91), mutate_context)
        .unwrap();
    let actor = source.spawn_actor().unwrap();
    source
        .create_actor_domain_context(
            actor,
            "model.ctx",
            EventKind::custom(91),
            Context { value: 17 },
        )
        .unwrap();
    let base = source
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap();

    let mut bad_schema = base.clone();
    bad_schema.context_stores[0].version = 2;
    let mut bad_key = base.clone();
    bad_key.context_stores[0].codec_key = "unknown.codec".into();
    let mut bad_index = base.clone();
    bad_index.context_stores[0].sparse_slots = 0;
    let mut too_many_actor_domains = base.clone();
    let duplicate_domain = too_many_actor_domains.actor_domains[0];
    too_many_actor_domains.actor_domains.push(duplicate_domain);

    for image in [bad_schema, bad_key, bad_index] {
        DECODE_CALLS.with(|count| count.set(0));
        assert!(
            FlowRuntime::restore_checkpoint(image, &codecs, FlowCheckpointLimits::default())
                .is_err()
        );
        DECODE_CALLS.with(|count| assert_eq!(count.get(), 0));
    }
    DECODE_CALLS.with(|count| count.set(0));
    assert!(matches!(
        FlowRuntime::restore_checkpoint(
            too_many_actor_domains,
            &codecs,
            FlowCheckpointLimits {
                max_actors: 1,
                ..FlowCheckpointLimits::default()
            }
        ),
        Err(FlowCheckpointError::LimitExceeded("actor domains"))
    ));
    DECODE_CALLS.with(|count| assert_eq!(count.get(), 0));
    DECODE_CALLS.with(|count| count.set(0));
    let limits = FlowCheckpointLimits {
        max_payload_bytes: 7,
        ..FlowCheckpointLimits::default()
    };
    assert!(matches!(
        FlowRuntime::restore_checkpoint(base, &codecs, limits),
        Err(FlowCheckpointError::LimitExceeded("payload bytes"))
    ));
    DECODE_CALLS.with(|count| assert_eq!(count.get(), 0));
}

fn lifecycle_codecs() -> FlowCheckpointCodecs {
    let mut codecs = FlowCheckpointCodecs::new();
    codecs
        .register_context("service.context.v1", 1, encode_context, decode_context)
        .unwrap();
    codecs
        .register_restart_template::<Template, Context>(
            "service.template.v1",
            1,
            encode_template,
            decode_template,
        )
        .unwrap();
    codecs
        .register_restart_factory(
            "service.template.v1",
            "service.factory.v1",
            template_context,
        )
        .unwrap();
    codecs
        .register_work_continuations(
            "service",
            FlowContinuationCodeIds {
                on_resume: Some(FlowCallbackCodeV1 {
                    stable_id: "service.resume".into(),
                    version: 1,
                }),
                on_restart: Some(FlowCallbackCodeV1 {
                    stable_id: "service.restart".into(),
                    version: 1,
                }),
                ..FlowContinuationCodeIds::default()
            },
            FlowContinuations {
                on_resume: Some(resume_context),
                on_restart: Some(restart_context),
                ..FlowContinuations::default()
            },
        )
        .unwrap();
    codecs
}

fn register_lifecycle_callbacks(runtime: &mut FlowRuntime) {
    runtime
        .register_work_continuations(
            "service",
            FlowContinuations {
                on_resume: Some(resume_context),
                on_restart: Some(restart_context),
                ..FlowContinuations::default()
            },
        )
        .unwrap();
}

#[test]
fn active_restartable_work_preserves_mutated_context_and_restarts_from_saved_template() {
    let codecs = lifecycle_codecs();
    let mut source = FlowRuntime::new();
    register_lifecycle_callbacks(&mut source);
    let owner = source.spawn_actor().unwrap();
    let resource = source.create_resource(1).unwrap();
    let low_work = source
        .create_restartable_work(
            owner,
            SimDuration::from_ticks(20),
            "service",
            Template { initial: 17 },
            template_context,
        )
        .unwrap();
    let low_request = source
        .acquire(resource)
        .owner(owner)
        .timed_work(low_work)
        .priority(9)
        .preemptible(PreemptionStrategy::Restart)
        .submit()
        .unwrap();
    source.step().unwrap();
    assert_eq!(
        source.request(low_request).unwrap().state,
        RequestState::Active
    );
    assert_eq!(
        source.work_progress(low_work).unwrap().state,
        WorkState::Active
    );

    let active_image = source
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    let active_restored =
        FlowRuntime::restore_checkpoint(active_image, &codecs, FlowCheckpointLimits::default())
            .unwrap();
    assert_eq!(
        active_restored.request(low_request).unwrap(),
        source.request(low_request).unwrap()
    );
    assert_eq!(
        active_restored.work_progress(low_work).unwrap(),
        source.work_progress(low_work).unwrap()
    );
    assert_eq!(
        active_restored.resource(resource).unwrap(),
        source.resource(resource).unwrap()
    );

    let urgent_work = source
        .create_work(
            owner,
            SimDuration::from_ticks(2),
            "service",
            Context { value: 500 },
        )
        .unwrap();
    let urgent_request = source
        .acquire(resource)
        .owner(owner)
        .timed_work(urgent_work)
        .priority(1)
        .can_preempt(true)
        .at(SimTime::from_ticks(5))
        .submit()
        .unwrap();
    source.step().unwrap();
    assert_eq!(
        source.request(low_request).unwrap().state,
        RequestState::Suspended
    );
    assert_eq!(
        source.request(urgent_request).unwrap().state,
        RequestState::Active
    );
    assert_eq!(
        source.work_progress(low_work).unwrap().state,
        WorkState::Suspended
    );

    let suspended_image = source
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    let mut restored =
        FlowRuntime::restore_checkpoint(suspended_image, &codecs, FlowCheckpointLimits::default())
            .unwrap();
    assert_eq!(
        restored.request(low_request).unwrap().state,
        RequestState::Suspended
    );

    let source_suffix = source.run_for(2).unwrap();
    let restored_suffix = restored.run_for(2).unwrap();
    assert_eq!(source_suffix, restored_suffix);
    assert_eq!(source.work_context::<Context>(low_work).unwrap().value, 117);
    assert_eq!(
        restored.work_context::<Context>(low_work).unwrap().value,
        117
    );
    assert_eq!(
        source.work_progress(low_work).unwrap(),
        restored.work_progress(low_work).unwrap()
    );
    assert_eq!(
        source.request(low_request).unwrap(),
        restored.request(low_request).unwrap()
    );
    assert_eq!(
        source.resource(resource).unwrap(),
        restored.resource(resource).unwrap()
    );

    let high_work = source
        .create_work(
            owner,
            SimDuration::from_ticks(2),
            "service",
            Context { value: 900 },
        )
        .unwrap();
    let high_request = source
        .acquire(resource)
        .owner(owner)
        .timed_work(high_work)
        .priority(1)
        .can_preempt(true)
        .at(SimTime::from_ticks(8))
        .submit()
        .unwrap();
    let restored_high_work = restored
        .create_work(
            owner,
            SimDuration::from_ticks(2),
            "service",
            Context { value: 900 },
        )
        .unwrap();
    let restored_high_request = restored
        .acquire(resource)
        .owner(owner)
        .timed_work(restored_high_work)
        .priority(1)
        .can_preempt(true)
        .at(SimTime::from_ticks(8))
        .submit()
        .unwrap();
    assert_eq!(high_work, restored_high_work);
    assert_eq!(high_request, restored_high_request);
    assert_eq!(source.step().unwrap(), restored.step().unwrap());
    assert_eq!(
        source.request(low_request).unwrap().state,
        RequestState::Suspended
    );
    assert_eq!(
        restored.request(low_request).unwrap().state,
        RequestState::Suspended
    );

    let second_suspended_image = source
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    let mut second_restored = FlowRuntime::restore_checkpoint(
        second_suspended_image,
        &codecs,
        FlowCheckpointLimits::default(),
    )
    .unwrap();
    assert_eq!(source.work_context::<Context>(low_work).unwrap().value, 117);
    let source_suffix = source.run_for(2).unwrap();
    let restored_suffix = restored.run_for(2).unwrap();
    let second_suffix = second_restored.run_for(2).unwrap();
    assert_eq!(source_suffix, restored_suffix);
    assert_eq!(source_suffix, second_suffix);
    assert_eq!(source.work_context::<Context>(low_work).unwrap().value, 117);
    assert_eq!(
        second_restored
            .work_context::<Context>(low_work)
            .unwrap()
            .value,
        117
    );
    assert_eq!(
        source.work_progress(low_work).unwrap().state,
        WorkState::Active
    );
    assert_eq!(
        source.work_progress(low_work).unwrap(),
        second_restored.work_progress(low_work).unwrap()
    );
    assert_eq!(
        source.request(low_request).unwrap(),
        second_restored.request(low_request).unwrap()
    );
    assert_eq!(
        source.resource(resource).unwrap(),
        second_restored.resource(resource).unwrap()
    );
}

#[test]
fn active_suspend_checkpoint_resumes_with_matching_progress_and_resources() {
    let codecs = lifecycle_codecs();
    let mut source = FlowRuntime::new();
    register_lifecycle_callbacks(&mut source);
    let owner = source.spawn_actor().unwrap();
    let resource = source.create_resource(1).unwrap();
    let low_work = source
        .create_work(
            owner,
            SimDuration::from_ticks(20),
            "service",
            Context { value: 2 },
        )
        .unwrap();
    let low_request = source
        .acquire(resource)
        .owner(owner)
        .timed_work(low_work)
        .priority(9)
        .preemptible(PreemptionStrategy::Suspend)
        .submit()
        .unwrap();
    source.step().unwrap();
    let active_image = source
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    assert!(source.request(low_request).unwrap().lease.is_some());
    let mut over_capacity = active_image.clone();
    over_capacity.builtins.capacities.as_mut().unwrap().rows[0]
        .1
        .total = 0;
    DECODE_CALLS.with(|count| count.set(0));
    assert!(matches!(
        FlowRuntime::restore_checkpoint(over_capacity, &codecs, FlowCheckpointLimits::default()),
        Err(FlowCheckpointError::InvalidState(_))
    ));
    DECODE_CALLS.with(|count| assert_eq!(count.get(), 0));
    let mut mismatched_progress = active_image.clone();
    mismatched_progress
        .builtins
        .work_progress
        .as_mut()
        .unwrap()
        .rows[0]
        .1
        .state = WorkState::Suspended;
    DECODE_CALLS.with(|count| count.set(0));
    assert!(matches!(
        FlowRuntime::restore_checkpoint(
            mismatched_progress,
            &codecs,
            FlowCheckpointLimits::default()
        ),
        Err(FlowCheckpointError::InvalidState(_))
    ));
    DECODE_CALLS.with(|count| assert_eq!(count.get(), 0));
    let active_restored =
        FlowRuntime::restore_checkpoint(active_image, &codecs, FlowCheckpointLimits::default())
            .unwrap();
    assert_eq!(
        active_restored.request(low_request).unwrap(),
        source.request(low_request).unwrap()
    );

    let urgent_work = source
        .create_work(
            owner,
            SimDuration::from_ticks(2),
            "service",
            Context { value: 3 },
        )
        .unwrap();
    let urgent_request = source
        .acquire(resource)
        .owner(owner)
        .timed_work(urgent_work)
        .priority(1)
        .can_preempt(true)
        .at(SimTime::from_ticks(5))
        .submit()
        .unwrap();
    source.step().unwrap();
    assert_eq!(
        source.request(low_request).unwrap().state,
        RequestState::Suspended
    );
    let suspended_image = source
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    let mut restored =
        FlowRuntime::restore_checkpoint(suspended_image, &codecs, FlowCheckpointLimits::default())
            .unwrap();
    let source_suffix = source.run_for(2).unwrap();
    let restored_suffix = restored.run_for(2).unwrap();
    assert_eq!(source_suffix, restored_suffix);
    assert_eq!(
        source.request(urgent_request).unwrap(),
        restored.request(urgent_request).unwrap()
    );
    assert_eq!(
        source.request(low_request).unwrap(),
        restored.request(low_request).unwrap()
    );
    assert_eq!(
        source.work_progress(low_work).unwrap(),
        restored.work_progress(low_work).unwrap()
    );
    assert_eq!(
        source.work_progress(low_work).unwrap().state,
        WorkState::Active
    );
    assert_eq!(source.work_context::<Context>(low_work).unwrap().value, 52);
    assert_eq!(
        restored.work_context::<Context>(low_work).unwrap().value,
        52
    );
    assert_eq!(
        source.resource(resource).unwrap(),
        restored.resource(resource).unwrap()
    );
}

#[test]
fn restore_rejects_dead_request_and_queued_request_with_missing_resource_before_decoder() {
    let codecs = register_codec(mutate_context);
    let mut source = FlowRuntime::new();
    source
        .register_domain_view_hook("model.ctx", EventKind::custom(91), mutate_context)
        .unwrap();
    let owner = source.spawn_actor().unwrap();
    let resource = source.create_resource(0).unwrap();
    let _work = source
        .create_actor_domain_context(
            owner,
            "model.ctx",
            EventKind::custom(91),
            Context { value: 4 },
        )
        .unwrap();
    let request = source
        .acquire(resource)
        .owner(owner)
        .deadline(SimTime::from_ticks(20))
        .submit()
        .unwrap();
    source.step().unwrap();
    assert_eq!(source.request(request).unwrap().state, RequestState::Queued);
    let base = source
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .unwrap();

    let mut dead_request = base.clone();
    let entity = request.entity_id();
    dead_request
        .world
        .live_entities
        .retain(|candidate| *candidate != entity);
    dead_request.world.slots[entity.index as usize].alive = false;
    dead_request.world.free_indices.push(entity.index);
    DECODE_CALLS.with(|count| count.set(0));
    assert!(matches!(
        FlowRuntime::restore_checkpoint(dead_request, &codecs, FlowCheckpointLimits::default()),
        Err(FlowCheckpointError::InvalidState(_))
    ));
    DECODE_CALLS.with(|count| assert_eq!(count.get(), 0));

    let mut missing_resource = base;
    let resource_entity = resource.entity_id();
    missing_resource.resources.clear();
    macro_rules! remove_resource_row {
        ($store:expr) => {
            if let Some(store) = &mut $store {
                store.rows.retain(|(entity, _)| *entity != resource_entity);
            }
        };
    }
    remove_resource_row!(missing_resource.builtins.capacities);
    remove_resource_row!(missing_resource.builtins.queues);
    remove_resource_row!(missing_resource.builtins.deadlines);
    remove_resource_row!(missing_resource.builtins.preempting);
    remove_resource_row!(missing_resource.builtins.allocations);
    DECODE_CALLS.with(|count| count.set(0));
    assert!(matches!(
        FlowRuntime::restore_checkpoint(missing_resource, &codecs, FlowCheckpointLimits::default()),
        Err(FlowCheckpointError::InvalidState(_))
    ));
    DECODE_CALLS.with(|count| assert_eq!(count.get(), 0));
}
