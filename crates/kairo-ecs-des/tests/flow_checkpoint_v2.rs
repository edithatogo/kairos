use kairo_ecs_des::{
    FlowCheckpointCodecError, FlowCheckpointCodecs, FlowCheckpointError, FlowCheckpointLimits,
    FlowCheckpointRebindV1, FlowRuntime, LifecycleTransition, FLOW_CHECKPOINT_VERSION_V2,
};
use kairo_ecs_types::{SimDuration, SimTime};
use std::cell::Cell;

thread_local! {
    static DECODER_CALLS: Cell<usize> = const { Cell::new(0) };
}

fn tick(value: u128) -> SimTime {
    SimTime::from_ticks(value)
}

#[test]
fn native_v2_roundtrips_disabled_resource_and_pending_enable() {
    assert_eq!(FLOW_CHECKPOINT_VERSION_V2, 2);
    let mut source = FlowRuntime::new();
    let resource = source.create_resource(1).unwrap();
    source
        .schedule_resource_grant_enabled(resource, false, tick(0), 0)
        .unwrap();
    source.run_for(1).unwrap();
    source
        .schedule_resource_grant_enabled(resource, true, tick(8), 0)
        .unwrap();
    let codecs = FlowCheckpointCodecs::new();
    let limits = FlowCheckpointLimits::default();

    assert!(matches!(
        source.capture_checkpoint(&codecs, limits),
        Err(FlowCheckpointError::InvalidState(_))
    ));
    let image = source.capture_checkpoint_v2(&codecs, limits).unwrap();
    assert_eq!(image.version, 2);
    assert_eq!(image.resource_grant_flags, vec![(resource, false)]);
    let mut restored = FlowRuntime::restore_checkpoint_v2(image, &codecs, limits).unwrap();
    assert_eq!(restored.resource_grant_enabled(resource), Ok(false));
    let source_tail = source.run_for(8).unwrap();
    let restored_tail = restored.run_for(8).unwrap();
    assert_eq!(restored_tail, source_tail);
    assert_eq!(restored.resource_grant_enabled(resource), Ok(true));
}

#[test]
fn image_routing_keeps_ungated_images_as_v1() {
    let mut source = FlowRuntime::new();
    source.create_resource(1).unwrap();
    let codecs = FlowCheckpointCodecs::new();
    let image = source
        .capture_checkpoint_image(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    assert!(matches!(
        image.as_ref(),
        kairo_ecs_des::FlowCheckpointRef::V1(_)
    ));
}

#[test]
fn image_routing_keeps_future_enabled_gate_images_as_v2() {
    let mut source = FlowRuntime::new();
    let resource = source.create_resource(1).unwrap();
    source
        .schedule_resource_grant_enabled(resource, true, tick(8), 0)
        .unwrap();
    let image = source
        .capture_checkpoint_image(
            &FlowCheckpointCodecs::new(),
            FlowCheckpointLimits::default(),
        )
        .unwrap();
    assert!(matches!(
        image.as_ref(),
        kairo_ecs_des::FlowCheckpointRef::V2(_)
    ));
}

#[test]
fn legacy_and_v2_native_images_continue_a_common_pending_command_identically() {
    let mut source = FlowRuntime::new();
    let resource = source.create_resource(1).unwrap();
    source.create_resource(2).unwrap();
    let owner = source.spawn_actor().unwrap();
    let work = source
        .create_work(owner, SimDuration::from_ticks(4), "bytes", 0x1122_u64)
        .unwrap();
    let request = source
        .acquire(resource)
        .owner(owner)
        .timed_work(work)
        .submit()
        .unwrap();
    let mut codecs = FlowCheckpointCodecs::new();
    codecs
        .register_context("bytes", 1, encode_u64, decode_u64_counted)
        .unwrap();
    let limits = FlowCheckpointLimits::default();
    let mut legacy = source.capture_checkpoint(&codecs, limits).unwrap();
    assert!(legacy
        .context_stores
        .iter()
        .flat_map(|store| &store.rows)
        .any(|(_, bytes)| !bytes.is_empty()));
    let legacy_bytes = legacy.encode_wire_v1(Default::default()).unwrap();
    let routed = source.capture_checkpoint_image(&codecs, limits).unwrap();
    let kairo_ecs_des::FlowCheckpointImage::V1(routed) = routed else {
        panic!("ungated native image must retain V1 wire encoding");
    };
    assert_eq!(
        routed.encode_wire_v1(Default::default()).unwrap(),
        legacy_bytes
    );
    legacy.resources.reverse();
    let native = source.capture_checkpoint_v2(&codecs, limits).unwrap();
    assert_eq!(native.commands.len(), legacy.commands.len());

    let mut legacy_runtime = FlowRuntime::restore_checkpoint(legacy, &codecs, limits).unwrap();
    let mut native_runtime = FlowRuntime::restore_checkpoint_v2(native, &codecs, limits).unwrap();
    let legacy_tail = legacy_runtime.run_for(4).unwrap();
    let native_tail = native_runtime.run_for(4).unwrap();
    assert_eq!(native_tail, legacy_tail);
    assert_eq!(
        legacy_runtime.request(request).unwrap(),
        native_runtime.request(request).unwrap()
    );
}

#[test]
fn malformed_native_v2_state_fails_before_context_decoders() {
    let mut source = FlowRuntime::new();
    let owner = source.spawn_actor().unwrap();
    let resource = source.create_resource(1).unwrap();
    let stale_resource_id = source.create_resource(2).unwrap();
    source.remove_resource(stale_resource_id).unwrap();
    source.step().unwrap().unwrap();
    source
        .create_work(owner, SimDuration::from_ticks(10), "ctx", 42_u64)
        .unwrap();
    source
        .schedule_resource_grant_enabled(resource, false, tick(8), 0)
        .unwrap();
    source
        .schedule_resource_grant_enabled(resource, true, tick(9), 0)
        .unwrap();
    let mut codecs = FlowCheckpointCodecs::new();
    codecs
        .register_context("ctx", 1, encode_u64, decode_u64_counted)
        .unwrap();
    let limits = FlowCheckpointLimits::default();
    let image = source.capture_checkpoint_v2(&codecs, limits).unwrap();

    DECODER_CALLS.set(0);
    let _valid_control =
        FlowRuntime::restore_checkpoint_v2(image.clone(), &codecs, limits).unwrap();
    assert!(DECODER_CALLS.get() > 0);
    DECODER_CALLS.set(0);
    let mut missing_flag = image.clone();
    missing_flag.resource_grant_flags.clear();
    assert!(matches!(
        FlowRuntime::restore_checkpoint_v2(missing_flag, &codecs, limits),
        Err(FlowCheckpointError::InvalidState(_))
    ));
    assert_eq!(DECODER_CALLS.get(), 0);

    let mut duplicate_flag = source.capture_checkpoint_v2(&codecs, limits).unwrap();
    duplicate_flag
        .resource_grant_flags
        .push(duplicate_flag.resource_grant_flags[0]);
    assert!(matches!(
        FlowRuntime::restore_checkpoint_v2(duplicate_flag, &codecs, limits),
        Err(FlowCheckpointError::InvalidState(_))
    ));
    assert_eq!(DECODER_CALLS.get(), 0);

    let mut wrong_frontier = source.capture_checkpoint_v2(&codecs, limits).unwrap();
    wrong_frontier.started_frontier = Some(tick(99));
    assert!(matches!(
        FlowRuntime::restore_checkpoint_v2(wrong_frontier, &codecs, limits),
        Err(FlowCheckpointError::InvalidState(_))
    ));
    assert_eq!(DECODER_CALLS.get(), 0);

    let mut orphan_gate = image;
    orphan_gate.commands[0].0 = kairo_ecs_types::EventId::new(u64::MAX, u32::MAX);
    assert!(matches!(
        FlowRuntime::restore_checkpoint_v2(orphan_gate, &codecs, limits),
        Err(FlowCheckpointError::InvalidState(_))
    ));
    assert_eq!(DECODER_CALLS.get(), 0);

    let mut stale_resource = source.capture_checkpoint_v2(&codecs, limits).unwrap();
    // Only the command payload is corrupted; all live resource/flag rows stay valid.
    stale_resource.commands[0].1 =
        kairo_ecs_des::FlowCommandV2::GrantEnabled(stale_resource_id, false);
    assert!(matches!(
        FlowRuntime::restore_checkpoint_v2(stale_resource, &codecs, limits),
        Err(FlowCheckpointError::InvalidState(_))
    ));
    assert_eq!(DECODER_CALLS.get(), 0);

    let mut wrong_kind = source.capture_checkpoint_v2(&codecs, limits).unwrap();
    let event = wrong_kind.commands[0].0;
    let entry = wrong_kind
        .scheduler
        .entries
        .iter_mut()
        .find(|entry| entry.id == event)
        .unwrap();
    entry.request.kind = kairo_ecs_types::EventKind::Custom(99_999);
    assert!(matches!(
        FlowRuntime::restore_checkpoint_v2(wrong_kind, &codecs, limits),
        Err(FlowCheckpointError::InvalidState(_))
    ));
    assert_eq!(DECODER_CALLS.get(), 0);

    let mut wrong_entity = source.capture_checkpoint_v2(&codecs, limits).unwrap();
    let event = wrong_entity.commands[0].0;
    let entry = wrong_entity
        .scheduler
        .entries
        .iter_mut()
        .find(|entry| entry.id == event)
        .unwrap();
    entry.request.entity = Some(owner);
    assert!(matches!(
        FlowRuntime::restore_checkpoint_v2(wrong_entity, &codecs, limits),
        Err(FlowCheckpointError::InvalidState(_))
    ));
    assert_eq!(DECODER_CALLS.get(), 0);

    let mut duplicate_boundary = source.capture_checkpoint_v2(&codecs, limits).unwrap();
    let event = duplicate_boundary.commands[1].0;
    duplicate_boundary.commands[1].1 = kairo_ecs_des::FlowCommandV2::GrantEnabled(resource, true);
    let entry = duplicate_boundary
        .scheduler
        .entries
        .iter_mut()
        .find(|entry| entry.id == event)
        .unwrap();
    entry.request.at = tick(8);
    assert!(matches!(
        FlowRuntime::restore_checkpoint_v2(duplicate_boundary, &codecs, limits),
        Err(FlowCheckpointError::InvalidState(_))
    ));
    assert_eq!(DECODER_CALLS.get(), 0);

    let mut remove_conflict = source.capture_checkpoint_v2(&codecs, limits).unwrap();
    remove_conflict.commands[1].1 = kairo_ecs_des::FlowCommandV2::Remove(resource);
    assert!(matches!(
        FlowRuntime::restore_checkpoint_v2(remove_conflict, &codecs, limits),
        Err(FlowCheckpointError::InvalidState(_))
    ));
    assert_eq!(DECODER_CALLS.get(), 0);
}

#[test]
fn native_v2_respects_resource_and_pending_operation_limits() {
    let mut source = FlowRuntime::new();
    let resource = source.create_resource(1).unwrap();
    source
        .schedule_resource_grant_enabled(resource, false, tick(5), 0)
        .unwrap();
    let codecs = FlowCheckpointCodecs::new();
    let limits = FlowCheckpointLimits {
        max_resources: 1,
        max_pending_operations: 1,
        ..Default::default()
    };
    let image = source.capture_checkpoint_v2(&codecs, limits).unwrap();
    assert!(matches!(
        source.capture_checkpoint_v2(
            &codecs,
            FlowCheckpointLimits {
                max_resources: 0,
                ..limits
            }
        ),
        Err(FlowCheckpointError::LimitExceeded(_))
    ));
    assert!(matches!(
        source.capture_checkpoint_v2(
            &codecs,
            FlowCheckpointLimits {
                max_pending_operations: 0,
                ..limits
            }
        ),
        Err(FlowCheckpointError::LimitExceeded(_))
    ));
    assert!(FlowRuntime::restore_checkpoint_v2(image, &codecs, limits).is_ok());
}

#[test]
fn native_v2_preflights_command_registration_key_component_sparse_and_payload_limits() {
    let mut source = FlowRuntime::new();
    let owner = source.spawn_actor().unwrap();
    let resource = source.create_resource(1).unwrap();
    let work = source
        .create_work(owner, SimDuration::from_ticks(10), "ctx", 7_u64)
        .unwrap();
    source
        .acquire(resource)
        .owner(owner)
        .timed_work(work)
        .submit()
        .unwrap();
    source
        .schedule_resource_grant_enabled(resource, false, tick(8), 0)
        .unwrap();
    let mut codecs = FlowCheckpointCodecs::new();
    codecs
        .register_context("ctx", 1, encode_u64, decode_u64_counted)
        .unwrap();
    let limits = FlowCheckpointLimits::default();
    let image = source.capture_checkpoint_v2(&codecs, limits).unwrap();
    let shapes = [
        image
            .builtins
            .capacities
            .as_ref()
            .map(|s| (s.rows.len(), s.sparse_slots)),
        image
            .builtins
            .queues
            .as_ref()
            .map(|s| (s.rows.len(), s.sparse_slots)),
        image
            .builtins
            .requests
            .as_ref()
            .map(|s| (s.rows.len(), s.sparse_slots)),
        image
            .builtins
            .deadlines
            .as_ref()
            .map(|s| (s.rows.len(), s.sparse_slots)),
        image
            .builtins
            .preempting
            .as_ref()
            .map(|s| (s.rows.len(), s.sparse_slots)),
        image
            .builtins
            .allocations
            .as_ref()
            .map(|s| (s.rows.len(), s.sparse_slots)),
        image
            .builtins
            .work_specs
            .as_ref()
            .map(|s| (s.rows.len(), s.sparse_slots)),
        image
            .builtins
            .work_roles
            .as_ref()
            .map(|s| (s.rows.len(), s.sparse_slots)),
        image
            .builtins
            .work_progress
            .as_ref()
            .map(|s| (s.rows.len(), s.sparse_slots)),
    ];
    assert!(image.restart_stores.is_empty());
    assert!(
        image.handlers.is_empty() && image.continuations.is_empty() && image.domains.is_empty()
    );
    let exact = FlowCheckpointLimits {
        max_commands: image.commands.len(),
        max_pending_operations: 1,
        max_registrations: image.contexts.len() + image.context_stores.len(),
        max_key_bytes: image
            .contexts
            .iter()
            .map(|c| c.runtime_key.len() + c.codec_key.len())
            .sum::<usize>()
            + image
                .work_registrations
                .iter()
                .map(|w| w.context_runtime_key.len() + w.context_codec_key.len())
                .sum::<usize>()
            + image
                .context_stores
                .iter()
                .map(|c| c.codec_key.len())
                .sum::<usize>(),
        max_component_rows: shapes.iter().flatten().map(|s| s.0).sum::<usize>()
            + image
                .context_stores
                .iter()
                .map(|s| s.rows.len())
                .sum::<usize>(),
        max_sparse_slots: shapes.iter().flatten().map(|s| s.1).sum::<usize>()
            + image
                .context_stores
                .iter()
                .map(|s| s.sparse_slots)
                .sum::<usize>(),
        max_payload_bytes: image
            .context_stores
            .iter()
            .flat_map(|s| &s.rows)
            .map(|(_, p)| p.len())
            .sum(),
        ..limits
    };
    DECODER_CALLS.set(0);
    FlowRuntime::restore_checkpoint_v2(image.clone(), &codecs, exact).unwrap();
    assert_eq!(DECODER_CALLS.get(), 1);
    assert_eq!(source.capture_checkpoint_v2(&codecs, exact).unwrap(), image);
    for bounded in [
        FlowCheckpointLimits {
            max_commands: exact.max_commands - 1,
            ..exact
        },
        FlowCheckpointLimits {
            max_pending_operations: 0,
            ..exact
        },
        FlowCheckpointLimits {
            max_registrations: exact.max_registrations - 1,
            ..exact
        },
        FlowCheckpointLimits {
            max_key_bytes: exact.max_key_bytes - 1,
            ..exact
        },
        FlowCheckpointLimits {
            max_component_rows: exact.max_component_rows - 1,
            ..exact
        },
        FlowCheckpointLimits {
            max_sparse_slots: exact.max_sparse_slots - 1,
            ..exact
        },
        FlowCheckpointLimits {
            max_payload_bytes: exact.max_payload_bytes - 1,
            ..exact
        },
    ] {
        DECODER_CALLS.set(0);
        assert!(matches!(
            FlowRuntime::restore_checkpoint_v2(image.clone(), &codecs, bounded),
            Err(FlowCheckpointError::LimitExceeded(_))
        ));
        assert_eq!(DECODER_CALLS.get(), 0);
    }
}

#[test]
fn restored_completion_first_tick_keeps_pending_disable_boundary() {
    let mut source = FlowRuntime::new();
    let resource = source.create_resource(1).unwrap();
    let owner = source.spawn_actor().unwrap();
    let first_work = source
        .create_work(owner, SimDuration::from_ticks(3), "unit", ())
        .unwrap();
    let second_work = source
        .create_work(owner, SimDuration::from_ticks(4), "unit", ())
        .unwrap();
    let first_request = source
        .acquire(resource)
        .owner(owner)
        .timed_work(first_work)
        .submit()
        .unwrap();
    let second_request = source
        .acquire(resource)
        .owner(owner)
        .timed_work(second_work)
        .at(tick(1))
        .submit()
        .unwrap();
    let disable_event = source
        .schedule_resource_grant_enabled(resource, false, tick(3), i32::MAX)
        .unwrap();
    source
        .schedule_resource_grant_enabled(resource, true, tick(5), i32::MAX)
        .unwrap();
    source.run_for(2).unwrap();
    let completion = source.step().unwrap().unwrap();
    assert_eq!(completion.at, tick(3));
    assert_ne!(completion.event, disable_event);
    assert!(completion.records.iter().any(|record| {
        record.request == first_request
            && record.transition == LifecycleTransition::Completed
            && record.at == tick(3)
    }));

    let mut codecs = FlowCheckpointCodecs::new();
    codecs
        .register_context("unit", 1, encode_unit, decode_unit)
        .unwrap();
    let limits = FlowCheckpointLimits::default();
    let image = source.capture_checkpoint_v2(&codecs, limits).unwrap();
    assert_eq!(image.started_frontier, Some(tick(3)));
    assert!(image.commands.iter().any(|(event, command)| {
        *event == disable_event
            && *command == kairo_ecs_des::FlowCommandV2::GrantEnabled(resource, false)
    }));
    assert!(image.scheduler.entries.iter().any(|entry| {
        entry.id == disable_event
            && entry.live
            && entry.request.at == tick(3)
            && entry.request.priority == i32::MAX
    }));
    let mut restored = FlowRuntime::restore_checkpoint_v2(image, &codecs, limits).unwrap();
    let expected_tail = source.run_for(20).unwrap();
    let actual_tail = restored.run_for(20).unwrap();
    assert_eq!(actual_tail, expected_tail);
    let records: Vec<_> = actual_tail
        .dispatches
        .iter()
        .flat_map(|dispatch| &dispatch.records)
        .collect();
    assert!(!records.iter().any(|record| {
        record.request == first_request && record.transition == LifecycleTransition::Completed
    }));
    assert!(!records.iter().any(|record| {
        record.request == second_request
            && record.transition == LifecycleTransition::Granted
            && record.at == tick(3)
    }));
    assert!(records.iter().any(|record| {
        record.request == second_request
            && record.transition == LifecycleTransition::Granted
            && record.at == tick(5)
    }));
}

#[test]
fn suspended_work_and_pending_enable_continue_identically_after_restore() {
    let mut source = FlowRuntime::new();
    let resource = source.create_resource(1).unwrap();
    let low_owner = source.spawn_actor().unwrap();
    let urgent_owner = source.spawn_actor().unwrap();
    let low_work = source
        .create_work(low_owner, SimDuration::from_ticks(10), "unit", ())
        .unwrap();
    let urgent_work = source
        .create_work(urgent_owner, SimDuration::from_ticks(2), "unit", ())
        .unwrap();
    let low = source
        .acquire(resource)
        .owner(low_owner)
        .timed_work(low_work)
        .priority(10)
        .preemptible(kairo_ecs_des::PreemptionStrategy::Suspend)
        .submit()
        .unwrap();
    let urgent = source
        .acquire(resource)
        .owner(urgent_owner)
        .timed_work(urgent_work)
        .at(tick(4))
        .priority(1)
        .can_preempt(true)
        .submit()
        .unwrap();
    let enable_event = source
        .schedule_resource_grant_enabled(resource, true, tick(8), 0)
        .unwrap();
    let disable_event = source
        .schedule_resource_grant_enabled(resource, false, tick(6), i32::MAX)
        .unwrap();
    let disable_dispatch = (0..32)
        .find_map(|_| {
            let dispatch = source.step().unwrap()?;
            (dispatch.event == disable_event).then_some(dispatch)
        })
        .expect("the exact pending disable boundary is dispatched");
    assert_eq!(disable_dispatch.at, tick(6));
    assert_eq!(source.resource_grant_enabled(resource), Ok(false));
    assert_eq!(
        source.request(low).unwrap().state,
        kairo_ecs_des::RequestState::Suspended
    );
    assert_eq!(
        source.request(urgent).unwrap().state,
        kairo_ecs_des::RequestState::Completed
    );
    assert_eq!(
        source.work_progress(low_work).unwrap().useful_elapsed,
        SimDuration::from_ticks(4)
    );
    assert_eq!(
        source.work_progress(low_work).unwrap().remaining,
        SimDuration::from_ticks(6)
    );

    let mut codecs = FlowCheckpointCodecs::new();
    codecs
        .register_context("unit", 1, encode_unit, decode_unit)
        .unwrap();
    let limits = FlowCheckpointLimits::default();
    let image = source.capture_checkpoint_v2(&codecs, limits).unwrap();
    assert_eq!(image.started_frontier, Some(tick(6)));
    assert!(image.commands.iter().any(|(id, command)| {
        *id == enable_event
            && *command == kairo_ecs_des::FlowCommandV2::GrantEnabled(resource, true)
    }));
    let mut restored = FlowRuntime::restore_checkpoint_v2(image, &codecs, limits).unwrap();
    let expected = source.run_for(20).unwrap();
    let actual = restored.run_for(20).unwrap();
    assert_eq!(actual, expected);
    assert_eq!(restored.request(low).unwrap(), source.request(low).unwrap());
    assert_eq!(
        restored.request(urgent).unwrap(),
        source.request(urgent).unwrap()
    );
    assert_eq!(
        restored.work_progress(low_work).unwrap(),
        source.work_progress(low_work).unwrap()
    );
    let records: Vec<_> = actual.dispatches.iter().flat_map(|d| &d.records).collect();
    assert!(!records.iter().any(|r| r.request == low
        && r.transition == LifecycleTransition::Resumed
        && r.at == tick(6)));
    assert!(records.iter().any(|r| r.request == low
        && r.transition == LifecycleTransition::Resumed
        && r.at == tick(8)));
    assert!(records.iter().any(|r| r.request == low
        && r.transition == LifecycleTransition::Completed
        && r.at == tick(14)));
}

#[test]
fn precommit_budget_halt_preserves_pending_gate_and_full_native_image() {
    let config = kairo_ecs_des::FlowConfig {
        max_same_tick_flow_transitions: std::num::NonZeroU64::new(1).unwrap(),
    };
    let mut source = FlowRuntime::with_config(config);
    let owner = source.spawn_actor().unwrap();
    let resource = source.create_resource(1).unwrap();
    let work = source
        .create_work(owner, SimDuration::from_ticks(10), "unit", ())
        .unwrap();
    source
        .acquire(resource)
        .owner(owner)
        .timed_work(work)
        .submit()
        .unwrap();
    let gate = source
        .schedule_resource_grant_enabled(resource, false, tick(8), 0)
        .unwrap();
    assert!(matches!(
        source.step(),
        Err(kairo_ecs_des::FlowError::SameTickBudgetExceeded { at_ticks: 0, .. })
    ));
    assert_eq!(source.budget_snapshot().tick, None);
    assert_eq!(source.budget_snapshot().consumed, 0);
    let mut codecs = FlowCheckpointCodecs::new();
    codecs
        .register_context("unit", 1, encode_unit, decode_unit)
        .unwrap();
    let limits = FlowCheckpointLimits::default();
    let image = source.capture_checkpoint_v2(&codecs, limits).unwrap();
    assert_eq!(image.started_frontier, None);
    assert!(image.commands.iter().any(|(id, command)| {
        *id == gate && *command == kairo_ecs_des::FlowCommandV2::GrantEnabled(resource, false)
    }));
    let restored = FlowRuntime::restore_checkpoint_v2(image.clone(), &codecs, limits).unwrap();
    assert_eq!(restored.budget_snapshot(), source.budget_snapshot());
    assert_eq!(
        restored.capture_checkpoint_v2(&codecs, limits).unwrap(),
        image
    );
}

fn encode_u64(value: &u64, remaining: usize) -> Result<Vec<u8>, FlowCheckpointCodecError> {
    if remaining < 8 {
        return Err(FlowCheckpointCodecError("payload limit".into()));
    }
    Ok(value.to_le_bytes().to_vec())
}

fn decode_u64_counted(
    bytes: &[u8],
    _: &FlowCheckpointRebindV1,
) -> Result<u64, FlowCheckpointCodecError> {
    DECODER_CALLS.set(DECODER_CALLS.get() + 1);
    let bytes: [u8; 8] = bytes
        .try_into()
        .map_err(|_| FlowCheckpointCodecError("invalid u64 payload".into()))?;
    Ok(u64::from_le_bytes(bytes))
}

fn encode_unit(_: &(), remaining: usize) -> Result<Vec<u8>, FlowCheckpointCodecError> {
    if remaining == 0 {
        return Err(FlowCheckpointCodecError("payload limit".into()));
    }
    Ok(Vec::new())
}

fn decode_unit(_: &[u8], _: &FlowCheckpointRebindV1) -> Result<(), FlowCheckpointCodecError> {
    Ok(())
}

#[test]
fn pending_release_despawn_and_gate_share_one_aggregate_limit() {
    let mut source = FlowRuntime::new();
    let owner = source.spawn_actor().unwrap();
    let departing = source.spawn_actor().unwrap();
    let resource = source.create_resource(1).unwrap();
    let request = source.acquire(resource).owner(owner).submit().unwrap();
    source.step().unwrap().unwrap();
    source
        .release(source.request(request).unwrap().lease.unwrap(), tick(5))
        .unwrap();
    source
        .despawn_actor_at_with_scheduler_priority(departing, tick(7), 0)
        .unwrap();
    let codecs = FlowCheckpointCodecs::new();
    let legacy_limit = FlowCheckpointLimits {
        max_pending_operations: 2,
        ..Default::default()
    };
    let legacy = source.capture_checkpoint(&codecs, legacy_limit).unwrap();
    FlowRuntime::restore_checkpoint(legacy.clone(), &codecs, legacy_limit).unwrap();
    assert!(matches!(
        FlowRuntime::restore_checkpoint(
            legacy,
            &codecs,
            FlowCheckpointLimits {
                max_pending_operations: 1,
                ..legacy_limit
            }
        ),
        Err(FlowCheckpointError::LimitExceeded("pending operations"))
    ));
    source
        .schedule_resource_grant_enabled(resource, false, tick(6), 0)
        .unwrap();
    let exact = FlowCheckpointLimits {
        max_pending_operations: 3,
        ..legacy_limit
    };
    let image = source.capture_checkpoint_v2(&codecs, exact).unwrap();
    FlowRuntime::restore_checkpoint_v2(image.clone(), &codecs, exact).unwrap();
    assert!(matches!(
        FlowRuntime::restore_checkpoint_v2(image, &codecs, legacy_limit),
        Err(FlowCheckpointError::LimitExceeded("pending operations"))
    ));
    assert!(matches!(
        source.capture_checkpoint_v2(&codecs, legacy_limit),
        Err(FlowCheckpointError::LimitExceeded("pending operations"))
    ));
}
