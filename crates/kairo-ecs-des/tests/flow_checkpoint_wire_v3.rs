use kairo_ecs_des::{
    FlowAllocationMode, FlowCheckpointCodecError, FlowCheckpointCodecs, FlowCheckpointError,
    FlowCheckpointLimits, FlowRuntime, RequestState, WorkState, FLOW_CHECKPOINT_VERSION_V3,
};
use kairo_ecs_types::{EntityId, EventId, SimDuration, SimTime};
use std::cell::Cell;
use std::num::NonZeroU64;

thread_local! {
    static CODEC_CALLS: Cell<usize> = const { Cell::new(0) };
    static ENCODE_CALLS: Cell<usize> = const { Cell::new(0) };
}

fn t(value: u128) -> SimTime {
    SimTime::from_ticks(value)
}

fn encode_u64(value: &u64, remaining: usize) -> Result<Vec<u8>, FlowCheckpointCodecError> {
    ENCODE_CALLS.with(|count| count.set(count.get() + 1));
    if remaining < 8 {
        return Err(FlowCheckpointCodecError("payload limit".into()));
    }
    Ok(value.to_le_bytes().to_vec())
}

fn decode_u64(
    bytes: &[u8],
    _: &kairo_ecs_des::FlowCheckpointRebindV1,
) -> Result<u64, FlowCheckpointCodecError> {
    CODEC_CALLS.with(|count| count.set(count.get() + 1));
    let bytes: [u8; 8] = bytes
        .try_into()
        .map_err(|_| FlowCheckpointCodecError("bad u64 payload".into()))?;
    Ok(u64::from_le_bytes(bytes))
}

fn template_context(value: &u64) -> u64 {
    *value
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
        .register_restart_factory("template", "factory", template_context)
        .unwrap();
    codecs
}

fn rich_deferred_seed() -> (
    FlowRuntime,
    FlowCheckpointCodecs,
    kairo_ecs_des::ResourceId,
    kairo_ecs_des::RequestId,
    EventId,
    kairo_ecs_des::WorkId,
) {
    let mut source = FlowRuntime::new();
    let eager_resource = source.create_resource(1).unwrap();
    let resource = source
        .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
        .unwrap();
    let eager_owner = source.spawn_actor().unwrap();
    let owner = source.spawn_actor().unwrap();
    let eager_work = source
        .create_work(
            eager_owner,
            SimDuration::from_ticks(40),
            "ctx",
            0x0102_0304_u64,
        )
        .unwrap();
    let work = source
        .create_restartable_work(
            owner,
            SimDuration::from_ticks(6),
            "ctx",
            0x1020_3040_5060_7080_u64,
            template_context,
        )
        .unwrap();
    let eager_request = source
        .acquire(eager_resource)
        .owner(eager_owner)
        .timed_work(eager_work)
        .submit()
        .unwrap();
    let request = source
        .acquire(resource)
        .owner(owner)
        .timed_work(work)
        .submit()
        .unwrap();
    let codecs = codecs();
    let mut reached_seed_state = false;
    for _ in 0..32 {
        let dispatch = source.step().unwrap().expect("seed scheduler event");
        if source.request(eager_request).unwrap().state == RequestState::Active
            && source.request(request).unwrap().state == RequestState::Queued
        {
            assert!(dispatch.at >= t(0));
            reached_seed_state = true;
            break;
        }
    }
    assert!(
        reached_seed_state,
        "seed did not reach active/queued state within bound"
    );
    let lease = source.request(eager_request).unwrap().lease.unwrap();
    source.release(lease, t(20)).unwrap();
    source
        .despawn_actor_at_with_scheduler_priority(eager_owner, t(20), 0)
        .unwrap();
    let flush = source
        .schedule_allocation_flush(resource, t(20), 0)
        .unwrap();
    source
        .schedule_allocation_flush(resource, t(21), i32::MAX)
        .unwrap();
    source
        .schedule_resource_grant_enabled(resource, false, t(20), 1)
        .unwrap();
    source
        .schedule_resource_grant_enabled(resource, true, t(21), 1)
        .unwrap();
    (source, codecs, resource, request, flush, work)
}

fn rich_wire_seed() -> (
    FlowRuntime,
    FlowCheckpointCodecs,
    kairo_ecs_des::ResourceId,
    kairo_ecs_des::ResourceId,
    Vec<kairo_ecs_des::RequestId>,
    Vec<kairo_ecs_des::RequestId>,
    Vec<kairo_ecs_des::WorkId>,
) {
    let mut source = FlowRuntime::new();
    let eager = source.create_resource(2).unwrap();
    let deferred = source
        .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
        .unwrap();
    source
        .schedule_resource_grant_enabled(deferred, false, t(0), -10)
        .unwrap();
    let mut eager_requests = Vec::new();
    let mut requests = Vec::new();
    let mut all_requests = Vec::new();
    let mut works = Vec::new();
    for seed in [11_u64, 12] {
        let owner = source.spawn_actor().unwrap();
        let work = source
            .create_work(owner, SimDuration::from_ticks(40), "ctx", seed)
            .unwrap();
        let request = source
            .acquire(eager)
            .owner(owner)
            .timed_work(work)
            .submit()
            .unwrap();
        eager_requests.push(request);
        all_requests.push(request);
        works.push(work);
    }
    for index in 0..3 {
        let owner = source.spawn_actor().unwrap();
        let work = if index == 2 {
            source
                .create_restartable_work(
                    owner,
                    SimDuration::from_ticks(3),
                    "ctx",
                    0x8877_u64,
                    template_context,
                )
                .unwrap()
        } else {
            source
                .create_work(owner, SimDuration::from_ticks(3), "ctx", 21_u64 + index)
                .unwrap()
        };
        let request = source
            .acquire(deferred)
            .owner(owner)
            .timed_work(work)
            .submit()
            .unwrap();
        requests.push(request);
        all_requests.push(request);
        works.push(work);
    }
    let mut reached_rich_state = false;
    for _ in 0..32 {
        let dispatch = source.step().unwrap().expect("rich seed scheduler event");
        if source.resource(eager).unwrap().active.len() == 2
            && source.resource(deferred).unwrap().queued.len() == 3
            && eager_requests
                .iter()
                .all(|id| source.request(*id).unwrap().state == RequestState::Active)
            && requests
                .iter()
                .all(|id| source.request(*id).unwrap().state == RequestState::Queued)
        {
            assert!(dispatch.at >= t(0));
            reached_rich_state = true;
            break;
        }
    }
    assert!(
        reached_rich_state,
        "rich seed did not reach required allocation state within bound"
    );
    assert_eq!(source.resource(eager).unwrap().active.len(), 2);
    assert_eq!(source.resource(deferred).unwrap().queued.len(), 3);
    source
        .schedule_resource_grant_enabled(deferred, true, t(20), -10)
        .unwrap();
    source
        .schedule_allocation_flush(deferred, t(20), i32::MAX)
        .unwrap();
    source
        .schedule_allocation_flush(deferred, t(23), i32::MAX)
        .unwrap();
    source
        .schedule_allocation_flush(deferred, t(26), i32::MAX)
        .unwrap();
    (
        source,
        codecs(),
        eager,
        deferred,
        requests,
        all_requests,
        works,
    )
}

#[test]
fn wire_v3_rich_mixed_modes_queues_allocations_restore_suffix() {
    let (mut source, codecs, eager, deferred, requests, all_requests, works) = rich_wire_seed();
    assert_eq!(source.resource(eager).unwrap().allocations.len(), 2);
    assert_eq!(source.resource(deferred).unwrap().queued.len(), 3);
    let image = source
        .capture_checkpoint_v3(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    let queued_dto_keys = image
        .builtins
        .requests
        .as_ref()
        .expect("request store")
        .rows
        .iter()
        .filter(|(_, request)| {
            request.resource == deferred && request.state == RequestState::Queued
        })
        .count();
    let active_dto_allocations = image
        .builtins
        .requests
        .as_ref()
        .expect("request store")
        .rows
        .iter()
        .filter(|(_, request)| request.state == RequestState::Active && request.lease.is_some())
        .count();
    assert!(
        queued_dto_keys >= 2,
        "DTO carries at least two actual queued request keys"
    );
    assert!(
        active_dto_allocations >= 2,
        "DTO carries at least two actual active allocation entries"
    );
    assert!(image
        .builtins
        .queues
        .as_ref()
        .is_some_and(|store| !store.rows.is_empty()));
    assert!(image
        .builtins
        .allocations
        .as_ref()
        .is_some_and(|store| store.rows.iter().any(|(id, _)| *id == eager.entity_id())));
    let request_store = image.builtins.requests.as_ref().expect("request store");
    assert!(
        request_store
            .rows
            .iter()
            .filter(|(_, request)| request.resource == deferred
                && request.state == RequestState::Queued)
            .count()
            >= 2,
        "DTO retains at least two queued keys"
    );
    assert!(
        request_store
            .rows
            .iter()
            .filter(|(_, request)| request.state == RequestState::Active && request.lease.is_some())
            .count()
            >= 2,
        "DTO retains at least two active allocation requests"
    );
    assert!(image
        .context_stores
        .iter()
        .any(|store| store.rows.iter().any(|(_, bytes)| !bytes.is_empty())));
    assert!(!image.restart_store_manifest.is_empty());
    assert!(image
        .restart_stores
        .iter()
        .any(|store| store.rows.iter().any(|(_, _, bytes)| !bytes.is_empty())));
    assert!(image
        .allocation_modes
        .iter()
        .any(|(_, mode)| *mode == FlowAllocationMode::Eager));
    assert!(image
        .allocation_modes
        .iter()
        .any(|(id, mode)| *id == deferred && *mode == FlowAllocationMode::Deferred));
    assert!(image
        .resource_grant_flags
        .iter()
        .any(|(id, enabled)| *id == deferred && !enabled));
    let future_gates = image.commands.iter().filter(|(_, command)| matches!(command, kairo_ecs_des::FlowCommandV3::GrantEnabled(id, true) if *id == deferred)).count();
    let future_flushes = image.commands.iter().filter(|(_, command)| matches!(command, kairo_ecs_des::FlowCommandV3::AllocationFlush(id) if *id == deferred)).count();
    assert_eq!(future_gates, 1);
    assert_eq!(future_flushes, 3);
    for (command, tick) in [
        (
            kairo_ecs_des::FlowCommandV3::GrantEnabled(deferred, true),
            20_u128,
        ),
        (kairo_ecs_des::FlowCommandV3::AllocationFlush(deferred), 20),
        (kairo_ecs_des::FlowCommandV3::AllocationFlush(deferred), 23),
        (kairo_ecs_des::FlowCommandV3::AllocationFlush(deferred), 26),
    ] {
        let event =
            image.commands.iter().find_map(|(event, row)| {
                (*row == command
                    && image.scheduler.entries.iter().any(|entry| {
                        entry.id == *event && entry.request.at == t(tick) && entry.live
                    }))
                .then_some(*event)
            });
        assert!(
            event.is_some(),
            "command remains scheduled at literal tick {tick}"
        );
    }
    let bytes = image
        .encode_wire_v3(kairo_ecs_des::FlowCheckpointWireLimits::default())
        .unwrap();
    let decoded = kairo_ecs_des::FlowCheckpointV3::decode_wire_v3(
        &bytes,
        kairo_ecs_des::FlowCheckpointWireLimits::default(),
    )
    .unwrap();
    assert_eq!(decoded, image);
    assert_eq!(
        decoded
            .encode_wire_v3(kairo_ecs_des::FlowCheckpointWireLimits::default())
            .unwrap(),
        bytes
    );
    let mut restored =
        FlowRuntime::restore_checkpoint_v3(decoded, &codecs, FlowCheckpointLimits::default())
            .unwrap();
    assert_eq!(
        restored
            .capture_checkpoint_v3(&codecs, FlowCheckpointLimits::default())
            .unwrap(),
        image
    );
    let source_tail = source.run_for(64).unwrap();
    let restored_tail = restored.run_for(64).unwrap();
    assert_eq!(source_tail, restored_tail);
    let records = source_tail
        .dispatches
        .iter()
        .flat_map(|dispatch| &dispatch.records)
        .collect::<Vec<_>>();
    let actual_deferred_lifecycle = records
        .iter()
        .filter(|record| {
            requests.contains(&record.request)
                && matches!(
                    record.transition,
                    kairo_ecs_des::LifecycleTransition::Granted
                        | kairo_ecs_des::LifecycleTransition::Completed
                )
        })
        .map(|record| (record.request, record.at.ticks(), record.transition))
        .collect::<Vec<_>>();
    assert_eq!(
        actual_deferred_lifecycle,
        vec![
            (requests[0], 20, kairo_ecs_des::LifecycleTransition::Granted),
            (
                requests[0],
                23,
                kairo_ecs_des::LifecycleTransition::Completed
            ),
            (requests[1], 23, kairo_ecs_des::LifecycleTransition::Granted),
            (
                requests[1],
                26,
                kairo_ecs_des::LifecycleTransition::Completed
            ),
            (requests[2], 26, kairo_ecs_des::LifecycleTransition::Granted),
            (
                requests[2],
                29,
                kairo_ecs_des::LifecycleTransition::Completed
            ),
        ],
    );
    assert!(records
        .iter()
        .any(|record| record.snapshot.work == Some(works[0])
            && record.at == t(40)
            && record.transition == kairo_ecs_des::LifecycleTransition::Completed));
    assert!(records
        .iter()
        .any(|record| record.snapshot.work == Some(works[1])
            && record.at == t(40)
            && record.transition == kairo_ecs_des::LifecycleTransition::Completed));
    assert!(!source_tail.budget_exhausted);
    assert!(!restored_tail.budget_exhausted);
    assert_eq!(source.budget_snapshot().scheduler.pending_events, 0);
    assert_eq!(restored.budget_snapshot().scheduler.pending_events, 0);
    for (request, work) in all_requests.iter().zip(&works) {
        assert_eq!(
            source.request(*request).unwrap(),
            restored.request(*request).unwrap()
        );
        let source_progress = source.work_progress(*work).unwrap();
        let restored_progress = restored.work_progress(*work).unwrap();
        assert_eq!(source_progress, restored_progress);
        assert_eq!(source_progress.state, WorkState::Completed);
        assert_eq!(
            source_progress.useful_elapsed,
            source_progress.original_duration
        );
        assert_eq!(source_progress.remaining, SimDuration::ZERO);
        assert_eq!(
            source.request(*request).unwrap().state,
            RequestState::Completed
        );
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ReadinessContext {
    resource: kairo_ecs_des::ResourceId,
    owner: EntityId,
    work: kairo_ecs_des::WorkId,
    calls: u32,
    ready_at: Option<SimTime>,
    emitted: bool,
}

fn encode_readiness(
    value: &ReadinessContext,
    remaining: usize,
) -> Result<Vec<u8>, FlowCheckpointCodecError> {
    ENCODE_CALLS.with(|count| count.set(count.get() + 1));
    if remaining < 58 {
        return Err(FlowCheckpointCodecError("payload limit".into()));
    }
    let mut bytes = Vec::with_capacity(58);
    for id in [
        value.resource.entity_id(),
        value.owner,
        value.work.entity_id(),
    ] {
        bytes.extend_from_slice(&id.index.to_le_bytes());
        bytes.extend_from_slice(&id.generation.to_le_bytes());
    }
    bytes.extend_from_slice(&value.calls.to_le_bytes());
    bytes.push(u8::from(value.ready_at.is_some()));
    bytes.extend_from_slice(
        &value
            .ready_at
            .unwrap_or(SimTime::ZERO)
            .ticks()
            .to_le_bytes(),
    );
    bytes.push(u8::from(value.emitted));
    Ok(bytes)
}

fn decode_readiness(
    bytes: &[u8],
    rebind: &kairo_ecs_des::FlowCheckpointRebindV1,
) -> Result<ReadinessContext, FlowCheckpointCodecError> {
    CODEC_CALLS.with(|count| count.set(count.get() + 1));
    if bytes.len() != 58 || bytes[40] > 1 || bytes[57] > 1 {
        return Err(FlowCheckpointCodecError("bad readiness payload".into()));
    }
    let entity = |offset: usize| {
        EntityId::new(
            u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap()),
            u32::from_le_bytes(bytes[offset + 8..offset + 12].try_into().unwrap()),
        )
    };
    let ticks = u128::from_le_bytes(bytes[41..57].try_into().unwrap());
    Ok(ReadinessContext {
        resource: rebind.resolve_resource(entity(0))?,
        owner: rebind.resolve_actor(entity(12))?,
        work: rebind.resolve_work(entity(24))?,
        calls: u32::from_le_bytes(bytes[36..40].try_into().unwrap()),
        ready_at: (bytes[40] == 1).then(|| SimTime::from_ticks(ticks)),
        emitted: bytes[57] == 1,
    })
}

fn create_ready_work(
    context: &mut ReadinessContext,
    snapshot: &kairo_ecs_des::FlowCallbackSnapshot,
    sink: &mut kairo_ecs_des::FlowCommandSink,
) {
    if matches!(
        &snapshot.cause,
        kairo_ecs_des::FlowCallbackCause::Work {
            transition: kairo_ecs_des::LifecycleTransition::Completed,
            ..
        }
    ) {
        context.calls += 1;
        context.ready_at = Some(snapshot.delivery.at);
        context.emitted = sink
            .emit(kairo_ecs_des::FlowOwnedCommand::Acquire(
                kairo_ecs_des::FlowAcquireCommand {
                    resource: context.resource,
                    owner: context.owner,
                    work: Some(context.work),
                    at: snapshot.delivery.at,
                    priority_level: 0,
                    deadline: None,
                    scheduler_priority: 0,
                    timed: true,
                    can_preempt: false,
                    preemptible: None,
                },
            ))
            .is_ok();
    }
}

fn readiness_codecs() -> FlowCheckpointCodecs {
    let mut codecs = codecs();
    codecs
        .register_context("trigger", 1, encode_readiness, decode_readiness)
        .unwrap();
    codecs
        .register_work_continuations(
            "trigger",
            kairo_ecs_des::FlowContinuationCodeIds {
                on_complete: Some(kairo_ecs_des::FlowCallbackCodeV1 {
                    stable_id: "checkpoint.trigger-ready".into(),
                    version: 1,
                }),
                ..Default::default()
            },
            kairo_ecs_des::FlowContinuations {
                on_complete: Some(create_ready_work),
                ..Default::default()
            },
        )
        .unwrap();
    codecs
}

fn drain_through_checkpoint_event(
    runtime: &mut FlowRuntime,
    event: EventId,
    tick: SimTime,
) -> Vec<kairo_ecs_des::FlowDispatch> {
    let mut dispatches = Vec::new();
    for _ in 0..32 {
        let dispatch = runtime.step().unwrap().expect("scheduled checkpoint event");
        let reached = dispatch.event == event;
        assert_eq!(dispatch.at, tick, "event closure crossed its expected tick");
        dispatches.push(dispatch);
        if reached {
            return dispatches;
        }
    }
    panic!("event closure exceeded dispatch bound before {event:?}");
}

fn run_checkpointed_readiness_order(trigger_completion_first: bool) {
    let mut source = FlowRuntime::new();
    let deferred = source
        .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
        .unwrap();
    let trigger_resource = source.create_resource(1).unwrap();
    let owner_low = source.spawn_actor().unwrap();
    let owner_urgent = source.spawn_actor().unwrap();
    let owner_trigger = source.spawn_actor().unwrap();
    let owner_ready = source.spawn_actor().unwrap();
    let low_work = source
        .create_work(owner_low, SimDuration::from_ticks(10), "ctx", 10_u64)
        .unwrap();
    let urgent_work = source
        .create_work(owner_urgent, SimDuration::from_ticks(2), "ctx", 20_u64)
        .unwrap();
    let ready_work = source
        .create_work(owner_ready, SimDuration::from_ticks(1), "ctx", 30_u64)
        .unwrap();
    source
        .register_work_continuations(
            "trigger",
            kairo_ecs_des::FlowContinuations {
                on_complete: Some(create_ready_work),
                ..Default::default()
            },
        )
        .unwrap();
    let trigger_work = source
        .create_work(
            owner_trigger,
            SimDuration::from_ticks(2),
            "trigger",
            ReadinessContext {
                resource: deferred,
                owner: owner_ready,
                work: ready_work,
                calls: 0,
                ready_at: None,
                emitted: false,
            },
        )
        .unwrap();
    let low = source
        .acquire(deferred)
        .owner(owner_low)
        .timed_work(low_work)
        .at(t(1))
        .priority(10)
        .preemptible(kairo_ecs_des::PreemptionStrategy::Suspend)
        .submit()
        .unwrap();
    let initial_flush = source
        .schedule_allocation_flush(deferred, t(1), i32::MAX)
        .unwrap();
    let mut prefix = drain_through_checkpoint_event(&mut source, initial_flush, t(1));
    assert!(prefix.iter().any(|dispatch| {
        dispatch.records.iter().any(|record| {
            record.request == low
                && record.transition == kairo_ecs_des::LifecycleTransition::Granted
        })
    }));

    let (trigger_priority, urgent_priority, flush_priority) = if trigger_completion_first {
        (-10, -5, 0)
    } else {
        (0, -10, -5)
    };
    let urgent = source
        .acquire(deferred)
        .owner(owner_urgent)
        .timed_work(urgent_work)
        .at(t(4))
        .priority(1)
        .can_preempt(true)
        .scheduler_priority(urgent_priority)
        .submit()
        .unwrap();
    let trigger = source
        .acquire(trigger_resource)
        .owner(owner_trigger)
        .timed_work(trigger_work)
        .at(t(4))
        .scheduler_priority(trigger_priority)
        .submit()
        .unwrap();
    let preemption_flush = source
        .schedule_allocation_flush(deferred, t(4), flush_priority)
        .unwrap();
    let mut saw_urgent_grant = false;
    let mut saw_trigger_grant = false;
    let mut saw_preemption_flush = false;
    for _ in 0..32 {
        let dispatch = source.step().unwrap().expect("tick four event");
        assert_eq!(dispatch.at, t(4));
        saw_urgent_grant |= dispatch.records.iter().any(|record| {
            record.request == urgent
                && record.transition == kairo_ecs_des::LifecycleTransition::Granted
        });
        saw_trigger_grant |= dispatch.records.iter().any(|record| {
            record.request == trigger
                && record.transition == kairo_ecs_des::LifecycleTransition::Granted
        });
        saw_preemption_flush |= dispatch.event == preemption_flush;
        prefix.push(dispatch);
        if saw_urgent_grant && saw_trigger_grant && saw_preemption_flush {
            break;
        }
    }
    assert!(saw_urgent_grant && saw_trigger_grant && saw_preemption_flush);
    assert_eq!(source.request(low).unwrap().state, RequestState::Suspended);
    assert_eq!(
        source.work_progress(low_work).unwrap().useful_elapsed,
        SimDuration::from_ticks(3)
    );
    assert_eq!(
        source.work_progress(low_work).unwrap().remaining,
        SimDuration::from_ticks(7)
    );

    let flush_six = source
        .schedule_allocation_flush(deferred, t(6), i32::MAX)
        .unwrap();
    let (first_completion, trigger_done_at_cut, urgent_done_at_cut) = loop {
        let dispatch = source.step().unwrap().expect("tick six event");
        assert_eq!(dispatch.at, t(6), "first completion crossed tick six");
        let trigger_done = dispatch.records.iter().any(|record| {
            record.request == trigger
                && record.transition == kairo_ecs_des::LifecycleTransition::Completed
        });
        let urgent_done = dispatch.records.iter().any(|record| {
            record.request == urgent
                && record.transition == kairo_ecs_des::LifecycleTransition::Completed
        });
        if trigger_done || urgent_done {
            break (dispatch, trigger_done, urgent_done);
        }
    };
    assert_ne!(trigger_done_at_cut, urgent_done_at_cut);
    assert_eq!(
        source.request(trigger).unwrap().state == RequestState::Completed,
        trigger_done_at_cut
    );
    assert_eq!(
        source.request(urgent).unwrap().state == RequestState::Completed,
        urgent_done_at_cut
    );
    assert_eq!(trigger_done_at_cut, trigger_completion_first);
    prefix.push(first_completion.clone());
    let other_request = if trigger_done_at_cut { urgent } else { trigger };
    assert_eq!(
        source.request(other_request).unwrap().state,
        RequestState::Active
    );

    let codecs = readiness_codecs();
    let image = source
        .capture_checkpoint_v3(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    assert_eq!(image.started_frontier, Some(t(6)));
    // Preemption invalidates the execution token without cancelling its event.
    // The retained tick-eleven completion must survive capture and be a no-op.
    let stale_completions = image
        .commands
        .iter()
        .filter_map(|(event, command)| match command {
            kairo_ecs_des::FlowCommandV3::Completion(request, lease, revision, due)
                if *request == low && *due == t(11) =>
            {
                Some((*event, *lease, *revision))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(stale_completions.len(), 1);
    let (stale_completion, stale_lease, stale_revision) = stale_completions[0];
    assert_eq!(source.request(low).unwrap().state, RequestState::Suspended);
    assert_ne!(source.request(low).unwrap().lease, Some(stale_lease));
    assert!(image.scheduler.entries.iter().any(|entry| {
        entry.id == stale_completion
            && entry.live
            && entry.request.at == t(11)
            && entry.request.kind == kairo_ecs_types::EventKind::custom(4001)
    }));
    assert!(image
        .commands
        .iter()
        .any(|(event, command)| *event == flush_six
            && *command == kairo_ecs_des::FlowCommandV3::AllocationFlush(deferred)));
    let remaining_completions = image
        .commands
        .iter()
        .filter_map(|(event, command)| match command {
            kairo_ecs_des::FlowCommandV3::Completion(request, _, _, due)
                if *request == other_request && *due == t(6) =>
            {
                Some((*event, *request))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(remaining_completions.len(), 1);
    let remaining_completion = remaining_completions[0];
    assert_eq!(remaining_completion.1, other_request);
    let remaining_completion_row = image
        .scheduler
        .entries
        .iter()
        .find(|entry| entry.id == remaining_completion.0)
        .expect("pending completion has a scheduler row");
    assert!(remaining_completion_row.live);
    assert_eq!(remaining_completion_row.request.at, t(6));
    assert_eq!(
        remaining_completion_row.request.kind,
        kairo_ecs_types::EventKind::custom(4001)
    );
    assert!(!image
        .commands
        .iter()
        .any(|(event, _)| *event == first_completion.event));
    assert!(image
        .context_stores
        .iter()
        .any(|store| store.codec_key == "trigger"
            && store.rows.iter().any(|(_, bytes)| !bytes.is_empty())));
    let pending_notify = image.commands.iter().find_map(|(event, command)| {
        matches!(command, kairo_ecs_des::FlowCommandV3::Notify).then_some(*event)
    });
    if trigger_done_at_cut {
        let notify = pending_notify.expect("trigger completion awaits callback Notify");
        let row = image
            .scheduler
            .entries
            .iter()
            .find(|entry| entry.id == notify)
            .expect("pending callback Notify has a scheduler row");
        assert!(row.live);
        assert_eq!(row.request.at, t(6));
        assert_eq!(row.request.kind, kairo_ecs_types::EventKind::custom(4003));
        assert_eq!(
            source.request(trigger).unwrap().state,
            RequestState::Completed
        );
    }
    let source_context = source
        .work_context::<ReadinessContext>(trigger_work)
        .unwrap()
        .clone();
    assert_eq!(source_context.calls, 0);
    assert_eq!(source_context.ready_at, None);
    assert!(!source_context.emitted);
    let wire = image
        .encode_wire_v3(kairo_ecs_des::FlowCheckpointWireLimits::default())
        .unwrap();
    let decoded = kairo_ecs_des::FlowCheckpointV3::decode_wire_v3(
        &wire,
        kairo_ecs_des::FlowCheckpointWireLimits::default(),
    )
    .unwrap();
    assert_eq!(decoded, image);
    assert_eq!(
        decoded
            .encode_wire_v3(kairo_ecs_des::FlowCheckpointWireLimits::default())
            .unwrap(),
        wire
    );
    let mut restored =
        FlowRuntime::restore_checkpoint_v3(decoded, &codecs, FlowCheckpointLimits::default())
            .unwrap();
    assert_eq!(
        restored
            .capture_checkpoint_v3(&codecs, FlowCheckpointLimits::default())
            .unwrap(),
        image
    );
    assert_eq!(
        restored
            .work_context::<ReadinessContext>(trigger_work)
            .unwrap(),
        &source_context
    );

    let mut source_suffix = drain_through_checkpoint_event(&mut source, flush_six, t(6));
    let mut restored_suffix = drain_through_checkpoint_event(&mut restored, flush_six, t(6));
    assert_eq!(source_suffix, restored_suffix);
    let source_context = source
        .work_context::<ReadinessContext>(trigger_work)
        .unwrap();
    let restored_context = restored
        .work_context::<ReadinessContext>(trigger_work)
        .unwrap();
    assert_eq!(source_context.calls, 1);
    assert_eq!(source_context.ready_at, Some(t(6)));
    assert!(source_context.emitted);
    assert_eq!(restored_context, source_context);
    let ready_request = source.work(ready_work).unwrap().request.unwrap();
    assert_eq!(
        source.request(ready_request).unwrap().state,
        RequestState::Active
    );
    assert_eq!(
        restored.work(ready_work).unwrap().request,
        Some(ready_request)
    );
    assert!(source_suffix
        .iter()
        .flat_map(|dispatch| &dispatch.records)
        .any(|record| record.request == ready_request
            && record.at == t(6)
            && record.transition == kairo_ecs_des::LifecycleTransition::Granted));
    assert!(!source_suffix
        .iter()
        .flat_map(|dispatch| &dispatch.records)
        .any(|record| record.request == low
            && record.at == t(6)
            && record.transition == kairo_ecs_des::LifecycleTransition::Resumed));

    let mut saw_ready_complete = false;
    for _ in 0..32 {
        let dispatch = source.step().unwrap().expect("B completion event");
        assert!(dispatch.at <= t(7));
        saw_ready_complete |= dispatch.records.iter().any(|record| {
            record.request == ready_request
                && record.at == t(7)
                && record.transition == kairo_ecs_des::LifecycleTransition::Completed
        });
        source_suffix.push(dispatch);
        if saw_ready_complete {
            break;
        }
    }
    let mut saw_ready_complete_restored = false;
    for _ in 0..32 {
        let dispatch = restored
            .step()
            .unwrap()
            .expect("restored B completion event");
        assert!(dispatch.at <= t(7));
        saw_ready_complete_restored |= dispatch.records.iter().any(|record| {
            record.request == ready_request
                && record.at == t(7)
                && record.transition == kairo_ecs_des::LifecycleTransition::Completed
        });
        restored_suffix.push(dispatch);
        if saw_ready_complete_restored {
            break;
        }
    }
    assert!(saw_ready_complete && saw_ready_complete_restored);
    assert_eq!(source.request(low).unwrap().state, RequestState::Suspended);
    assert_eq!(
        restored.request(low).unwrap().state,
        RequestState::Suspended
    );
    let resume_flush = source
        .schedule_allocation_flush(deferred, t(7), i32::MAX)
        .unwrap();
    let restored_resume_flush = restored
        .schedule_allocation_flush(deferred, t(7), i32::MAX)
        .unwrap();
    assert_eq!(resume_flush, restored_resume_flush);
    let source_resume = drain_through_checkpoint_event(&mut source, resume_flush, t(7));
    let restored_resume = drain_through_checkpoint_event(&mut restored, resume_flush, t(7));
    assert_eq!(source_resume, restored_resume);
    assert!(source_resume
        .iter()
        .flat_map(|dispatch| &dispatch.records)
        .any(|record| record.request == low
            && record.at == t(7)
            && record.transition == kairo_ecs_des::LifecycleTransition::Resumed));
    source_suffix.extend(source_resume);
    restored_suffix.extend(restored_resume);

    for (runtime, suffix) in [
        (&mut source, &mut source_suffix),
        (&mut restored, &mut restored_suffix),
    ] {
        let mut saw_stale = false;
        for _ in 0..32 {
            let request_before = runtime.request(low).unwrap();
            let progress_before = runtime.work_progress(low_work).unwrap();
            let stored_progress_before = runtime
                .capture_checkpoint_v3(&codecs, FlowCheckpointLimits::default())
                .unwrap()
                .builtins
                .work_progress;
            let resource_before = runtime.resource(deferred).unwrap();
            let mode_before = runtime.resource_allocation_mode(deferred).unwrap();
            let dispatch = runtime.step().unwrap().expect("retained stale completion");
            assert!(dispatch.at <= t(11));
            if dispatch.event == stale_completion {
                assert_eq!(dispatch.at, t(11));
                assert_eq!(dispatch.error, None);
                assert!(dispatch.records.is_empty());
                assert!(dispatch.callback_batches.is_empty());
                assert_eq!(request_before.state, RequestState::Active);
                assert_ne!(request_before.lease, Some(stale_lease));
                assert_ne!(progress_before.execution_revision, stale_revision);
                assert_eq!(progress_before.completion_at, Some(t(14)));
                assert_eq!(progress_before.useful_elapsed, SimDuration::from_ticks(3));
                assert_eq!(progress_before.remaining, SimDuration::from_ticks(7));
                assert_eq!(runtime.request(low).unwrap(), request_before);
                // Public inspection derives elapsed work from the advanced clock;
                // the stored progress itself must remain unchanged by this event.
                let mut observed_at_eleven = progress_before.clone();
                observed_at_eleven.useful_elapsed = SimDuration::from_ticks(7);
                observed_at_eleven.remaining = SimDuration::from_ticks(3);
                observed_at_eleven.cumulative_busy = SimDuration::from_ticks(7);
                assert_eq!(runtime.work_progress(low_work).unwrap(), observed_at_eleven);
                assert_eq!(
                    runtime
                        .capture_checkpoint_v3(&codecs, FlowCheckpointLimits::default())
                        .unwrap()
                        .builtins
                        .work_progress,
                    stored_progress_before
                );
                assert_eq!(runtime.resource(deferred).unwrap(), resource_before);
                assert_eq!(
                    runtime.resource_allocation_mode(deferred).unwrap(),
                    mode_before
                );
                saw_stale = true;
            }
            suffix.push(dispatch);
            if saw_stale {
                break;
            }
        }
        assert!(saw_stale);
    }
    assert_eq!(source_suffix, restored_suffix);

    let mut saw_low_complete = false;
    for _ in 0..32 {
        let dispatch = source.step().unwrap().expect("low completion event");
        saw_low_complete |= dispatch.records.iter().any(|record| {
            record.request == low
                && record.at == t(14)
                && record.transition == kairo_ecs_des::LifecycleTransition::Completed
        });
        source_suffix.push(dispatch);
        if saw_low_complete {
            break;
        }
    }
    let mut saw_low_complete_restored = false;
    for _ in 0..32 {
        let dispatch = restored
            .step()
            .unwrap()
            .expect("restored low completion event");
        saw_low_complete_restored |= dispatch.records.iter().any(|record| {
            record.request == low
                && record.at == t(14)
                && record.transition == kairo_ecs_des::LifecycleTransition::Completed
        });
        restored_suffix.push(dispatch);
        if saw_low_complete_restored {
            break;
        }
    }
    assert!(saw_low_complete && saw_low_complete_restored);
    assert_eq!(source_suffix, restored_suffix);
    let full_lifecycle = prefix
        .iter()
        .chain(&source_suffix)
        .flat_map(|dispatch| &dispatch.records)
        .filter(|record| {
            matches!(record.snapshot.work, Some(work) if work == low_work || work == ready_work)
                && matches!(
                    record.transition,
                    kairo_ecs_des::LifecycleTransition::Granted
                        | kairo_ecs_des::LifecycleTransition::Preempted
                        | kairo_ecs_des::LifecycleTransition::Resumed
                        | kairo_ecs_des::LifecycleTransition::Completed
                )
        })
        .map(|record| {
            (
                record.snapshot.work.unwrap(),
                record.at.ticks(),
                record.transition,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        full_lifecycle,
        vec![
            (low_work, 1, kairo_ecs_des::LifecycleTransition::Granted),
            (low_work, 4, kairo_ecs_des::LifecycleTransition::Preempted),
            (ready_work, 6, kairo_ecs_des::LifecycleTransition::Granted),
            (ready_work, 7, kairo_ecs_des::LifecycleTransition::Completed),
            (low_work, 7, kairo_ecs_des::LifecycleTransition::Resumed),
            (low_work, 14, kairo_ecs_des::LifecycleTransition::Completed),
        ]
    );
    assert_eq!(source.request(low).unwrap(), restored.request(low).unwrap());
    assert_eq!(
        source.request(urgent).unwrap(),
        restored.request(urgent).unwrap()
    );
    assert_eq!(
        source.request(trigger).unwrap(),
        restored.request(trigger).unwrap()
    );
    assert_eq!(
        source.work_progress(low_work).unwrap(),
        restored.work_progress(low_work).unwrap()
    );
    assert_eq!(
        source.work_progress(urgent_work).unwrap(),
        restored.work_progress(urgent_work).unwrap()
    );
    assert_eq!(
        source.work_progress(trigger_work).unwrap(),
        restored.work_progress(trigger_work).unwrap()
    );
    assert_eq!(
        source.work_progress(ready_work).unwrap(),
        restored.work_progress(ready_work).unwrap()
    );
}

#[test]
fn wire_v3_mixed_interior_cut_both_completion_orders_restore_suffix() {
    run_checkpointed_readiness_order(true);
    run_checkpointed_readiness_order(false);
}

#[test]
fn wire_v3_preflush_budget_halt_wire_roundtrip_and_repeat_halt() {
    let mut source = FlowRuntime::with_config(kairo_ecs_des::FlowConfig {
        max_same_tick_flow_transitions: NonZeroU64::new(1).unwrap(),
    });
    let resource = source
        .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
        .unwrap();
    let owner = source.spawn_actor().unwrap();
    let work = source
        .create_work(owner, SimDuration::from_ticks(4), "ctx", 7_u64)
        .unwrap();
    let request = source
        .acquire(resource)
        .owner(owner)
        .timed_work(work)
        .submit()
        .unwrap();
    assert!(source.step().unwrap().is_some());
    let flush = source
        .schedule_allocation_flush(resource, source.now(), i32::MAX)
        .unwrap();
    assert!(matches!(
        source.step(),
        Err(kairo_ecs_des::FlowError::SameTickBudgetExceeded { .. })
    ));
    let halted = source.budget_snapshot();
    let image = source
        .capture_checkpoint_v3(&codecs(), FlowCheckpointLimits::default())
        .unwrap();
    assert!(image.budget_halt.is_some());
    assert_eq!(image.started_frontier, Some(source.now()));
    assert!(image.commands.iter().any(|(event, command)| *event == flush
        && *command == kairo_ecs_des::FlowCommandV3::AllocationFlush(resource)));
    let bytes = image
        .encode_wire_v3(kairo_ecs_des::FlowCheckpointWireLimits::default())
        .unwrap();
    let decoded = kairo_ecs_des::FlowCheckpointV3::decode_wire_v3(
        &bytes,
        kairo_ecs_des::FlowCheckpointWireLimits::default(),
    )
    .unwrap();
    assert_eq!(decoded, image);
    let mut restored =
        FlowRuntime::restore_checkpoint_v3(decoded, &codecs(), FlowCheckpointLimits::default())
            .unwrap();
    assert_eq!(
        restored
            .capture_checkpoint_v3(&codecs(), FlowCheckpointLimits::default())
            .unwrap(),
        image
    );
    assert_eq!(
        source.request(request).unwrap(),
        restored.request(request).unwrap()
    );
    for runtime in [&mut source, &mut restored] {
        assert!(matches!(
            runtime.step(),
            Err(kairo_ecs_des::FlowError::SameTickBudgetExceeded { .. })
        ));
        assert_eq!(runtime.budget_snapshot(), halted);
        let repeated = runtime
            .capture_checkpoint_v3(&codecs(), FlowCheckpointLimits::default())
            .unwrap();
        assert_eq!(repeated, image);
        assert_eq!(
            repeated
                .encode_wire_v3(kairo_ecs_des::FlowCheckpointWireLimits::default())
                .unwrap(),
            bytes
        );
    }
}

#[test]
fn wire_v3_started_now_flush_roundtrip_and_same_tick_readmission() {
    let mut source = FlowRuntime::new();
    let resource = source
        .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
        .unwrap();
    let owner = source.spawn_actor().unwrap();
    let work = source
        .create_work(owner, SimDuration::from_ticks(3), "ctx", 13_u64)
        .unwrap();
    let request = source
        .acquire(resource)
        .owner(owner)
        .timed_work(work)
        .submit()
        .unwrap();
    let queued_owner = source.spawn_actor().unwrap();
    let queued_work = source
        .create_work(queued_owner, SimDuration::from_ticks(2), "ctx", 17_u64)
        .unwrap();
    let queued_request = source
        .acquire(resource)
        .owner(queued_owner)
        .timed_work(queued_work)
        .submit()
        .unwrap();
    source.step().unwrap();
    let flush = source
        .schedule_allocation_flush(resource, source.now(), i32::MAX)
        .unwrap();
    let codecs = codecs();
    let image = source
        .capture_checkpoint_v3(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    assert_eq!(image.started_frontier, Some(source.now()));
    assert!(image.commands.iter().any(|(event, _)| *event == flush));
    let wire = image
        .encode_wire_v3(kairo_ecs_des::FlowCheckpointWireLimits::default())
        .unwrap();
    let decoded = kairo_ecs_des::FlowCheckpointV3::decode_wire_v3(
        &wire,
        kairo_ecs_des::FlowCheckpointWireLimits::default(),
    )
    .unwrap();
    assert_eq!(decoded, image);
    assert_eq!(
        decoded
            .encode_wire_v3(kairo_ecs_des::FlowCheckpointWireLimits::default())
            .unwrap(),
        wire
    );
    let mut restored =
        FlowRuntime::restore_checkpoint_v3(decoded, &codecs, FlowCheckpointLimits::default())
            .unwrap();
    assert_eq!(
        restored
            .capture_checkpoint_v3(&codecs, FlowCheckpointLimits::default())
            .unwrap(),
        image
    );
    let source_first_cut = source.now();
    let source_first_flush = drain_through_checkpoint_event(&mut source, flush, source_first_cut);
    let restored_first_cut = restored.now();
    let restored_first_flush =
        drain_through_checkpoint_event(&mut restored, flush, restored_first_cut);
    assert_eq!(source_first_flush, restored_first_flush);
    assert!(source_first_flush
        .iter()
        .any(|dispatch| dispatch.event == flush));
    assert_eq!(
        source.request(request).unwrap(),
        restored.request(request).unwrap()
    );
    assert_eq!(source.request(request).unwrap().state, RequestState::Active);
    assert_eq!(
        source.resource_allocation_mode(resource),
        Ok(FlowAllocationMode::Deferred)
    );
    assert_eq!(
        restored.resource_allocation_mode(resource),
        Ok(FlowAllocationMode::Deferred)
    );
    assert_eq!(
        source.request(queued_request).unwrap().state,
        RequestState::Queued
    );
    let lease = source.request(request).unwrap().lease.unwrap();
    let restored_lease = restored.request(request).unwrap().lease.unwrap();
    assert_eq!(lease, restored_lease);
    source.release(lease, source.now()).unwrap();
    restored.release(restored_lease, restored.now()).unwrap();
    let second_flush = source
        .schedule_allocation_flush(resource, source.now(), i32::MAX)
        .unwrap();
    let restored_second_flush = restored
        .schedule_allocation_flush(resource, restored.now(), i32::MAX)
        .unwrap();
    assert_eq!(second_flush, restored_second_flush);
    let second_image = source
        .capture_checkpoint_v3(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    assert_eq!(second_image.started_frontier, Some(source.now()));
    assert!(second_image
        .commands
        .iter()
        .any(|(event, command)| *event == second_flush
            && *command == kairo_ecs_des::FlowCommandV3::AllocationFlush(resource)));
    let second_wire = second_image
        .encode_wire_v3(kairo_ecs_des::FlowCheckpointWireLimits::default())
        .unwrap();
    let second_decoded = kairo_ecs_des::FlowCheckpointV3::decode_wire_v3(
        &second_wire,
        kairo_ecs_des::FlowCheckpointWireLimits::default(),
    )
    .unwrap();
    assert_eq!(second_decoded, second_image);
    assert_eq!(
        second_decoded
            .encode_wire_v3(kairo_ecs_des::FlowCheckpointWireLimits::default())
            .unwrap(),
        second_wire
    );
    let mut restored_second = FlowRuntime::restore_checkpoint_v3(
        second_decoded,
        &codecs,
        FlowCheckpointLimits::default(),
    )
    .unwrap();
    let source_second_cut = source.now();
    let source_second_flush =
        drain_through_checkpoint_event(&mut source, second_flush, source_second_cut);
    let restored_second_cut = restored_second.now();
    let restored_second_flush =
        drain_through_checkpoint_event(&mut restored_second, second_flush, restored_second_cut);
    assert_eq!(source_second_flush, restored_second_flush);
    assert!(source_second_flush.iter().any(|dispatch| {
        dispatch.event == second_flush
            && dispatch.records.iter().any(|record| {
                record.request == queued_request
                    && record.transition == kairo_ecs_des::LifecycleTransition::Granted
            })
    }));
    assert_eq!(
        source.request(request).unwrap().state,
        RequestState::Released
    );
    assert_eq!(
        source.request(queued_request).unwrap().state,
        RequestState::Active
    );
    assert_eq!(
        source.request(request).unwrap(),
        restored_second.request(request).unwrap()
    );
    assert_eq!(
        source.request(queued_request).unwrap(),
        restored_second.request(queued_request).unwrap()
    );
    let source_tail = source.run_for(8).unwrap();
    let restored_tail = restored_second.run_for(8).unwrap();
    assert_eq!(source_tail, restored_tail);
    assert!(!source_tail.budget_exhausted);
    assert_eq!(source.budget_snapshot().scheduler.pending_events, 0);
    assert_eq!(
        restored_second.budget_snapshot().scheduler.pending_events,
        0
    );
    assert_eq!(
        source.request(request).unwrap(),
        restored_second.request(request).unwrap()
    );
}
#[test]
fn wire_v3_structural_flush_bad_scheduler_joins_fail_native_precodec() {
    let (mut source, codecs, resource, _, flush, _) = rich_deferred_seed();
    let stale_resource = source.create_resource(1).unwrap();
    source.remove_resource(stale_resource).unwrap();
    let removal_image = source
        .capture_checkpoint_v3(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    let removal_event = removal_image
        .commands
        .iter()
        .find_map(|(event, command)| {
            (*command == kairo_ecs_des::FlowCommandV3::Remove(stale_resource)).then_some(*event)
        })
        .unwrap();
    let removal_tick = removal_image
        .scheduler
        .entries
        .iter()
        .find(|entry| entry.id == removal_event)
        .unwrap()
        .request
        .at;
    drain_through_checkpoint_event(&mut source, removal_event, removal_tick);
    let replacement = source.create_resource(1).unwrap();
    assert_eq!(
        stale_resource.entity_id().index,
        replacement.entity_id().index
    );
    assert_ne!(
        stale_resource.entity_id().generation,
        replacement.entity_id().generation
    );
    let advance = source
        .schedule_resource_grant_enabled(replacement, false, t(1), 0)
        .unwrap();
    drain_through_checkpoint_event(&mut source, advance, t(1));
    let image = source
        .capture_checkpoint_v3(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    let roundtrip = |candidate: &kairo_ecs_des::FlowCheckpointV3| {
        let bytes = candidate
            .encode_wire_v3(kairo_ecs_des::FlowCheckpointWireLimits::default())
            .expect("native-join corruption remains structurally encodable");
        let decoded = kairo_ecs_des::FlowCheckpointV3::decode_wire_v3(
            &bytes,
            kairo_ecs_des::FlowCheckpointWireLimits::default(),
        )
        .expect("wire decoder remains a detached structural decoder");
        assert_eq!(&decoded, candidate);
        assert_eq!(
            decoded
                .encode_wire_v3(kairo_ecs_des::FlowCheckpointWireLimits::default())
                .unwrap(),
            bytes
        );
        decoded
    };
    CODEC_CALLS.with(|count| count.set(0));
    let valid = roundtrip(&image);
    let _restored =
        FlowRuntime::restore_checkpoint_v3(valid, &codecs, FlowCheckpointLimits::default())
            .unwrap();
    CODEC_CALLS.with(|count| assert!(count.get() > 0));
    let gate = image
        .commands
        .iter()
        .find_map(|(event, command)| {
            matches!(command, kairo_ecs_des::FlowCommandV3::GrantEnabled(..)).then_some(*event)
        })
        .unwrap();
    let mut cases = Vec::new();
    let mut past_flush = image.clone();
    past_flush
        .scheduler
        .entries
        .iter_mut()
        .find(|entry| entry.id == flush)
        .unwrap()
        .request
        .at = t(0);
    cases.push((past_flush, "invalid or conflicting allocation flush join"));
    let mut wrong_kind = image.clone();
    wrong_kind
        .scheduler
        .entries
        .iter_mut()
        .find(|entry| entry.id == flush)
        .unwrap()
        .request
        .kind = kairo_ecs_types::EventKind::Custom(9999);
    cases.push((wrong_kind, "command scheduler kind mismatch"));
    let mut wrong_entity = image.clone();
    wrong_entity
        .scheduler
        .entries
        .iter_mut()
        .find(|entry| entry.id == flush)
        .unwrap()
        .request
        .entity = Some(EntityId::new(u64::MAX, u32::MAX));
    cases.push((wrong_entity, "invalid or conflicting allocation flush join"));
    let mut stale_flush = image.clone();
    stale_flush
        .commands
        .iter_mut()
        .find(|(event, _)| *event == flush)
        .unwrap()
        .1 = kairo_ecs_des::FlowCommandV3::AllocationFlush(stale_resource);
    cases.push((stale_flush, "invalid or conflicting allocation flush join"));
    let mut wrong_mode = image.clone();
    wrong_mode
        .allocation_modes
        .iter_mut()
        .find(|(id, _)| *id == resource)
        .unwrap()
        .1 = FlowAllocationMode::Eager;
    cases.push((wrong_mode, "invalid or conflicting allocation flush join"));
    let mut remove_conflict = image.clone();
    remove_conflict
        .commands
        .iter_mut()
        .find(|(event, _)| *event == gate)
        .unwrap()
        .1 = kairo_ecs_des::FlowCommandV3::Remove(resource);
    cases.push((
        remove_conflict,
        "invalid or conflicting allocation flush join",
    ));
    for (index, (candidate, expected)) in cases.into_iter().enumerate() {
        CODEC_CALLS.with(|count| count.set(0));
        let decoded = roundtrip(&candidate);
        assert_eq!(decoded.version, FLOW_CHECKPOINT_VERSION_V3);
        assert!(
            matches!(FlowRuntime::restore_checkpoint_v3(decoded, &codecs, FlowCheckpointLimits::default()), Err(FlowCheckpointError::InvalidState(actual)) if actual == expected),
            "native join case {index}"
        );
        CODEC_CALLS.with(|count| assert_eq!(count.get(), 0));
    }
}
