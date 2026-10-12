use kairo_ecs_des::*;
use kairo_ecs_types::{EntityId, EventId, SimDuration, SimTime};
use std::cell::Cell;
use std::num::NonZeroU64;
thread_local! {
    static CODEC_CALLS: Cell<usize> = const { Cell::new(0) };
    static ENCODE_CALLS: Cell<usize> = const { Cell::new(0) };
}
fn wire_limits(limits: FlowCheckpointLimits) -> FlowCheckpointWireLimits {
    FlowCheckpointWireLimits {
        flow: limits,
        ..FlowCheckpointWireLimits::default()
    }
}
fn capture_routed(
    runtime: &FlowRuntime,
    codecs: &FlowCheckpointCodecs,
    limits: FlowCheckpointLimits,
) -> Result<FlowCheckpointV3, FlowCheckpointError> {
    let image = runtime.capture_checkpoint_image(codecs, limits)?;
    let bytes = image.encode_wire(wire_limits(limits)).unwrap();
    let decoded = FlowCheckpointImage::decode_wire(&bytes, wire_limits(limits)).unwrap();
    assert_eq!(decoded, image);
    assert_eq!(decoded.encode_wire(wire_limits(limits)).unwrap(), bytes);
    let FlowCheckpointImage::V3(image) = decoded else {
        panic!("Deferred cut must remain complete V3");
    };
    Ok(image)
}
fn restore_routed(
    image: FlowCheckpointV3,
    codecs: &FlowCheckpointCodecs,
    limits: FlowCheckpointLimits,
) -> Result<FlowRuntime, FlowCheckpointError> {
    let bytes = FlowCheckpointImage::V3(image)
        .encode_wire(wire_limits(limits))
        .unwrap();
    let decoded = FlowCheckpointImage::decode_wire(&bytes, wire_limits(limits)).unwrap();
    assert_eq!(decoded.encode_wire(wire_limits(limits)).unwrap(), bytes);
    decoded
        .restore_with_rebind(codecs, limits)
        .map(|(runtime, _)| runtime)
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

fn template_context(template: &u64) -> u64 {
    *template
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
    let image = capture_routed(&source, &codecs, FlowCheckpointLimits::default()).unwrap();
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
    let complete = FlowCheckpointRef::V3(&image).commands().collect::<Vec<_>>();
    assert_eq!(complete, image.commands);
    assert_eq!(
        FlowCheckpointRef::V3(&image).commands().collect::<Vec<_>>(),
        complete
    );
    assert!(complete
        .iter()
        .any(|(event, command)| *event == stale_completion
            && matches!(command, FlowCommandV3::Completion(_, _, _, due) if *due == t(11))));
    let source_context = source
        .work_context::<ReadinessContext>(trigger_work)
        .unwrap()
        .clone();
    assert_eq!(source_context.calls, 0);
    assert_eq!(source_context.ready_at, None);
    assert!(!source_context.emitted);
    let mut restored =
        restore_routed(image.clone(), &codecs, FlowCheckpointLimits::default()).unwrap();
    assert_eq!(
        capture_routed(&restored, &codecs, FlowCheckpointLimits::default()).unwrap(),
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
            let stored_progress_before =
                capture_routed(runtime, &codecs, FlowCheckpointLimits::default())
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
                    capture_routed(runtime, &codecs, FlowCheckpointLimits::default())
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

fn prove_preflush_halt() {
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
    let before = source.budget_snapshot();
    let initial_error = source.step().unwrap_err();
    assert!(matches!(
        initial_error,
        FlowError::SameTickBudgetExceeded { .. }
    ));
    let halted = source.budget_snapshot();
    let image = capture_routed(&source, &codecs(), FlowCheckpointLimits::default()).unwrap();
    assert!(image.budget_halt.is_some());
    assert_eq!(image.started_frontier, Some(source.now()));
    assert!(image.commands.iter().any(|(id, _)| *id == flush));
    let mut restored =
        restore_routed(image.clone(), &codecs(), FlowCheckpointLimits::default()).unwrap();
    assert_eq!(restored.budget_snapshot(), source.budget_snapshot());
    assert_eq!(before.scheduler, source.budget_snapshot().scheduler);
    assert_eq!(
        capture_routed(&restored, &codecs(), FlowCheckpointLimits::default()).unwrap(),
        image
    );
    assert_eq!(
        source.request(request).unwrap(),
        restored.request(request).unwrap()
    );
    assert_eq!(source.step().unwrap_err(), initial_error);
    assert_eq!(restored.step().unwrap_err(), initial_error);
    assert_eq!(source.budget_snapshot(), halted);
    assert_eq!(restored.budget_snapshot(), halted);
    assert_eq!(
        capture_routed(&restored, &codecs(), FlowCheckpointLimits::default()).unwrap(),
        image
    );
}
fn prove_current_tick() {
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
    let image = capture_routed(&source, &codecs, FlowCheckpointLimits::default()).unwrap();
    assert_eq!(image.started_frontier, Some(source.now()));
    assert!(image.commands.iter().any(|(event, _)| *event == flush));
    let mut restored =
        restore_routed(image.clone(), &codecs, FlowCheckpointLimits::default()).unwrap();
    assert_eq!(
        capture_routed(&restored, &codecs, FlowCheckpointLimits::default()).unwrap(),
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
    let second_image = capture_routed(&source, &codecs, FlowCheckpointLimits::default()).unwrap();
    assert_eq!(second_image.started_frontier, Some(source.now()));
    assert!(second_image
        .commands
        .iter()
        .any(|(event, command)| *event == second_flush
            && *command == kairo_ecs_des::FlowCommandV3::AllocationFlush(resource)));
    let mut restored_second =
        restore_routed(second_image, &codecs, FlowCheckpointLimits::default()).unwrap();
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
fn routing_v3_capture_truth_table() {
    for case in 0..5 {
        let mut source = FlowRuntime::new();
        let deferred = case >= 3;
        let resource = source
            .create_resource_with_allocation_mode(
                1,
                if deferred {
                    FlowAllocationMode::Deferred
                } else {
                    FlowAllocationMode::Eager
                },
            )
            .unwrap();
        if case == 1 {
            source
                .schedule_resource_grant_enabled(resource, false, t(0), 0)
                .unwrap();
            source.step().unwrap().unwrap();
        }
        if case == 2 {
            source
                .schedule_resource_grant_enabled(resource, true, t(4), 0)
                .unwrap();
        }
        let flush = if case == 4 {
            Some(source.schedule_allocation_flush(resource, t(4), 0).unwrap())
        } else {
            None
        };
        let codecs = codecs();
        let limits = FlowCheckpointLimits::default();
        let before = source.capture_checkpoint_v3(&codecs, limits).unwrap();
        let budget = source.budget_snapshot();
        let routed = source.capture_checkpoint_image(&codecs, limits).unwrap();
        match (&routed, case) {
            (FlowCheckpointImage::V1(v1), 0) => {
                assert_eq!(*v1, source.capture_checkpoint(&codecs, limits).unwrap())
            }
            (FlowCheckpointImage::V2(v2), 1 | 2) => {
                assert_eq!(*v2, source.capture_checkpoint_v2(&codecs, limits).unwrap())
            }
            (FlowCheckpointImage::V3(v3), 3 | 4) => {
                assert_eq!(*v3, before);
                assert_eq!(
                    v3.allocation_modes,
                    vec![(resource, FlowAllocationMode::Deferred)]
                );
                if let Some(event) = flush {
                    assert_eq!(
                        v3.commands,
                        vec![(event, FlowCommandV3::AllocationFlush(resource))]
                    );
                }
            }
            _ => panic!("wrong narrowest version for case{case}"),
        }
        assert_eq!(
            source.capture_checkpoint_v3(&codecs, limits).unwrap(),
            before
        );
        assert_eq!(source.budget_snapshot(), budget);
    }
}
fn restart_source(mode: FlowAllocationMode) -> (FlowRuntime, FlowCheckpointCodecs) {
    let mut source = FlowRuntime::new();
    source
        .create_resource_with_allocation_mode(1, mode)
        .unwrap();
    let owner = source.spawn_actor().unwrap();
    source
        .create_restartable_work(
            owner,
            SimDuration::from_ticks(3),
            "ctx",
            0x1234_u64,
            template_context,
        )
        .unwrap();
    (source, codecs())
}
#[test]
fn routing_v3_legacy_limits_errors_restart_and_wire_parity() {
    let (source, codecs) = restart_source(FlowAllocationMode::Eager);
    let mut limits = FlowCheckpointLimits {
        max_registrations: 4,
        ..FlowCheckpointLimits::default()
    };
    ENCODE_CALLS.with(|n| n.set(0));
    let direct = source.capture_checkpoint(&codecs, limits).unwrap();
    assert_eq!(ENCODE_CALLS.with(|n| n.get()), 2);
    assert!(!direct.restart_stores[0].rows[0].2.is_empty());
    assert!(!direct.context_stores[0].rows[0].1.is_empty());
    ENCODE_CALLS.with(|n| n.set(0));
    let routed = source.capture_checkpoint_image(&codecs, limits).unwrap();
    assert_eq!(ENCODE_CALLS.with(|n| n.get()), 2);
    assert_eq!(routed, FlowCheckpointImage::V1(direct.clone()));
    assert_eq!(
        routed.encode_wire(wire_limits(limits)).unwrap(),
        direct.encode_wire_v1(wire_limits(limits)).unwrap()
    );
    CODEC_CALLS.with(|n| n.set(0));
    let bytes = routed.encode_wire(wire_limits(limits)).unwrap();
    let decoded = FlowCheckpointImage::decode_wire(&bytes, wire_limits(limits)).unwrap();
    assert_eq!(decoded, routed);
    let (restored, _) = decoded.restore_with_rebind(&codecs, limits).unwrap();
    assert_eq!(CODEC_CALLS.with(|n| n.get()), 2);
    assert_eq!(
        restored.capture_checkpoint(&codecs, limits).unwrap(),
        direct
    );
    let mut gated = source;
    let resource = gated
        .capture_checkpoint_v3(&codecs, FlowCheckpointLimits::default())
        .unwrap()
        .resources[0];
    gated
        .schedule_resource_grant_enabled(resource, true, t(4), 0)
        .unwrap();
    ENCODE_CALLS.with(|n| n.set(0));
    let direct = gated.capture_checkpoint_v2(&codecs, limits).unwrap();
    assert_eq!(ENCODE_CALLS.with(|n| n.get()), 2);
    ENCODE_CALLS.with(|n| n.set(0));
    let routed = gated.capture_checkpoint_image(&codecs, limits).unwrap();
    assert_eq!(ENCODE_CALLS.with(|n| n.get()), 2);
    assert_eq!(routed, FlowCheckpointImage::V2(direct.clone()));
    assert_eq!(
        routed.encode_wire(wire_limits(limits)).unwrap(),
        direct.encode_wire_v2(wire_limits(limits)).unwrap()
    );
    CODEC_CALLS.with(|n| n.set(0));
    let bytes = routed.encode_wire(wire_limits(limits)).unwrap();
    let decoded = FlowCheckpointImage::decode_wire(&bytes, wire_limits(limits)).unwrap();
    assert_eq!(decoded, routed);
    let (restored, _) = decoded.restore_with_rebind(&codecs, limits).unwrap();
    assert_eq!(CODEC_CALLS.with(|n| n.get()), 2);
    assert_eq!(
        restored.capture_checkpoint_v2(&codecs, limits).unwrap(),
        direct
    );
    limits.max_registrations = 3;
    ENCODE_CALLS.with(|n| n.set(0));
    let expected = gated.capture_checkpoint_v2(&codecs, limits).unwrap_err();
    assert_eq!(
        expected,
        FlowCheckpointError::LimitExceeded("registrations")
    );
    assert_eq!(
        gated.capture_checkpoint_image(&codecs, limits).unwrap_err(),
        expected
    );
    assert_eq!(ENCODE_CALLS.with(|n| n.get()), 0);
    let (deferred, codecs) = restart_source(FlowAllocationMode::Deferred);
    limits.max_registrations = 4;
    ENCODE_CALLS.with(|n| n.set(0));
    assert_eq!(
        deferred
            .capture_checkpoint_image(&codecs, limits)
            .unwrap_err(),
        FlowCheckpointError::LimitExceeded("registrations")
    );
    assert_eq!(ENCODE_CALLS.with(|n| n.get()), 0);
    limits.max_registrations = 5;
    ENCODE_CALLS.with(|n| n.set(0));
    let routed = deferred.capture_checkpoint_image(&codecs, limits).unwrap();
    assert!(matches!(routed, FlowCheckpointImage::V3(_)));
    assert_eq!(ENCODE_CALLS.with(|n| n.get()), 2);
    limits.max_registrations = 3;
    limits.max_payload_bytes = 0;
    ENCODE_CALLS.with(|n| n.set(0));
    assert_eq!(
        gated.capture_checkpoint_image(&codecs, limits).unwrap_err(),
        gated.capture_checkpoint_v2(&codecs, limits).unwrap_err()
    );
    assert_eq!(ENCODE_CALLS.with(|n| n.get()), 0);
}
#[test]
fn routing_v3_deferred_current_tick_restore_complete_suffix() {
    prove_current_tick();
    prove_preflush_halt();
}
#[test]
fn routing_v3_complete_command_iterator_all_versions() {
    let (mut source, codecs) = restart_source(FlowAllocationMode::Eager);
    let limits = FlowCheckpointLimits::default();
    let resource = source
        .capture_checkpoint(&codecs, limits)
        .unwrap()
        .resources[0];
    let entity = source.spawn_actor().unwrap();
    let work = source
        .create_work(entity, SimDuration::from_ticks(3), "ctx", 19_u64)
        .unwrap();
    let request = source
        .acquire(resource)
        .owner(entity)
        .timed_work(work)
        .submit()
        .unwrap();
    source.step().unwrap().unwrap();
    let lease = source
        .request(request)
        .unwrap()
        .lease
        .expect("real active lease");
    let v1 = source.capture_checkpoint(&codecs, limits).unwrap();
    let v2 = source.capture_checkpoint_v2(&codecs, limits).unwrap();
    let common = [
        FlowCommandV1::Submit(request),
        FlowCommandV1::Release(lease),
        FlowCommandV1::Capacity(resource, 2),
        FlowCommandV1::Remove(resource),
        FlowCommandV1::Despawn(entity),
        FlowCommandV1::Deadline(request),
        FlowCommandV1::Cancel(request),
        FlowCommandV1::Reprioritize(request, -1),
        FlowCommandV1::Completion(request, lease, 9, t(11)),
        FlowCommandV1::Notify,
        FlowCommandV1::Domain(work, kairo_ecs_types::EventKind::custom(700)),
        FlowCommandV1::DomainControl(
            work,
            kairo_ecs_types::EventKind::custom(700),
            FlowDomainControl::Pause,
        ),
    ];
    let mut one = v1;
    one.commands = common
        .iter()
        .copied()
        .enumerate()
        .map(|(i, c)| (EventId::new(i as u64, 0), c))
        .collect();
    let mut two = v2;
    two.commands = one
        .commands
        .iter()
        .map(|(e, c)| (*e, (*c).into()))
        .collect();
    two.commands.push((
        EventId::new(12, 0),
        FlowCommandV2::GrantEnabled(resource, true),
    ));
    let (deferred, codecs) = restart_source(FlowAllocationMode::Deferred);
    let mut three = deferred.capture_checkpoint_v3(&codecs, limits).unwrap();
    three.commands = two
        .commands
        .iter()
        .map(|(e, c)| (*e, (*c).into()))
        .collect();
    three.commands.push((
        EventId::new(13, 0),
        FlowCommandV3::AllocationFlush(three.resources[0]),
    ));
    // Detached structural ITERATOR ONLY controls; no native validity claim.
    // The real mixed runtime cuts below separately prove live stale continuation.
    for image in [
        FlowCheckpointImage::V1(one),
        FlowCheckpointImage::V2(two),
        FlowCheckpointImage::V3(three),
    ] {
        let original = image.clone();
        let expected: Vec<_> = match &image {
            FlowCheckpointImage::V1(i) => {
                i.commands.iter().map(|(e, c)| (*e, (*c).into())).collect()
            }
            FlowCheckpointImage::V2(i) => {
                i.commands.iter().map(|(e, c)| (*e, (*c).into())).collect()
            }
            FlowCheckpointImage::V3(i) => i.commands.clone(),
            FlowCheckpointImage::V4(_) => panic!("legacy iterator fixture must remain V1/V2/V3"),
        };
        assert_eq!(image.as_ref().commands().collect::<Vec<_>>(), expected);
        assert_eq!(image.as_ref().commands().collect::<Vec<_>>(), expected);
        assert_eq!(image, original);
    }
    run_checkpointed_readiness_order(false);
    run_checkpointed_readiness_order(true);
}
#[test]
fn routing_v3_invalid_modes_flush_joins_fail_precodec_without_mutation() {
    let (mut source, codecs) = restart_source(FlowAllocationMode::Deferred);
    let limits = FlowCheckpointLimits::default();
    let resource = source
        .capture_checkpoint_v3(&codecs, limits)
        .unwrap()
        .resources[0];
    let event = source.schedule_allocation_flush(resource, t(4), 0).unwrap();
    let remove_event = source
        .schedule_resource_grant_enabled(resource, true, t(5), 0)
        .unwrap();
    let before = source.capture_checkpoint_v3(&codecs, limits).unwrap();
    let budget = source.budget_snapshot();
    let mut foreign_runtime = FlowRuntime::new();
    for _ in 0..8 {
        foreign_runtime.spawn_actor().unwrap();
    }
    let foreign_resource = foreign_runtime.create_resource(1).unwrap();
    assert!(!before.resources.contains(&foreign_resource));
    for change in 0..8 {
        let mut bad = before.clone();
        match change {
            0 => bad.allocation_modes.clear(),
            1 => bad.allocation_modes[0].0 = foreign_resource,
            2 => {
                bad.scheduler
                    .entries
                    .iter_mut()
                    .find(|e| e.id == event)
                    .unwrap()
                    .request
                    .kind = kairo_ecs_types::EventKind::custom(u32::MAX)
            }
            3 => {
                bad.scheduler
                    .entries
                    .iter_mut()
                    .find(|e| e.id == event)
                    .unwrap()
                    .request
                    .entity = Some(EntityId::new(u64::MAX, 0))
            }
            4 => bad.allocation_modes[0].1 = FlowAllocationMode::Eager,
            5 => {
                bad.commands[0].1 = FlowCommandV3::AllocationFlush(foreign_resource);
            }
            6 => {
                bad.allocation_modes
                    .push((foreign_resource, FlowAllocationMode::Eager));
            }
            _ => {
                bad.commands
                    .iter_mut()
                    .find(|(event, _)| *event == remove_event)
                    .unwrap()
                    .1 = FlowCommandV3::Remove(resource);
            }
        }
        CODEC_CALLS.with(|n| n.set(0));
        let expected = FlowCheckpointError::InvalidState(match change {
            0 | 1 | 6 => "resource allocation modes are not canonical",
            2 => "command scheduler kind mismatch",
            _ => "invalid or conflicting allocation flush join",
        });
        let bad = FlowCheckpointImage::V3(bad);
        let staged = if matches!(change, 0 | 1 | 6) {
            bad
        } else {
            let bytes = bad.encode_wire(wire_limits(limits)).unwrap();
            let staged = FlowCheckpointImage::decode_wire(&bytes, wire_limits(limits)).unwrap();
            assert_eq!(staged, bad);
            staged
        };
        assert_eq!(
            staged.restore_with_rebind(&codecs, limits).err().unwrap(),
            expected
        );
        assert_eq!(CODEC_CALLS.with(|n| n.get()), 0);
        assert_eq!(source.budget_snapshot(), budget);
        assert_eq!(
            source.capture_checkpoint_v3(&codecs, limits).unwrap(),
            before
        );
    }
    CODEC_CALLS.with(|n| n.set(0));
    let (restored, _) = FlowCheckpointImage::V3(before.clone())
        .restore_with_rebind(&codecs, limits)
        .unwrap();
    assert_eq!(CODEC_CALLS.with(|n| n.get()), 2);
    assert_eq!(
        restored.capture_checkpoint_v3(&codecs, limits).unwrap(),
        before
    );
}
#[test]
fn routing_v3_known_magic_dispatch_no_fallback() {
    let limits = FlowCheckpointLimits::default();
    let wire = wire_limits(limits);
    let (source, codecs) = restart_source(FlowAllocationMode::Eager);
    let v1 = FlowCheckpointImage::V1(source.capture_checkpoint(&codecs, limits).unwrap());
    let v2 = FlowCheckpointImage::V2(source.capture_checkpoint_v2(&codecs, limits).unwrap());
    let (source, codecs) = restart_source(FlowAllocationMode::Deferred);
    let v3 = FlowCheckpointImage::V3(source.capture_checkpoint_v3(&codecs, limits).unwrap());
    for image in [v1, v2, v3] {
        let bytes = image.encode_wire(wire).unwrap();
        assert_eq!(
            FlowCheckpointImage::decode_wire(&bytes, wire).unwrap(),
            image
        );
        let mut schema = bytes.clone();
        schema[8..10].copy_from_slice(&77u16.to_le_bytes());
        assert_eq!(
            FlowCheckpointImage::decode_wire(&schema, wire).unwrap_err(),
            FlowCheckpointWireError::UnsupportedSchema(77)
        );
        let mut version = bytes.clone();
        version[10..14].copy_from_slice(&99u32.to_le_bytes());
        assert_eq!(
            FlowCheckpointImage::decode_wire(&version, wire).unwrap_err(),
            FlowCheckpointWireError::InvalidValue("Flow checkpoint version")
        );
        let mut trailing = bytes;
        trailing.push(0);
        assert_eq!(
            FlowCheckpointImage::decode_wire(&trailing, wire).unwrap_err(),
            FlowCheckpointWireError::TrailingBytes
        );
    }
    assert_eq!(
        FlowCheckpointImage::decode_wire(b"UNKNOWN!", wire).unwrap_err(),
        FlowCheckpointWireError::InvalidTag
    );
    assert_eq!(
        FlowCheckpointImage::decode_wire(b"KFLOWV", wire).unwrap_err(),
        FlowCheckpointWireError::Truncated
    );
}
