use kairo_ecs_core::ScheduledEventPreview;
use kairo_ecs_des::{
    FlowAllocationMode, FlowCallbackCause, FlowCallbackSnapshot, FlowCheckpointCodecError,
    FlowCheckpointCodecs, FlowCheckpointImage, FlowCheckpointLimits, FlowCommandSink,
    FlowContinuations, FlowRuntime, LifecycleTransition, PreemptionStrategy, RequestState,
    WorkState,
};
use kairo_ecs_types::{EventId, EventKind, SimDuration, SimTime};

fn t(value: u128) -> SimTime {
    SimTime::from_ticks(value)
}

fn step(flow: &mut FlowRuntime) -> kairo_ecs_des::FlowDispatch {
    let dispatch = flow.step().unwrap().expect("a scheduled Flow event");
    assert!(
        dispatch.error.is_none(),
        "unexpected semantic error: {dispatch:?}"
    );
    dispatch
}

fn codecs() -> FlowCheckpointCodecs {
    let mut codecs = FlowCheckpointCodecs::new();
    codecs
        .register_context("preview.u64", 1, encode_u64, decode_u64)
        .unwrap();
    codecs
}

fn encode_u64(value: &u64, remaining: usize) -> Result<Vec<u8>, FlowCheckpointCodecError> {
    if remaining < 8 {
        return Err(FlowCheckpointCodecError("payload limit".into()));
    }
    Ok(value.to_le_bytes().to_vec())
}

fn decode_u64(
    bytes: &[u8],
    _: &kairo_ecs_des::FlowCheckpointRebindV1,
) -> Result<u64, FlowCheckpointCodecError> {
    let bytes: [u8; 8] = bytes
        .try_into()
        .map_err(|_| FlowCheckpointCodecError("bad u64 payload".into()))?;
    Ok(u64::from_le_bytes(bytes))
}

fn image(flow: &FlowRuntime, codecs: &FlowCheckpointCodecs) -> FlowCheckpointImage {
    flow.capture_checkpoint_image(codecs, FlowCheckpointLimits::default())
        .unwrap()
}

fn expected_head(image: &FlowCheckpointImage) -> Option<ScheduledEventPreview> {
    let entries = match image {
        FlowCheckpointImage::V1(image) => &image.scheduler.entries,
        FlowCheckpointImage::V2(image) => &image.scheduler.entries,
        FlowCheckpointImage::V3(image) => &image.scheduler.entries,
        FlowCheckpointImage::V4(image) => &image.scheduler.entries,
    };
    entries
        .iter()
        .filter(|entry| entry.live)
        .min_by_key(|entry| (entry.request.at, entry.request.priority, entry.sequence))
        .map(|entry| ScheduledEventPreview {
            id: entry.id,
            at: entry.request.at,
            priority: entry.request.priority,
            sequence: entry.sequence,
            entity: entry.request.entity,
            kind: entry.request.kind,
        })
}

fn preview_without_change(
    flow: &mut FlowRuntime,
    codecs: &FlowCheckpointCodecs,
    expected: Option<ScheduledEventPreview>,
) -> Option<ScheduledEventPreview> {
    let before_image = image(flow, codecs);
    let before_budget = flow.budget_snapshot();
    let before_now = flow.now();
    assert_eq!(flow.peek_next(), expected);
    assert_eq!(
        flow.peek_next(),
        expected,
        "repeated preview changed the head"
    );
    assert_eq!(flow.now(), before_now);
    assert_eq!(flow.budget_snapshot(), before_budget);
    assert_eq!(image(flow, codecs), before_image);
    expected
}

fn preview_next_without_change(
    flow: &mut FlowRuntime,
    codecs: &FlowCheckpointCodecs,
) -> Option<ScheduledEventPreview> {
    let expected = expected_head(&image(flow, codecs));
    preview_without_change(flow, codecs, expected)
}

fn assert_event(
    event: Option<ScheduledEventPreview>,
    id: EventId,
    at: u128,
    priority: i32,
    sequence: u64,
    kind: EventKind,
) {
    assert_eq!(
        event,
        Some(ScheduledEventPreview {
            id,
            at: t(at),
            priority,
            sequence,
            entity: None,
            kind,
        })
    );
}

#[test]
fn flow_preview_empty() {
    let mut flow = FlowRuntime::new();
    let codecs = codecs();
    assert_eq!(flow.now(), SimTime::ZERO);
    assert_eq!(preview_without_change(&mut flow, &codecs, None), None);
    assert_eq!(preview_without_change(&mut flow, &codecs, None), None);
}

#[test]
fn flow_preview_ordered_live_head() {
    let mut flow = FlowRuntime::new();
    let resources = [
        flow.create_resource(1).unwrap(),
        flow.create_resource(1).unwrap(),
        flow.create_resource(1).unwrap(),
    ];
    let first = flow
        .schedule_resource_grant_enabled(resources[0], false, t(4), 7)
        .unwrap();
    let tied_first = flow
        .schedule_resource_grant_enabled(resources[1], false, t(4), -2)
        .unwrap();
    let tied_second = flow
        .schedule_resource_grant_enabled(resources[2], false, t(4), -2)
        .unwrap();
    let later = flow
        .schedule_resource_grant_enabled(resources[0], true, t(5), -10)
        .unwrap();
    let codecs = codecs();
    let dispatch_kind = EventKind::custom(4000);

    for (id, priority, sequence) in [(tied_first, -2, 1), (tied_second, -2, 2), (first, 7, 0)] {
        let preview = preview_without_change(
            &mut flow,
            &codecs,
            Some(ScheduledEventPreview {
                id,
                at: t(4),
                priority,
                sequence,
                entity: None,
                kind: dispatch_kind,
            }),
        )
        .unwrap();
        assert_event(Some(preview), id, 4, priority, sequence, dispatch_kind);
        let dispatch = step(&mut flow);
        assert_eq!(dispatch.event, id);
        assert_eq!(dispatch.at, t(4));
    }
    assert_event(
        preview_without_change(
            &mut flow,
            &codecs,
            Some(ScheduledEventPreview {
                id: later,
                at: t(5),
                priority: -10,
                sequence: 3,
                entity: None,
                kind: dispatch_kind,
            }),
        ),
        later,
        5,
        -10,
        3,
        dispatch_kind,
    );
    let dispatch = step(&mut flow);
    assert_eq!(dispatch.event, later);
    assert_eq!(dispatch.at, t(5));
    assert_eq!(flow.resource_grant_enabled(resources[0]), Ok(true));
}

#[test]
fn flow_preview_deferred_disabled_flush() {
    let codecs = codecs();
    let mut flow = FlowRuntime::new();
    let deferred = flow
        .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
        .unwrap();
    let owner = flow.spawn_actor().unwrap();
    let work = flow
        .create_work(owner, SimDuration::from_ticks(5), "preview.u64", 11_u64)
        .unwrap();
    let request = flow
        .acquire(deferred)
        .owner(owner)
        .timed_work(work)
        .at(t(0))
        .scheduler_priority(0)
        .submit()
        .unwrap();
    let submit = step(&mut flow);
    assert_eq!(submit.at, t(0));
    assert_eq!(flow.request(request).unwrap().state, RequestState::Queued);
    assert_eq!(flow.work_progress(work).unwrap().state, WorkState::Pending);

    let flush = flow
        .schedule_allocation_flush(deferred, t(4), i32::MAX)
        .unwrap();
    assert_event(
        preview_without_change(
            &mut flow,
            &codecs,
            Some(ScheduledEventPreview {
                id: flush,
                at: t(4),
                priority: i32::MAX,
                sequence: 1,
                entity: None,
                kind: EventKind::custom(4000),
            }),
        ),
        flush,
        4,
        i32::MAX,
        1,
        EventKind::custom(4000),
    );
    let grant = step(&mut flow);
    assert_eq!(grant.event, flush);
    assert_eq!(grant.at, t(4));
    assert!(grant.records.iter().any(|record| {
        record.request == request && record.transition == LifecycleTransition::Granted
    }));
    assert_eq!(flow.request(request).unwrap().state, RequestState::Active);
    let completion = preview_next_without_change(&mut flow, &codecs);
    assert_eq!(completion.unwrap().at, t(9));
    assert_eq!(completion.unwrap().kind, EventKind::custom(4001));
    let completed = step(&mut flow);
    assert_eq!(completed.event, completion.unwrap().id);
    assert_eq!(completed.at, t(9));
    assert!(completed.records.iter().any(|record| {
        record.request == request && record.transition == LifecycleTransition::Completed
    }));

    let mut disabled = FlowRuntime::new();
    let deferred = disabled
        .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
        .unwrap();
    let owner = disabled.spawn_actor().unwrap();
    let work = disabled
        .create_work(owner, SimDuration::from_ticks(5), "preview.u64", 17_u64)
        .unwrap();
    let disabled_event = disabled
        .schedule_resource_grant_enabled(deferred, false, t(0), -10)
        .unwrap();
    let request = disabled
        .acquire(deferred)
        .owner(owner)
        .timed_work(work)
        .at(t(0))
        .scheduler_priority(0)
        .submit()
        .unwrap();
    let flush = disabled
        .schedule_allocation_flush(deferred, t(0), 10)
        .unwrap();
    assert_eq!(
        preview_without_change(
            &mut disabled,
            &codecs,
            Some(ScheduledEventPreview {
                id: disabled_event,
                at: t(0),
                priority: -10,
                sequence: 0,
                entity: None,
                kind: EventKind::custom(4000),
            }),
        )
        .unwrap()
        .id,
        disabled_event
    );
    assert_eq!(step(&mut disabled).event, disabled_event);
    let submit_head = preview_next_without_change(&mut disabled, &codecs).unwrap();
    assert_eq!(submit_head.at, t(0));
    assert_eq!(submit_head.kind, EventKind::custom(4000));
    assert_eq!(step(&mut disabled).event, submit_head.id);
    assert_eq!(
        disabled.request(request).unwrap().state,
        RequestState::Queued
    );
    let flush_head = preview_without_change(
        &mut disabled,
        &codecs,
        Some(ScheduledEventPreview {
            id: flush,
            at: t(0),
            priority: 10,
            sequence: 2,
            entity: None,
            kind: EventKind::custom(4000),
        }),
    )
    .unwrap();
    assert_eq!(step(&mut disabled).event, flush_head.id);
    assert_eq!(
        disabled.request(request).unwrap().state,
        RequestState::Queued
    );
    assert_eq!(disabled.request(request).unwrap().lease, None);
    assert!(disabled.resource(deferred).unwrap().active.is_empty());
    assert_eq!(
        disabled.work_progress(work).unwrap().state,
        WorkState::Pending
    );
    assert_eq!(preview_without_change(&mut disabled, &codecs, None), None);
}

fn count_completion(context: &mut u64, snapshot: &FlowCallbackSnapshot, _: &mut FlowCommandSink) {
    if matches!(
        &snapshot.cause,
        FlowCallbackCause::Work {
            transition: LifecycleTransition::Completed,
            ..
        }
    ) {
        *context += 1;
    }
}

fn callback_codecs() -> FlowCheckpointCodecs {
    let mut codecs = codecs();
    codecs
        .register_work_continuations(
            "preview.u64",
            kairo_ecs_des::FlowContinuationCodeIds {
                on_complete: Some(kairo_ecs_des::FlowCallbackCodeV1 {
                    stable_id: "flow-preview.count-completion".into(),
                    version: 1,
                }),
                ..Default::default()
            },
            FlowContinuations {
                on_complete: Some(count_completion),
                ..Default::default()
            },
        )
        .unwrap();
    codecs
}

fn drain_through(
    flow: &mut FlowRuntime,
    event: EventId,
    at: SimTime,
) -> Vec<kairo_ecs_des::FlowDispatch> {
    let mut dispatches = Vec::new();
    for _ in 0..16 {
        let dispatch = step(flow);
        assert_eq!(dispatch.at, at, "event closure crossed its expected tick");
        let reached = dispatch.event == event;
        dispatches.push(dispatch);
        if reached {
            return dispatches;
        }
    }
    panic!("event closure exceeded its bounded dispatch count");
}

fn v3(image: FlowCheckpointImage) -> kairo_ecs_des::FlowCheckpointV3 {
    match image {
        FlowCheckpointImage::V3(image) => image,
        other => panic!("Deferred preview fixture expected V3 image, got {other:?}"),
    }
}

fn raw_progress(
    image: &kairo_ecs_des::FlowCheckpointV3,
    work: kairo_ecs_des::WorkId,
) -> kairo_ecs_des::WorkProgress {
    image
        .builtins
        .work_progress
        .as_ref()
        .unwrap()
        .rows
        .iter()
        .find(|(entity, _)| *entity == work.entity_id())
        .unwrap()
        .1
        .clone()
}

#[test]
fn flow_preview_live_stale_completion() {
    let mut flow = FlowRuntime::new();
    flow.register_work_continuations(
        "preview.u64",
        FlowContinuations {
            on_complete: Some(count_completion),
            ..Default::default()
        },
    )
    .unwrap();
    let resource = flow
        .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
        .unwrap();
    let low_owner = flow.spawn_actor().unwrap();
    let low_work = flow
        .create_work(low_owner, SimDuration::from_ticks(10), "preview.u64", 0_u64)
        .unwrap();
    let low = flow
        .acquire(resource)
        .owner(low_owner)
        .timed_work(low_work)
        .at(t(1))
        .priority(10)
        .preemptible(PreemptionStrategy::Suspend)
        .submit()
        .unwrap();
    let first_flush = flow
        .schedule_allocation_flush(resource, t(1), i32::MAX)
        .unwrap();
    drain_through(&mut flow, first_flush, t(1));
    assert_eq!(flow.request(low).unwrap().state, RequestState::Active);

    let urgent_owner = flow.spawn_actor().unwrap();
    let urgent_work = flow
        .create_work(
            urgent_owner,
            SimDuration::from_ticks(2),
            "preview.u64",
            0_u64,
        )
        .unwrap();
    let urgent = flow
        .acquire(resource)
        .owner(urgent_owner)
        .timed_work(urgent_work)
        .at(t(4))
        .priority(1)
        .can_preempt(true)
        .submit()
        .unwrap();
    let preempt_flush = flow
        .schedule_allocation_flush(resource, t(4), i32::MAX)
        .unwrap();
    drain_through(&mut flow, preempt_flush, t(4));
    assert_eq!(flow.request(low).unwrap().state, RequestState::Suspended);
    assert_eq!(flow.request(urgent).unwrap().state, RequestState::Active);

    let resume_flush = flow
        .schedule_allocation_flush(resource, t(7), i32::MAX)
        .unwrap();
    let urgent_completion = step(&mut flow);
    assert_eq!(urgent_completion.at, t(6));
    assert!(urgent_completion.records.iter().any(|record| {
        record.request == urgent && record.transition == LifecycleTransition::Completed
    }));
    let urgent_notification = step(&mut flow);
    assert_eq!(urgent_notification.at, t(6));
    assert_eq!(flow.work_context::<u64>(urgent_work), Ok(&1));
    drain_through(&mut flow, resume_flush, t(7));
    assert_eq!(flow.now(), t(7));
    assert_eq!(flow.request(low).unwrap().state, RequestState::Active);

    let codecs = callback_codecs();
    let before_preview = v3(image(&flow, &codecs));
    let stale_event = before_preview
        .commands
        .iter()
        .find_map(|(event, command)| match command {
            kairo_ecs_des::FlowCommandV3::Completion(request, _, _, due)
                if *request == low && *due == t(11) =>
            {
                Some(*event)
            }
            _ => None,
        })
        .expect("superseded completion remains in the native command map");
    assert!(before_preview
        .scheduler
        .entries
        .iter()
        .any(|entry| { entry.id == stale_event && entry.live && entry.request.at == t(11) }));
    let stale_head = ScheduledEventPreview {
        id: stale_event,
        at: t(11),
        priority: 0,
        sequence: before_preview
            .scheduler
            .entries
            .iter()
            .find(|entry| entry.id == stale_event)
            .unwrap()
            .sequence,
        entity: None,
        kind: EventKind::custom(4001),
    };
    assert_eq!(
        preview_without_change(&mut flow, &codecs, Some(stale_head)),
        Some(stale_head)
    );
    assert_eq!(flow.work_context::<u64>(low_work), Ok(&0));
    assert_eq!(flow.work_context::<u64>(urgent_work), Ok(&1));

    let raw_before_stale = raw_progress(&before_preview, low_work);
    let stale = step(&mut flow);
    assert_eq!(stale.event, stale_event);
    assert_eq!(stale.at, t(11));
    assert!(stale.records.is_empty());
    assert!(stale.callback_batches.is_empty());
    let after_stale = v3(image(&flow, &codecs));
    assert_eq!(raw_progress(&after_stale, low_work), raw_before_stale);
    assert_eq!(flow.work_context::<u64>(low_work), Ok(&0));
    assert_eq!(flow.work_context::<u64>(urgent_work), Ok(&1));
    assert_eq!(
        flow.work_progress(low_work).unwrap().useful_elapsed,
        SimDuration::from_ticks(7)
    );
    assert_eq!(
        flow.work_progress(low_work).unwrap().remaining,
        SimDuration::from_ticks(3)
    );

    let useful_head = preview_next_without_change(&mut flow, &codecs).unwrap();
    assert_eq!(useful_head.at, t(14));
    assert_eq!(useful_head.kind, EventKind::custom(4001));
    let useful = step(&mut flow);
    assert_eq!(useful.event, useful_head.id);
    assert_eq!(useful.at, t(14));
    assert!(useful.records.iter().any(|record| {
        record.request == low && record.transition == LifecycleTransition::Completed
    }));
    let notification = step(&mut flow);
    assert_eq!(notification.at, t(14));
    assert_eq!(flow.work_context::<u64>(low_work), Ok(&1));
}

#[test]
fn flow_preview_budget_halted() {
    let codecs = codecs();
    let mut flow = FlowRuntime::with_config(kairo_ecs_des::FlowConfig {
        max_same_tick_flow_transitions: std::num::NonZeroU64::new(1).unwrap(),
    });
    let resource = flow
        .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
        .unwrap();
    let owner = flow.spawn_actor().unwrap();
    let work = flow
        .create_work(owner, SimDuration::from_ticks(10), "preview.u64", 3_u64)
        .unwrap();
    let request = flow
        .acquire(resource)
        .owner(owner)
        .timed_work(work)
        .at(t(0))
        .submit()
        .unwrap();
    assert_eq!(step(&mut flow).at, t(0));
    assert_eq!(flow.request(request).unwrap().state, RequestState::Queued);
    let flush = flow
        .schedule_allocation_flush(resource, flow.now(), i32::MAX)
        .unwrap();
    let pending = preview_next_without_change(&mut flow, &codecs).unwrap();
    assert_eq!(pending.id, flush);
    assert_eq!(pending.at, t(0));

    let halt = kairo_ecs_des::FlowError::SameTickBudgetExceeded {
        at_ticks: 0,
        limit: 1,
    };
    assert_eq!(flow.step(), Err(halt));
    assert!(flow.budget_snapshot().halted.is_some());
    assert_eq!(flow.request(request).unwrap().state, RequestState::Queued);
    assert_eq!(flow.request(request).unwrap().lease, None);
    let halted_image = image(&flow, &codecs);
    let halted_budget = flow.budget_snapshot();
    assert_eq!(
        preview_without_change(&mut flow, &codecs, Some(pending)),
        Some(pending)
    );
    assert_eq!(flow.step(), Err(halt));
    assert_eq!(flow.budget_snapshot(), halted_budget);
    assert_eq!(image(&flow, &codecs), halted_image);
    assert_eq!(flow.request(request).unwrap().state, RequestState::Queued);
}

#[test]
fn flow_preview_horizon() {
    let mut flow = FlowRuntime::new();
    let at_horizon = flow.create_resource(1).unwrap();
    let after_horizon = flow.create_resource(1).unwrap();
    let at_h = flow
        .schedule_resource_grant_enabled(at_horizon, false, t(5), 0)
        .unwrap();
    let after_h = flow
        .schedule_resource_grant_enabled(after_horizon, false, t(6), 0)
        .unwrap();
    let codecs = codecs();
    let horizon = t(5);
    let mut admitted = Vec::new();
    while let Some(next) = preview_next_without_change(&mut flow, &codecs) {
        if next.at > horizon {
            break;
        }
        let dispatch = step(&mut flow);
        assert_eq!(dispatch.event, next.id);
        assert_eq!(dispatch.at, next.at);
        admitted.push(dispatch.event);
    }
    assert_eq!(admitted, vec![at_h]);
    assert_eq!(flow.now(), horizon);
    assert_eq!(flow.resource_grant_enabled(at_horizon), Ok(false));
    assert_eq!(flow.resource_grant_enabled(after_horizon), Ok(true));
    assert_event(
        preview_without_change(
            &mut flow,
            &codecs,
            Some(ScheduledEventPreview {
                id: after_h,
                at: t(6),
                priority: 0,
                sequence: 1,
                entity: None,
                kind: EventKind::custom(4000),
            }),
        ),
        after_h,
        6,
        0,
        1,
        EventKind::custom(4000),
    );
    assert_eq!(flow.now(), horizon);
    assert_eq!(flow.resource_grant_enabled(after_horizon), Ok(true));
}
