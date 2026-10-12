use kairo_ecs_des::{
    FlowAllocationMode, FlowCallbackCodeV1, FlowCallbackSnapshot, FlowCheckpointCodecError,
    FlowCheckpointCodecs, FlowCheckpointError, FlowCheckpointImage, FlowCheckpointLimits,
    FlowCheckpointWireError, FlowCheckpointWireLimits, FlowCommandSink, FlowCommandV3,
    FlowDomainControl, FlowQueueOrderKey, FlowQueueOrdering, FlowRequestQueueOrderV4, FlowRuntime,
    FlowWorldView, RequestState,
};
use kairo_ecs_types::{SimDuration, SimTime};
use std::cell::Cell;

thread_local! {
    static CODEC_CALLS: Cell<usize> = const { Cell::new(0) };
}

fn t(ticks: u128) -> SimTime {
    SimTime::from_ticks(ticks)
}

fn duration(ticks: u128) -> SimDuration {
    SimDuration::from_ticks(ticks)
}

fn key(bytes: &[u8]) -> FlowQueueOrderKey {
    FlowQueueOrderKey::new(bytes).unwrap()
}

fn encode_u64(value: &u64, remaining: usize) -> Result<Vec<u8>, FlowCheckpointCodecError> {
    CODEC_CALLS.with(|calls| calls.set(calls.get() + 1));
    if remaining < 8 {
        return Err(FlowCheckpointCodecError("payload limit".into()));
    }
    Ok(value.to_le_bytes().to_vec())
}

fn decode_u64(
    bytes: &[u8],
    _: &kairo_ecs_des::FlowCheckpointRebindV1,
) -> Result<u64, FlowCheckpointCodecError> {
    CODEC_CALLS.with(|calls| calls.set(calls.get() + 1));
    let payload: [u8; 8] = bytes
        .try_into()
        .map_err(|_| FlowCheckpointCodecError("bad u64 payload".into()))?;
    Ok(u64::from_le_bytes(payload))
}

fn codecs() -> FlowCheckpointCodecs {
    let mut codecs = FlowCheckpointCodecs::new();
    codecs
        .register_context("ctx", 1, encode_u64, decode_u64)
        .unwrap();
    codecs
        .register_restart_template::<u64, u64>("template", 1, encode_u64, decode_u64)
        .unwrap();
    codecs
        .register_restart_factory::<u64, u64>("template", "identity", identity_context)
        .unwrap();
    codecs
}

fn identity_context(template: &u64) -> u64 {
    *template
}

fn noop_domain(
    _: &mut u64,
    _: &FlowCallbackSnapshot,
    _: FlowWorldView<'_>,
    _: &mut FlowCommandSink,
) {
}

fn assert_legacy_captures_fail_closed(runtime: &FlowRuntime, codecs: &FlowCheckpointCodecs) {
    CODEC_CALLS.with(|calls| calls.set(0));
    assert!(matches!(
        runtime.capture_checkpoint(codecs, limits()),
        Err(FlowCheckpointError::InvalidState(_))
    ));
    CODEC_CALLS.with(|calls| assert_eq!(calls.get(), 0));
    CODEC_CALLS.with(|calls| calls.set(0));
    assert!(matches!(
        runtime.capture_checkpoint_v2(codecs, limits()),
        Err(FlowCheckpointError::InvalidState(_))
    ));
    CODEC_CALLS.with(|calls| assert_eq!(calls.get(), 0));
    CODEC_CALLS.with(|calls| calls.set(0));
    assert!(matches!(
        runtime.capture_checkpoint_v3(codecs, limits()),
        Err(FlowCheckpointError::InvalidState(_))
    ));
    CODEC_CALLS.with(|calls| assert_eq!(calls.get(), 0));
}

fn limits() -> FlowCheckpointLimits {
    FlowCheckpointLimits::default()
}

fn wire_limits() -> FlowCheckpointWireLimits {
    FlowCheckpointWireLimits::default()
}

fn expect_v4(
    runtime: &FlowRuntime,
    codecs: &FlowCheckpointCodecs,
) -> kairo_ecs_des::FlowCheckpointV4 {
    match runtime.capture_checkpoint_image(codecs, limits()).unwrap() {
        FlowCheckpointImage::V4(image) => image,
        other => panic!("stable queue metadata selected a legacy image: {other:?}"),
    }
}

#[test]
fn routing_v4_empty_stable_resource_selects_before_capture() {
    let codecs = FlowCheckpointCodecs::new();
    let mut stable = FlowRuntime::new();
    let stable_id = stable
        .create_resource_with_queue_ordering(
            0,
            FlowAllocationMode::Eager,
            FlowQueueOrdering::StableKey,
        )
        .unwrap();
    let routed = stable.capture_checkpoint_image(&codecs, limits()).unwrap();
    let FlowCheckpointImage::V4(image) = routed else {
        panic!("an empty StableKey resource must select V4 before capture");
    };
    assert_eq!(
        image,
        stable.capture_checkpoint_v4(&codecs, limits()).unwrap()
    );
    assert_eq!(image.resources, vec![stable_id]);
    assert_eq!(
        image.queue_order_modes,
        vec![(stable_id, FlowQueueOrdering::StableKey)]
    );
    assert!(image.request_queue_orders.is_empty());

    let mut admission = FlowRuntime::new();
    admission.create_resource(0).unwrap();
    assert!(matches!(
        admission
            .capture_checkpoint_image(&codecs, limits())
            .unwrap(),
        FlowCheckpointImage::V1(_)
    ));
}

#[test]
fn routing_v4_all_keyed_lifetimes_and_removed_generation() {
    let codecs = codecs();
    let mut queued = FlowRuntime::new();
    let owner = queued.spawn_actor().unwrap();
    let resource = queued
        .create_resource_with_queue_ordering(
            1,
            FlowAllocationMode::Eager,
            FlowQueueOrdering::StableKey,
        )
        .unwrap();
    let active_work = queued
        .create_work(owner, duration(30), "ctx", 1_u64)
        .unwrap();
    let active = queued
        .acquire(resource)
        .owner(owner)
        .timed_work(active_work)
        .preemptible(kairo_ecs_des::PreemptionStrategy::Suspend)
        .queue_order_key(key(b"active"))
        .submit()
        .unwrap();
    queued.step().unwrap().unwrap();
    let waiting = queued
        .acquire(resource)
        .owner(owner)
        .queue_order_key(key(b"waiting"))
        .submit()
        .unwrap();
    queued.step().unwrap().unwrap();
    let future = queued
        .acquire(resource)
        .owner(owner)
        .at(t(50))
        .queue_order_key(key(b"future"))
        .submit()
        .unwrap();
    assert_eq!(queued.request(active).unwrap().state, RequestState::Active);
    assert_eq!(queued.request(waiting).unwrap().state, RequestState::Queued);
    assert_eq!(queued.request(future).unwrap().state, RequestState::Pending);
    let queued_image = expect_v4(&queued, &codecs);
    assert_eq!(queued_image.request_queue_orders.len(), 3);

    let mut suspended = FlowRuntime::new();
    let owner = suspended.spawn_actor().unwrap();
    let resource = suspended
        .create_resource_with_queue_ordering(
            1,
            FlowAllocationMode::Eager,
            FlowQueueOrdering::StableKey,
        )
        .unwrap();
    let holder_work = suspended
        .create_work(owner, duration(40), "ctx", 2_u64)
        .unwrap();
    let holder = suspended
        .acquire(resource)
        .owner(owner)
        .timed_work(holder_work)
        .preemptible(kairo_ecs_des::PreemptionStrategy::Suspend)
        .queue_order_key(key(b"holder"))
        .submit()
        .unwrap();
    suspended.step().unwrap().unwrap();
    let urgent_work = suspended
        .create_work(owner, duration(3), "ctx", 3_u64)
        .unwrap();
    let urgent = suspended
        .acquire(resource)
        .owner(owner)
        .at(t(1))
        .timed_work(urgent_work)
        .priority(-1)
        .can_preempt(true)
        .queue_order_key(key(b"urgent"))
        .submit()
        .unwrap();
    suspended.step().unwrap().unwrap();
    assert_eq!(
        suspended.request(holder).unwrap().state,
        RequestState::Suspended
    );
    assert_eq!(
        suspended.request(urgent).unwrap().state,
        RequestState::Active
    );
    let suspended_image = expect_v4(&suspended, &codecs);
    assert!(suspended_image
        .request_queue_orders
        .iter()
        .any(|(id, _)| *id == holder));

    let mut removed = FlowRuntime::new();
    let owner = removed.spawn_actor().unwrap();
    let old_resource = removed
        .create_resource_with_queue_ordering(
            0,
            FlowAllocationMode::Eager,
            FlowQueueOrdering::StableKey,
        )
        .unwrap();
    let terminal = removed
        .acquire(old_resource)
        .owner(owner)
        .queue_order_key(key(b"retained"))
        .submit()
        .unwrap();
    removed.step().unwrap().unwrap();
    removed.cancel(terminal, t(1)).unwrap();
    removed.step().unwrap().unwrap();
    assert_eq!(
        removed.request(terminal).unwrap().state,
        RequestState::Cancelled
    );
    removed.remove_resource(old_resource).unwrap();
    removed.step().unwrap().unwrap();
    let new_resource = removed
        .create_resource_with_queue_ordering(
            1,
            FlowAllocationMode::Eager,
            FlowQueueOrdering::StableKey,
        )
        .unwrap();
    assert_eq!(
        old_resource.entity_id().index,
        new_resource.entity_id().index
    );
    assert_ne!(
        old_resource.entity_id().generation,
        new_resource.entity_id().generation
    );
    let removed_image = expect_v4(&removed, &codecs);
    assert!(!removed_image
        .queue_order_modes
        .iter()
        .any(|(id, _)| *id == old_resource));
    assert!(removed_image
        .request_queue_orders
        .iter()
        .any(|(id, row)| *id == terminal
            && matches!(row, FlowRequestQueueOrderV4::StableKey(bytes) if bytes == b"retained")));
}

#[test]
fn routing_v4_ref_borrowed_accessors_and_commands_complete() {
    let mut runtime = FlowRuntime::new();
    let kind = kairo_ecs_types::EventKind::custom(91);
    runtime
        .register_domain_view_hook("ctx", kind, noop_domain)
        .unwrap();
    let owner = runtime.spawn_actor().unwrap();
    let active_resource = runtime
        .create_resource_with_queue_ordering(
            1,
            FlowAllocationMode::Eager,
            FlowQueueOrdering::StableKey,
        )
        .unwrap();
    let deferred_resource = runtime
        .create_resource_with_queue_ordering(
            1,
            FlowAllocationMode::Deferred,
            FlowQueueOrdering::StableKey,
        )
        .unwrap();
    let work = runtime
        .create_restartable_work(owner, duration(30), "ctx", 42_u64, identity_context)
        .unwrap();
    let request = runtime
        .acquire(active_resource)
        .owner(owner)
        .timed_work(work)
        .queue_order_key(key(b"active"))
        .submit()
        .unwrap();
    runtime.step().unwrap().unwrap();
    let queued = runtime
        .acquire(deferred_resource)
        .owner(owner)
        .at(t(5))
        .queue_order_key(key(b"queued"))
        .submit()
        .unwrap();
    let flush = runtime
        .schedule_allocation_flush(deferred_resource, t(20), 0)
        .unwrap();
    let grant_boundary = runtime
        .schedule_resource_grant_enabled(active_resource, false, t(10), 0)
        .unwrap();
    let mut codecset = codecs();
    codecset
        .register_domain_view_hook::<u64>(
            "ctx",
            kind,
            noop_domain,
            FlowCallbackCodeV1 {
                stable_id: "routing.domain".into(),
                version: 1,
            },
        )
        .unwrap();
    runtime.schedule_domain(work, kind, t(12), 0).unwrap();
    runtime
        .schedule_domain_control(work, kind, FlowDomainControl::Pause, t(13), 0)
        .unwrap();
    let image = expect_v4(&runtime, &codecset);
    let owned_image = FlowCheckpointImage::V4(image);
    let FlowCheckpointImage::V4(image) = &owned_image else {
        unreachable!()
    };
    let reference = owned_image.as_ref();
    assert!(std::ptr::eq(reference.scheduler(), &image.scheduler));
    assert!(std::ptr::eq(
        reference.work_registrations().as_ptr(),
        image.work_registrations.as_ptr()
    ));
    assert!(std::ptr::eq(
        reference.context_stores().as_ptr(),
        image.context_stores.as_ptr()
    ));
    assert!(!image.work_registrations.is_empty());
    assert!(!image.context_stores.is_empty());
    let commands: Vec<_> = reference.commands().collect();
    assert_eq!(commands, image.commands);
    assert!(commands
        .iter()
        .any(|(_, c)| *c == FlowCommandV3::Submit(queued)));
    assert!(commands
        .iter()
        .any(|(_, c)| matches!(c, FlowCommandV3::Completion(id, _, _, _) if *id == request)));
    assert!(commands
        .iter()
        .any(|(_, c)| *c == FlowCommandV3::GrantEnabled(active_resource, false)));
    assert!(commands
        .iter()
        .any(|(_, c)| matches!(c, FlowCommandV3::Domain(id, k) if *id == work && *k == kind)));
    assert!(commands.iter().any(|(_, c)| matches!(c, FlowCommandV3::DomainControl(id, k, FlowDomainControl::Pause) if *id == work && *k == kind)));
    assert!(commands
        .iter()
        .any(|(id, c)| *id == flush && *c == FlowCommandV3::AllocationFlush(deferred_resource)));
    assert_eq!(
        grant_boundary,
        commands
            .iter()
            .find(|(_, c)| matches!(c, FlowCommandV3::GrantEnabled(_, _)))
            .unwrap()
            .0
    );
}

#[test]
fn routing_v4_image_direct_native_restore_suffix_equal() {
    let codecs = codecs();
    let mut source = FlowRuntime::new();
    let owner = source.spawn_actor().unwrap();
    let resource = source
        .create_resource_with_queue_ordering(
            1,
            FlowAllocationMode::Eager,
            FlowQueueOrdering::StableKey,
        )
        .unwrap();
    let work = source
        .create_work(owner, duration(3), "ctx", 90_u64)
        .unwrap();
    let request = source
        .acquire(resource)
        .owner(owner)
        .timed_work(work)
        .queue_order_key(key(b"suffix"))
        .submit()
        .unwrap();
    source.step().unwrap().unwrap();
    let image = expect_v4(&source, &codecs);
    let direct = FlowRuntime::restore_checkpoint_v4(image.clone(), &codecs, limits()).unwrap();
    let (via_image, rebind) = FlowCheckpointImage::V4(image.clone())
        .restore_with_rebind(&codecs, limits())
        .unwrap();
    assert_eq!(
        rebind.resolve_resource(resource.entity_id()).unwrap(),
        resource
    );
    let mut direct = direct;
    let mut via_image = via_image;
    assert_eq!(
        direct.request(request).unwrap(),
        via_image.request(request).unwrap()
    );
    for _ in 0..16 {
        let left = direct.step().unwrap();
        let right = via_image.step().unwrap();
        assert_eq!(left, right);
        assert_eq!(
            direct.request(request).unwrap(),
            via_image.request(request).unwrap()
        );
        assert_eq!(
            direct.work_progress(work).unwrap(),
            via_image.work_progress(work).unwrap()
        );
        assert_eq!(
            direct.work_context::<u64>(work).unwrap(),
            via_image.work_context::<u64>(work).unwrap()
        );
        assert_eq!(
            direct.capture_checkpoint_v4(&codecs, limits()).unwrap(),
            via_image.capture_checkpoint_v4(&codecs, limits()).unwrap()
        );
        if left.is_none() {
            break;
        }
    }
    assert_eq!(
        direct.request(request).unwrap().state,
        RequestState::Completed
    );
    assert_eq!(
        direct.request(request).unwrap(),
        via_image.request(request).unwrap()
    );
    assert_eq!(
        direct.work_progress(work).unwrap(),
        via_image.work_progress(work).unwrap()
    );
    assert_eq!(
        direct.work_context::<u64>(work).unwrap(),
        via_image.work_context::<u64>(work).unwrap()
    );
}

#[test]
fn routing_v4_image_wire_magic_roundtrip() {
    let codecs = codecs();
    let mut stable = FlowRuntime::new();
    let owner = stable.spawn_actor().unwrap();
    let resource = stable
        .create_resource_with_queue_ordering(
            1,
            FlowAllocationMode::Eager,
            FlowQueueOrdering::StableKey,
        )
        .unwrap();
    let work = stable
        .create_restartable_work(owner, duration(3), "ctx", 41_u64, identity_context)
        .unwrap();
    let request = stable
        .acquire(resource)
        .owner(owner)
        .timed_work(work)
        .queue_order_key(key(b"wire"))
        .submit()
        .unwrap();
    stable.step().unwrap().unwrap();
    let image = FlowCheckpointImage::V4(expect_v4(&stable, &codecs));
    let bytes = image.encode_wire(wire_limits()).unwrap();
    assert_eq!(&bytes[..8], b"KFLOWV4\0");
    let direct_bytes = match &image {
        FlowCheckpointImage::V4(direct) => direct.encode_wire_v4(wire_limits()).unwrap(),
        _ => unreachable!(),
    };
    assert_eq!(bytes, direct_bytes);
    let decoded = FlowCheckpointImage::decode_wire(&bytes, wire_limits()).unwrap();
    assert_eq!(decoded, image);
    assert_eq!(decoded.encode_wire(wire_limits()).unwrap(), bytes);

    let direct_image = match image.clone() {
        FlowCheckpointImage::V4(v4) => v4,
        _ => unreachable!(),
    };
    let mut direct_restored =
        FlowRuntime::restore_checkpoint_v4(direct_image, &codecs, limits()).unwrap();
    let (mut routed_restored, _) = decoded.restore_with_rebind(&codecs, limits()).unwrap();
    for _ in 0..16 {
        let left = direct_restored.step().unwrap();
        let right = routed_restored.step().unwrap();
        assert_eq!(left, right);
        assert_eq!(
            direct_restored.request(request).unwrap(),
            routed_restored.request(request).unwrap()
        );
        assert_eq!(
            direct_restored.work_progress(work).unwrap(),
            routed_restored.work_progress(work).unwrap()
        );
        assert_eq!(
            direct_restored
                .capture_checkpoint_v4(&codecs, limits())
                .unwrap(),
            routed_restored
                .capture_checkpoint_v4(&codecs, limits())
                .unwrap()
        );
        if left.is_none() {
            break;
        }
    }
    assert_eq!(
        direct_restored.request(request).unwrap().state,
        RequestState::Completed
    );

    let mut admission = FlowRuntime::new();
    let resource = admission.create_resource(1).unwrap();
    let owner = admission.spawn_actor().unwrap();
    let request = admission.acquire(resource).owner(owner).submit().unwrap();
    admission.step().unwrap().unwrap();
    let v1 = admission.capture_checkpoint(&codecs, limits()).unwrap();
    let v2 = admission.capture_checkpoint_v2(&codecs, limits()).unwrap();
    let v3 = admission.capture_checkpoint_v3(&codecs, limits()).unwrap();
    for (legacy, magic) in [
        (FlowCheckpointImage::V1(v1), b"KFLOWV1\0" as &[u8]),
        (FlowCheckpointImage::V2(v2), b"KFLOWV2\0" as &[u8]),
        (FlowCheckpointImage::V3(v3), b"KFLOWV3\0" as &[u8]),
    ] {
        let legacy_bytes = legacy.encode_wire(wire_limits()).unwrap();
        assert_eq!(&legacy_bytes[..8], magic);
        assert_eq!(
            FlowCheckpointImage::decode_wire(&legacy_bytes, wire_limits()).unwrap(),
            legacy
        );
    }
    assert_eq!(
        admission.request(request).unwrap().state,
        RequestState::Active
    );
    assert_eq!(
        FlowCheckpointImage::decode_wire(b"KFLOWX4\0", wire_limits()),
        Err(FlowCheckpointWireError::InvalidTag)
    );
    assert_eq!(
        FlowCheckpointImage::decode_wire(b"short", wire_limits()),
        Err(FlowCheckpointWireError::Truncated)
    );
}

#[test]
fn routing_v4_malformed_metadata_and_limits_never_fallback() {
    let codecs = codecs();
    let mut runtime = FlowRuntime::new();
    let owner = runtime.spawn_actor().unwrap();
    let resource = runtime
        .create_resource_with_queue_ordering(
            1,
            FlowAllocationMode::Eager,
            FlowQueueOrdering::StableKey,
        )
        .unwrap();
    let work = runtime
        .create_work(owner, duration(5), "ctx", 5_u64)
        .unwrap();
    let request = runtime
        .acquire(resource)
        .owner(owner)
        .timed_work(work)
        .queue_order_key(key(b"limited"))
        .submit()
        .unwrap();
    runtime.step().unwrap().unwrap();
    let routed = runtime.capture_checkpoint_image(&codecs, limits()).unwrap();
    assert!(matches!(routed, FlowCheckpointImage::V4(_)));
    let exact = FlowCheckpointLimits {
        max_resources: 1,
        max_requests: 1,
        ..limits()
    };
    assert!(runtime.capture_checkpoint_image(&codecs, exact).is_ok());
    let too_few = FlowCheckpointLimits {
        max_resources: 0,
        ..limits()
    };
    CODEC_CALLS.with(|calls| calls.set(0));
    assert!(matches!(
        runtime.capture_checkpoint_image(&codecs, too_few),
        Err(FlowCheckpointError::LimitExceeded(_))
    ));
    CODEC_CALLS.with(|calls| assert_eq!(calls.get(), 0));

    let FlowCheckpointImage::V4(mut malformed) = routed else {
        unreachable!()
    };
    let (_, order) = malformed
        .request_queue_orders
        .iter_mut()
        .find(|(id, _)| *id == request)
        .expect("fixture request key missing");
    *order = FlowRequestQueueOrderV4::StableKey(Vec::new());
    CODEC_CALLS.with(|calls| calls.set(0));
    assert!(matches!(
        FlowRuntime::restore_checkpoint_v4(malformed, &codecs, limits()),
        Err(FlowCheckpointError::InvalidState(_))
    ));
    CODEC_CALLS.with(|calls| assert_eq!(calls.get(), 0));
}

#[test]
fn routing_v4_legacy_minimal_selection_unchanged() {
    let codecs = codecs();
    let mut eager = FlowRuntime::new();
    eager.create_resource(1).unwrap();
    assert_eq!(
        eager.capture_checkpoint_image(&codecs, limits()).unwrap(),
        FlowCheckpointImage::V1(eager.capture_checkpoint(&codecs, limits()).unwrap())
    );

    let mut disabled = FlowRuntime::new();
    let resource = disabled.create_resource(1).unwrap();
    disabled
        .schedule_resource_grant_enabled(resource, false, t(2), 0)
        .unwrap();
    assert_eq!(
        disabled
            .capture_checkpoint_image(&codecs, limits())
            .unwrap(),
        FlowCheckpointImage::V2(disabled.capture_checkpoint_v2(&codecs, limits()).unwrap())
    );

    let mut deferred = FlowRuntime::new();
    deferred
        .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
        .unwrap();
    assert_eq!(
        deferred
            .capture_checkpoint_image(&codecs, limits())
            .unwrap(),
        FlowCheckpointImage::V3(deferred.capture_checkpoint_v3(&codecs, limits()).unwrap())
    );

    let mut flush = FlowRuntime::new();
    let resource = flush
        .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
        .unwrap();
    flush.schedule_allocation_flush(resource, t(3), 0).unwrap();
    assert_eq!(
        flush.capture_checkpoint_image(&codecs, limits()).unwrap(),
        FlowCheckpointImage::V3(flush.capture_checkpoint_v3(&codecs, limits()).unwrap())
    );

    let mut current_flush = FlowRuntime::new();
    let resource = current_flush
        .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
        .unwrap();
    current_flush
        .schedule_allocation_flush(resource, t(0), 0)
        .unwrap();
    current_flush.step().unwrap().unwrap();
    current_flush
        .schedule_allocation_flush(resource, t(0), 0)
        .unwrap();
    assert_eq!(
        current_flush
            .capture_checkpoint_image(&codecs, limits())
            .unwrap(),
        FlowCheckpointImage::V3(
            current_flush
                .capture_checkpoint_v3(&codecs, limits())
                .unwrap()
        )
    );

    let mut frontier = FlowRuntime::new();
    let resource = frontier.create_resource(1).unwrap();
    frontier
        .schedule_resource_grant_enabled(resource, true, t(1), 0)
        .unwrap();
    frontier.step().unwrap().unwrap();
    assert_eq!(
        frontier
            .capture_checkpoint_image(&codecs, limits())
            .unwrap(),
        FlowCheckpointImage::V1(frontier.capture_checkpoint(&codecs, limits()).unwrap())
    );

    let mut halted = FlowRuntime::with_config(kairo_ecs_des::FlowConfig {
        max_same_tick_flow_transitions: std::num::NonZeroU64::new(1).unwrap(),
    });
    let owner = halted.spawn_actor().unwrap();
    let resource = halted.create_resource(1).unwrap();
    let work = halted
        .create_work(owner, duration(10), "ctx", 42_u64)
        .unwrap();
    halted
        .acquire(resource)
        .owner(owner)
        .timed_work(work)
        .submit()
        .unwrap();
    let gate = halted
        .schedule_resource_grant_enabled(resource, false, t(8), 0)
        .unwrap();
    assert!(matches!(
        halted.step(),
        Err(kairo_ecs_des::FlowError::SameTickBudgetExceeded {
            at_ticks: 0,
            limit: 1
        })
    ));
    assert_eq!(halted.budget_snapshot().tick, None);
    assert_eq!(halted.budget_snapshot().consumed, 0);
    let halted_v2 = halted.capture_checkpoint_v2(&codecs, limits()).unwrap();
    assert!(halted_v2.budget_halt.is_some());
    assert_eq!(halted_v2.started_frontier, None);
    assert!(halted_v2.commands.iter().any(|(id, command)| {
        *id == gate && *command == kairo_ecs_des::FlowCommandV2::GrantEnabled(resource, false)
    }));
    assert_eq!(
        halted.capture_checkpoint_image(&codecs, limits()).unwrap(),
        FlowCheckpointImage::V2(halted_v2)
    );
}

#[test]
fn routing_v4_old_direct_capture_remains_failclosed() {
    let codecs = codecs();
    let mut runtime = FlowRuntime::new();
    let owner = runtime.spawn_actor().unwrap();
    let resource = runtime
        .create_resource_with_queue_ordering(
            1,
            FlowAllocationMode::Eager,
            FlowQueueOrdering::StableKey,
        )
        .unwrap();
    let work = runtime
        .create_work(owner, duration(4), "ctx", 11_u64)
        .unwrap();
    runtime
        .acquire(resource)
        .owner(owner)
        .timed_work(work)
        .queue_order_key(key(b"key"))
        .submit()
        .unwrap();
    runtime.step().unwrap().unwrap();
    let routed = expect_v4(&runtime, &codecs);
    assert_legacy_captures_fail_closed(&runtime, &codecs);
    assert_eq!(expect_v4(&runtime, &codecs), routed);

    let mut terminal = FlowRuntime::new();
    let owner = terminal.spawn_actor().unwrap();
    let resource = terminal
        .create_resource_with_queue_ordering(
            0,
            FlowAllocationMode::Eager,
            FlowQueueOrdering::StableKey,
        )
        .unwrap();
    let request = terminal
        .acquire(resource)
        .owner(owner)
        .queue_order_key(key(b"old"))
        .submit()
        .unwrap();
    terminal.step().unwrap().unwrap();
    terminal.cancel(request, t(1)).unwrap();
    terminal.step().unwrap().unwrap();
    terminal.remove_resource(resource).unwrap();
    terminal.step().unwrap().unwrap();
    let terminal_image = expect_v4(&terminal, &codecs);
    assert!(terminal_image
        .request_queue_orders
        .iter()
        .any(|(id, _)| *id == request));
    assert_legacy_captures_fail_closed(&terminal, &codecs);
}
