use kairo_ecs_des::{
    FlowAcquireCommand, FlowAllocationMode, FlowCallbackCause, FlowCallbackSnapshot,
    FlowCheckpointCodecError, FlowCheckpointCodecs, FlowCheckpointError, FlowCheckpointLimits,
    FlowCommandSink, FlowConfig, FlowContinuations, FlowOwnedCommand, FlowRuntime,
    LifecycleTransition as L, PreemptionStrategy, RequestState, ResourceAllocationAdmissionError,
    WorkState,
};
use kairo_ecs_types::{EntityId, EventId, SimDuration, SimTime};
use std::num::NonZeroU64;

fn t(value: u128) -> SimTime {
    SimTime::from_ticks(value)
}

fn step(flow: &mut FlowRuntime) -> kairo_ecs_des::FlowDispatch {
    let dispatch = flow.step().unwrap().expect("a scheduled event");
    assert!(dispatch.error.is_none(), "unexpected semantic error");
    dispatch
}

fn has_transition(
    d: &kairo_ecs_des::FlowDispatch,
    request: kairo_ecs_des::RequestId,
    at: u128,
    transition: L,
) -> bool {
    d.records
        .iter()
        .any(|row| row.request == request && row.at == t(at) && row.transition == transition)
}

fn has_any_transition(
    dispatch: &kairo_ecs_des::FlowDispatch,
    request: kairo_ecs_des::RequestId,
    transition: L,
) -> bool {
    dispatch
        .records
        .iter()
        .any(|row| row.request == request && row.transition == transition)
}

fn acquire_work(
    flow: &mut FlowRuntime,
    resource: kairo_ecs_des::ResourceId,
    owner: EntityId,
    work: kairo_ecs_des::WorkId,
    at: u128,
    priority: i32,
) -> kairo_ecs_des::RequestId {
    flow.acquire(resource)
        .owner(owner)
        .timed_work(work)
        .at(t(at))
        .priority(priority)
        .submit()
        .unwrap()
}

#[test]
fn deferred_submit_and_release_wait_for_explicit_flush() {
    let mut flow = FlowRuntime::new();
    let deferred = flow
        .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
        .unwrap();
    assert_eq!(
        flow.resource_allocation_mode(deferred),
        Ok(FlowAllocationMode::Deferred)
    );
    let owner_a = flow.spawn_actor().unwrap();
    let owner_b = flow.spawn_actor().unwrap();
    let owner_c = flow.spawn_actor().unwrap();
    let first_work = flow
        .create_work(owner_a, SimDuration::from_ticks(4), "first", ())
        .unwrap();
    let second_work = flow
        .create_work(owner_b, SimDuration::from_ticks(2), "second", ())
        .unwrap();
    let third_work = flow
        .create_work(owner_c, SimDuration::from_ticks(1), "third", ())
        .unwrap();

    let first = acquire_work(&mut flow, deferred, owner_a, first_work, 0, 0);
    let submit = step(&mut flow);
    assert!(has_transition(&submit, first, 0, L::Queued));
    assert!(!has_transition(&submit, first, 0, L::Granted));
    assert_eq!(flow.request(first).unwrap().state, RequestState::Queued);
    flow.schedule_allocation_flush(deferred, t(0), i32::MAX)
        .unwrap();
    assert!(has_transition(&step(&mut flow), first, 0, L::Granted));

    let second = acquire_work(&mut flow, deferred, owner_b, second_work, 1, 0);
    assert!(has_transition(&step(&mut flow), second, 1, L::Queued));
    let first_lease = flow.request(first).unwrap().lease.unwrap();
    flow.release(first_lease, t(2)).unwrap();
    let release = step(&mut flow);
    assert!(has_transition(&release, first, 2, L::Released));
    assert!(!has_transition(&release, second, 2, L::Granted));
    assert_eq!(flow.request(second).unwrap().state, RequestState::Queued);

    flow.schedule_allocation_flush(deferred, t(2), i32::MAX)
        .unwrap();
    assert!(has_transition(&step(&mut flow), second, 2, L::Granted));
    let third = acquire_work(&mut flow, deferred, owner_c, third_work, 3, 0);
    assert!(has_transition(&step(&mut flow), third, 3, L::Queued));
    let due = flow.run_for(8).unwrap();
    assert_eq!(flow.request(second).unwrap().state, RequestState::Completed);
    assert_eq!(flow.request(third).unwrap().state, RequestState::Queued);
    assert!(!due
        .dispatches
        .iter()
        .any(|d| has_any_transition(d, third, L::Granted)));

    flow.set_capacity(deferred, 2).unwrap();
    let growth = step(&mut flow);
    assert!(!has_any_transition(&growth, third, L::Granted));
    assert_eq!(flow.request(third).unwrap().state, RequestState::Queued);
    flow.schedule_resource_grant_enabled(deferred, false, t(6), 0)
        .unwrap();
    flow.schedule_resource_grant_enabled(deferred, true, t(7), 0)
        .unwrap();
    let gate_changes = flow.run_for(2).unwrap();
    assert_eq!(flow.resource_grant_enabled(deferred), Ok(true));
    assert!(!gate_changes
        .dispatches
        .iter()
        .any(|d| has_any_transition(d, third, L::Granted)));
    assert_eq!(flow.request(third).unwrap().state, RequestState::Queued);
    let before_flush = flow.work_progress(third_work).unwrap();
    assert_eq!(before_flush.state, WorkState::Pending);
    assert_eq!(before_flush.useful_elapsed, SimDuration::ZERO);
    flow.schedule_allocation_flush(deferred, flow.now(), i32::MAX)
        .unwrap();
    let final_flush = step(&mut flow);
    assert!(has_transition(&final_flush, third, 7, L::Granted));
    assert!(!has_transition(&final_flush, third, 7, L::Completed));
    let completion = step(&mut flow);
    assert!(has_transition(&completion, third, 8, L::Completed));

    // A pending request receives no service before the named horizon H; the
    // explicit flush grants it at H and starts positive service there.
    let mut horizon = FlowRuntime::new();
    let horizon_resource = horizon
        .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
        .unwrap();
    let horizon_owner = horizon.spawn_actor().unwrap();
    let horizon_work = horizon
        .create_work(horizon_owner, SimDuration::from_ticks(3), "horizon", ())
        .unwrap();
    let horizon_request = acquire_work(
        &mut horizon,
        horizon_resource,
        horizon_owner,
        horizon_work,
        0,
        0,
    );
    assert!(has_transition(
        &step(&mut horizon),
        horizon_request,
        0,
        L::Queued
    ));
    let h = 50;
    horizon
        .schedule_resource_grant_enabled(horizon_resource, false, t(h - 1), 0)
        .unwrap();
    assert_eq!(horizon.run_for(1).unwrap().dispatches.len(), 1);
    assert_eq!(horizon.now(), t(h - 1));
    assert_eq!(
        horizon.request(horizon_request).unwrap().state,
        RequestState::Queued
    );
    assert_eq!(
        horizon.work_progress(horizon_work).unwrap().useful_elapsed,
        SimDuration::ZERO
    );
    horizon
        .schedule_resource_grant_enabled(horizon_resource, true, t(h), 0)
        .unwrap();
    horizon
        .schedule_allocation_flush(horizon_resource, t(h), i32::MAX)
        .unwrap();
    let enable = step(&mut horizon);
    assert_eq!(enable.at, t(h));
    assert!(!has_transition(&enable, horizon_request, h, L::Granted));
    let at_h = step(&mut horizon);
    assert_eq!(at_h.at, t(h));
    assert!(has_transition(&at_h, horizon_request, h, L::Granted));
    assert!(!has_transition(&at_h, horizon_request, h, L::Completed));
    assert_eq!(
        horizon.work_progress(horizon_work).unwrap().useful_elapsed,
        SimDuration::ZERO
    );
    assert!(has_transition(
        &step(&mut horizon),
        horizon_request,
        h + 3,
        L::Completed
    ));

    let mut eager = FlowRuntime::new();
    let eager_resource = eager.create_resource(1).unwrap();
    let eager_owner = eager.spawn_actor().unwrap();
    let eager_work = eager
        .create_work(eager_owner, SimDuration::from_ticks(1), "eager", ())
        .unwrap();
    let eager_request = acquire_work(&mut eager, eager_resource, eager_owner, eager_work, 0, 0);
    assert!(has_transition(
        &step(&mut eager),
        eager_request,
        0,
        L::Granted
    ));
}

#[test]
fn deferred_calendar_gate_blocks_flush() {
    let mut flow = FlowRuntime::new();
    let resource = flow
        .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
        .unwrap();
    let owner = flow.spawn_actor().unwrap();
    let work = flow
        .create_work(owner, SimDuration::from_ticks(3), "gated", ())
        .unwrap();
    let request = flow
        .acquire(resource)
        .owner(owner)
        .timed_work(work)
        .at(t(0))
        .scheduler_priority(-10)
        .submit()
        .unwrap();
    flow.schedule_resource_grant_enabled(resource, false, t(0), i32::MAX)
        .unwrap();
    let initial = flow.run_for(2).unwrap();
    assert_eq!(flow.request(request).unwrap().state, RequestState::Queued);
    assert!(!initial
        .dispatches
        .iter()
        .any(|d| has_any_transition(d, request, L::Granted)));

    flow.schedule_allocation_flush(resource, t(0), i32::MAX)
        .unwrap();
    let closed_now = step(&mut flow);
    assert!(!has_any_transition(&closed_now, request, L::Granted));
    assert_eq!(flow.request(request).unwrap().state, RequestState::Queued);

    flow.schedule_resource_grant_enabled(resource, true, t(1), 0)
        .unwrap();
    let enable_open = step(&mut flow);
    assert_eq!(enable_open.at, t(1));
    assert!(!has_any_transition(&enable_open, request, L::Granted));
    flow.schedule_resource_grant_enabled(resource, false, t(2), 20)
        .unwrap();
    flow.schedule_allocation_flush(resource, t(2), -10).unwrap();
    let pending_disable_flush = step(&mut flow);
    assert_eq!(pending_disable_flush.at, t(2));
    assert!(!has_any_transition(
        &pending_disable_flush,
        request,
        L::Granted
    ));
    let disable = step(&mut flow);
    assert_eq!(disable.at, t(2));
    assert!(!has_any_transition(&disable, request, L::Granted));

    flow.schedule_resource_grant_enabled(resource, true, t(4), 10)
        .unwrap();
    flow.schedule_allocation_flush(resource, t(4), -10).unwrap();
    let closed_before_enable = step(&mut flow);
    assert_eq!(closed_before_enable.at, t(4));
    assert!(!has_any_transition(
        &closed_before_enable,
        request,
        L::Granted
    ));
    let enable = step(&mut flow);
    assert_eq!(enable.at, t(4));
    assert!(!has_any_transition(&enable, request, L::Granted));
    assert_eq!(flow.resource_grant_enabled(resource), Ok(true));
    flow.schedule_allocation_flush(resource, t(4), i32::MAX)
        .unwrap();
    assert!(has_transition(&step(&mut flow), request, 4, L::Granted));

    let other = flow
        .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
        .unwrap();
    let other_owner = flow.spawn_actor().unwrap();
    let other_work = flow
        .create_work(other_owner, SimDuration::from_ticks(1), "other", ())
        .unwrap();
    let other_ready_at = flow.now();
    let other_request = flow
        .acquire(other)
        .owner(other_owner)
        .timed_work(other_work)
        .at(other_ready_at)
        .submit()
        .unwrap();
    assert!(has_transition(
        &step(&mut flow),
        other_request,
        other_ready_at.ticks(),
        L::Queued
    ));
    let own_flush = flow
        .schedule_allocation_flush(resource, t(9), i32::MAX)
        .unwrap();
    let mut other_resource_granted = false;
    loop {
        let dispatch = step(&mut flow);
        other_resource_granted |= has_any_transition(&dispatch, other_request, L::Granted);
        if dispatch.event == own_flush {
            assert_eq!(dispatch.at, t(9));
            break;
        }
    }
    assert_eq!(
        flow.request(other_request).unwrap().state,
        RequestState::Queued
    );
    assert!(!other_resource_granted);
}

#[derive(Clone, Copy)]
struct ReadinessContext {
    resource: kairo_ecs_des::ResourceId,
    owner: EntityId,
    work: kairo_ecs_des::WorkId,
    calls: u32,
    ready_at: Option<SimTime>,
    emit_native: bool,
    emitted: bool,
}

fn create_ready_work(
    context: &mut ReadinessContext,
    snapshot: &FlowCallbackSnapshot,
    sink: &mut FlowCommandSink,
) {
    if matches!(
        &snapshot.cause,
        FlowCallbackCause::Work {
            transition: L::Completed,
            ..
        }
    ) {
        context.calls += 1;
        context.ready_at = Some(snapshot.delivery.at);
        if context.emit_native {
            context.emitted = sink
                .emit(FlowOwnedCommand::Acquire(FlowAcquireCommand {
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
                }))
                .is_ok();
        }
    }
}

fn drain_through_event(flow: &mut FlowRuntime, event: EventId) -> Vec<kairo_ecs_des::FlowDispatch> {
    let mut dispatches = Vec::new();
    for _ in 0..32 {
        let dispatch = step(flow);
        let reached = dispatch.event == event;
        dispatches.push(dispatch);
        if reached {
            return dispatches;
        }
    }
    panic!("event closure exceeded 32 dispatches before event {event:?}");
}

#[derive(Clone, Copy)]
enum ReadyMode {
    DirectCallback,
    LogicalCallback,
    Fixed,
}

#[test]
fn deferred_completion_created_readiness_precedes_resume() {
    for trigger_completion_first in [true, false] {
        for mode in [
            ReadyMode::DirectCallback,
            ReadyMode::LogicalCallback,
            ReadyMode::Fixed,
        ] {
            let fixed_ready_at = t(6);
            let mut flow = FlowRuntime::new();
            let deferred = flow
                .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
                .unwrap();
            let trigger_resource = flow.create_resource(1).unwrap();
            let owner_low = flow.spawn_actor().unwrap();
            let owner_urgent = flow.spawn_actor().unwrap();
            let owner_trigger = flow.spawn_actor().unwrap();
            let owner_ready = flow.spawn_actor().unwrap();
            let low_work = flow
                .create_work(owner_low, SimDuration::from_ticks(10), "low", ())
                .unwrap();
            let urgent_work = flow
                .create_work(owner_urgent, SimDuration::from_ticks(2), "urgent", ())
                .unwrap();
            flow.register_work_continuations(
                "trigger",
                FlowContinuations {
                    on_complete: Some(create_ready_work),
                    ..FlowContinuations::default()
                },
            )
            .unwrap();
            let ready_work = flow
                .create_work(owner_ready, SimDuration::from_ticks(1), "ready", ())
                .unwrap();
            let trigger_work = flow
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
                        emit_native: matches!(mode, ReadyMode::DirectCallback),
                        emitted: false,
                    },
                )
                .unwrap();

            let low = flow
                .acquire(deferred)
                .owner(owner_low)
                .timed_work(low_work)
                .at(t(1))
                .priority(10)
                .preemptible(PreemptionStrategy::Suspend)
                .submit()
                .unwrap();
            let initial_flush = flow
                .schedule_allocation_flush(deferred, t(1), i32::MAX)
                .unwrap();
            let mut observed = drain_through_event(&mut flow, initial_flush);
            assert!(observed
                .iter()
                .any(|d| has_transition(d, low, 1, L::Granted)));

            let (trigger_priority, urgent_priority, flush_priority) = if trigger_completion_first {
                (-10, -5, 0)
            } else {
                (0, -10, -5)
            };
            let urgent = flow
                .acquire(deferred)
                .owner(owner_urgent)
                .timed_work(urgent_work)
                .at(t(4))
                .priority(1)
                .can_preempt(true)
                .scheduler_priority(urgent_priority)
                .submit()
                .unwrap();
            let trigger = flow
                .acquire(trigger_resource)
                .owner(owner_trigger)
                .timed_work(trigger_work)
                .at(t(4))
                .scheduler_priority(trigger_priority)
                .submit()
                .unwrap();
            let preemption_flush = flow
                .schedule_allocation_flush(deferred, t(4), flush_priority)
                .unwrap();
            let mut at_four = Vec::new();
            let mut saw_trigger_grant = false;
            let mut saw_preemption_flush = false;
            for _ in 0..32 {
                let dispatch = step(&mut flow);
                assert_eq!(dispatch.at, t(4));
                saw_trigger_grant |= has_transition(&dispatch, trigger, 4, L::Granted);
                saw_preemption_flush |= dispatch.event == preemption_flush;
                at_four.push(dispatch);
                if saw_trigger_grant && saw_preemption_flush {
                    break;
                }
            }
            assert!(
                saw_trigger_grant,
                "tick-four trigger grant was not observed"
            );
            assert!(
                saw_preemption_flush,
                "tick-four allocation flush was not drained"
            );
            assert_eq!(at_four.len(), 3);
            assert!(at_four.iter().all(|d| d.at == t(4)));
            assert!(at_four
                .iter()
                .any(|d| has_transition(d, urgent, 4, L::Granted)));
            assert!(at_four
                .iter()
                .any(|d| has_transition(d, trigger, 4, L::Granted)));
            observed.extend(at_four);
            assert_eq!(flow.request(low).unwrap().state, RequestState::Suspended);
            let low_progress = flow.work_progress(low_work).unwrap();
            assert_eq!(low_progress.useful_elapsed, SimDuration::from_ticks(3));
            assert_eq!(low_progress.remaining, SimDuration::from_ticks(7));

            let flush_at_six = flow
                .schedule_allocation_flush(deferred, t(6), i32::MAX)
                .unwrap();
            let ready = loop {
                let trigger_done = observed
                    .iter()
                    .any(|d| has_transition(d, trigger, 6, L::Completed));
                let urgent_done = observed
                    .iter()
                    .any(|d| has_transition(d, urgent, 6, L::Completed));
                let ready_at = flow
                    .work_context::<ReadinessContext>(trigger_work)
                    .unwrap()
                    .ready_at;
                let direct_queued = flow
                    .work(ready_work)
                    .unwrap()
                    .request
                    .is_some_and(|request| {
                        flow.request(request)
                            .is_ok_and(|request| request.state == RequestState::Queued)
                    });
                if trigger_done
                    && urgent_done
                    && ready_at == Some(t(6))
                    && (!matches!(mode, ReadyMode::DirectCallback) || direct_queued)
                {
                    break flow.work(ready_work).unwrap().request;
                }
                let dispatch = step(&mut flow);
                assert_eq!(dispatch.at, t(6), "readiness closure crossed tick six");
                observed.push(dispatch);
                assert!(
                    observed.len() <= 32,
                    "tick-six event closure exceeded bound"
                );
            };

            assert!(observed
                .iter()
                .any(|d| has_transition(d, trigger, 6, L::Completed)));
            assert!(observed
                .iter()
                .any(|d| has_transition(d, urgent, 6, L::Completed)));
            assert_eq!(flow.request(urgent).unwrap().state, RequestState::Completed);
            assert_eq!(
                flow.request(trigger).unwrap().state,
                RequestState::Completed
            );
            let context = flow.work_context::<ReadinessContext>(trigger_work).unwrap();
            assert_eq!(context.calls, 1);
            assert_eq!(context.ready_at, Some(t(6)));
            assert_eq!(
                observed
                    .iter()
                    .position(|d| has_transition(d, trigger, 6, L::Completed))
                    < observed
                        .iter()
                        .position(|d| has_transition(d, urgent, 6, L::Completed)),
                trigger_completion_first
            );

            let ready = match mode {
                ReadyMode::DirectCallback => {
                    assert!(context.emitted);
                    ready.expect("direct callback admitted native request")
                }
                ReadyMode::LogicalCallback => {
                    assert!(!context.emitted);
                    assert!(ready.is_none());
                    flow.acquire(deferred)
                        .owner(owner_ready)
                        .timed_work(ready_work)
                        .at(t(6))
                        .priority(0)
                        .scheduler_priority(2)
                        .submit()
                        .unwrap()
                }
                ReadyMode::Fixed => {
                    let disclosed_ready_at = fixed_ready_at;
                    assert_eq!(disclosed_ready_at, t(6));
                    assert!(ready.is_none());
                    flow.acquire(deferred)
                        .owner(owner_ready)
                        .timed_work(ready_work)
                        .at(disclosed_ready_at)
                        .priority(0)
                        .scheduler_priority(2)
                        .submit()
                        .unwrap()
                }
            };
            let six_dispatches = drain_through_event(&mut flow, flush_at_six);
            assert!(six_dispatches.iter().all(|d| d.at == t(6)));
            assert!(observed
                .iter()
                .chain(six_dispatches.iter())
                .any(|d| has_transition(d, ready, 6, L::Queued)));
            let flush = six_dispatches.last().unwrap();
            assert_eq!(flush.event, flush_at_six);
            assert!(has_transition(flush, ready, 6, L::Granted));
            assert!(!has_any_transition(flush, low, L::Resumed));
            assert_eq!(flow.request(low).unwrap().state, RequestState::Suspended);
            let low_at_six = flow.work_progress(low_work).unwrap();
            assert_eq!(low_at_six.useful_elapsed, SimDuration::from_ticks(3));
            assert_eq!(low_at_six.remaining, SimDuration::from_ticks(7));
            observed.extend(six_dispatches);

            let mut saw_ready_complete = false;
            for _ in 0..32 {
                let dispatch = step(&mut flow);
                assert!(
                    dispatch.at <= t(7),
                    "positive work crossed expected tick seven"
                );
                if has_transition(&dispatch, ready, 7, L::Completed) {
                    saw_ready_complete = true;
                    assert!(!has_any_transition(&dispatch, low, L::Resumed));
                }
                observed.push(dispatch);
                if saw_ready_complete {
                    break;
                }
            }
            assert!(saw_ready_complete, "B did not complete at tick seven");
            assert_eq!(flow.now(), t(7));
            assert_eq!(flow.request(low).unwrap().state, RequestState::Suspended);
            let resume_flush = flow
                .schedule_allocation_flush(deferred, t(7), i32::MAX)
                .unwrap();
            let resume_dispatches = drain_through_event(&mut flow, resume_flush);
            assert!(resume_dispatches.iter().all(|d| d.at == t(7)));
            let resume = resume_dispatches.last().unwrap();
            assert!(has_transition(resume, low, 7, L::Resumed));
            let low_at_seven = flow.work_progress(low_work).unwrap();
            assert_eq!(low_at_seven.useful_elapsed, SimDuration::from_ticks(3));
            assert_eq!(low_at_seven.remaining, SimDuration::from_ticks(7));
            observed.extend(resume_dispatches);

            let mut saw_low_complete = false;
            for _ in 0..32 {
                let dispatch = step(&mut flow);
                if has_transition(&dispatch, low, 14, L::Completed) {
                    saw_low_complete = true;
                }
                observed.push(dispatch);
                if saw_low_complete {
                    break;
                }
            }
            assert!(saw_low_complete, "low did not complete at tick fourteen");
            let final_progress = flow.work_progress(low_work).unwrap();
            assert_eq!(final_progress.useful_elapsed, SimDuration::from_ticks(10));
            assert_eq!(final_progress.remaining, SimDuration::ZERO);

            let lifecycle: Vec<_> = observed
                .iter()
                .flat_map(|dispatch| dispatch.records.iter())
                .filter(|record| {
                    [low, ready].contains(&record.request)
                        && matches!(
                            record.transition,
                            L::Granted | L::Preempted | L::Resumed | L::Completed
                        )
                })
                .map(|record| (record.request, record.at.ticks(), record.transition))
                .collect();
            assert_eq!(
                lifecycle,
                vec![
                    (low, 1, L::Granted),
                    (low, 4, L::Preempted),
                    (ready, 6, L::Granted),
                    (ready, 7, L::Completed),
                    (low, 7, L::Resumed),
                    (low, 14, L::Completed),
                ]
            );
        }
    }

    // A callback-created logical A and fixed logical Z are merged by urgency,
    // readiness, and ASCII identity before native submission. Reverse both
    // completion insertion order and initial cohort presentation.
    for callback_completion_first in [true, false] {
        for reverse_initial_cohort in [true, false] {
            let mut flow = FlowRuntime::new();
            let deferred = flow
                .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
                .unwrap();
            let resource_a = flow.create_resource(1).unwrap();
            let resource_z = flow.create_resource(1).unwrap();
            let owner_a_trigger = flow.spawn_actor().unwrap();
            let owner_z_trigger = flow.spawn_actor().unwrap();
            let owner_a = flow.spawn_actor().unwrap();
            let owner_z = flow.spawn_actor().unwrap();
            let a_work = flow
                .create_work(owner_a, SimDuration::from_ticks(1), "A", ())
                .unwrap();
            let z_work = flow
                .create_work(owner_z, SimDuration::from_ticks(1), "Z", ())
                .unwrap();
            flow.register_work_continuations(
                "trigger",
                FlowContinuations {
                    on_complete: Some(create_ready_work),
                    ..FlowContinuations::default()
                },
            )
            .unwrap();
            let a_trigger = flow
                .create_work(
                    owner_a_trigger,
                    SimDuration::from_ticks(6),
                    "trigger",
                    ReadinessContext {
                        resource: deferred,
                        owner: owner_a,
                        work: a_work,
                        calls: 0,
                        ready_at: None,
                        emit_native: false,
                        emitted: false,
                    },
                )
                .unwrap();
            let z_trigger = flow
                .create_work(owner_z_trigger, SimDuration::from_ticks(6), "z-control", ())
                .unwrap();
            let fixed_z_ready_at = t(6);
            let (a_trigger_priority, z_trigger_priority) = if callback_completion_first {
                (-10, -5)
            } else {
                (-5, -10)
            };
            let a_trigger_request = flow
                .acquire(resource_a)
                .owner(owner_a_trigger)
                .timed_work(a_trigger)
                .at(t(0))
                .scheduler_priority(a_trigger_priority)
                .submit()
                .unwrap();
            let z_trigger_request = flow
                .acquire(resource_z)
                .owner(owner_z_trigger)
                .timed_work(z_trigger)
                .at(t(0))
                .scheduler_priority(z_trigger_priority)
                .submit()
                .unwrap();
            let first_start = step(&mut flow);
            let second_start = step(&mut flow);
            assert_eq!(first_start.at, t(0));
            assert_eq!(second_start.at, t(0));
            assert_eq!(
                flow.request(a_trigger_request).unwrap().state,
                RequestState::Active
            );
            assert_eq!(
                flow.request(z_trigger_request).unwrap().state,
                RequestState::Active
            );

            let flush = flow
                .schedule_allocation_flush(deferred, fixed_z_ready_at, i32::MAX)
                .unwrap();
            let mut completions = Vec::new();
            for _ in 0..32 {
                let dispatch = step(&mut flow);
                assert_eq!(
                    dispatch.at,
                    t(6),
                    "fixed readiness closure crossed tick six"
                );
                let a_done = has_transition(&dispatch, a_trigger_request, 6, L::Completed);
                let z_done = has_transition(&dispatch, z_trigger_request, 6, L::Completed);
                if a_done || z_done {
                    completions.push(if a_done { 'A' } else { 'Z' });
                }
                let a_context = flow.work_context::<ReadinessContext>(a_trigger).unwrap();
                if completions.len() == 2
                    && a_context.calls == 1
                    && a_context.ready_at == Some(t(6))
                {
                    break;
                }
            }
            assert_eq!(
                completions.len(),
                2,
                "both real completion events must drain"
            );
            assert_eq!(completions[0] == 'A', callback_completion_first);
            assert_eq!(
                flow.work_context::<ReadinessContext>(a_trigger)
                    .unwrap()
                    .ready_at,
                Some(t(6))
            );
            assert!(
                !flow
                    .work_context::<ReadinessContext>(a_trigger)
                    .unwrap()
                    .emitted
            );

            let initial_cohort = if reverse_initial_cohort {
                [('Z', owner_z, z_work), ('A', owner_a, a_work)]
            } else {
                [('A', owner_a, a_work), ('Z', owner_z, z_work)]
            };
            let mut merged = initial_cohort.to_vec();
            merged.sort_by_key(|(identity, _, _)| *identity);
            let mut requests = std::collections::BTreeMap::new();
            for (identity, owner, work) in merged {
                let at = if identity == 'A' {
                    flow.work_context::<ReadinessContext>(a_trigger)
                        .unwrap()
                        .ready_at
                        .expect("callback marks A readiness")
                } else {
                    fixed_z_ready_at
                };
                let request = flow
                    .acquire(deferred)
                    .owner(owner)
                    .timed_work(work)
                    .at(at)
                    .priority(0)
                    .scheduler_priority(2)
                    .submit()
                    .unwrap();
                requests.insert(identity, request);
            }
            let a = requests[&'A'];
            let z = requests[&'Z'];
            let six_dispatches = drain_through_event(&mut flow, flush);
            assert!(six_dispatches.iter().all(|d| d.at == t(6)));
            assert!(six_dispatches
                .iter()
                .any(|d| has_transition(d, a, 6, L::Granted)));
            assert!(!six_dispatches
                .iter()
                .any(|d| has_transition(d, z, 6, L::Granted)));
            assert_eq!(flow.request(a).unwrap().state, RequestState::Active);
            assert_eq!(flow.request(z).unwrap().state, RequestState::Queued);
            assert!(has_transition(&step(&mut flow), a, 7, L::Completed));
            assert_eq!(flow.request(z).unwrap().state, RequestState::Queued);
            let flush_z = flow
                .schedule_allocation_flush(deferred, t(7), i32::MAX)
                .unwrap();
            let seven_dispatches = drain_through_event(&mut flow, flush_z);
            assert!(seven_dispatches
                .iter()
                .any(|d| has_transition(d, z, 7, L::Granted)));
            assert!(has_transition(&step(&mut flow), z, 8, L::Completed));
        }
    }
}

#[test]
fn deferred_flush_admission_rejections_are_atomic() {
    let mut flow = FlowRuntime::new();
    let deferred = flow
        .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
        .unwrap();
    let eager = flow.create_resource(1).unwrap();
    assert_eq!(
        flow.schedule_allocation_flush(eager, t(0), 0),
        Err(ResourceAllocationAdmissionError::NotDeferred)
    );
    flow.set_capacity(eager, 1).unwrap();
    step(&mut flow);
    flow.set_capacity(eager, 1).unwrap();
    step(&mut flow);
    assert_eq!(flow.now(), t(0));
    assert!(flow.schedule_allocation_flush(deferred, t(0), 0).is_ok());
    assert_eq!(
        flow.schedule_allocation_flush(deferred, t(0), 1),
        Err(ResourceAllocationAdmissionError::DuplicateFlush)
    );

    let mut stale_flow = FlowRuntime::new();
    let stale = stale_flow
        .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
        .unwrap();
    stale_flow.remove_resource(stale).unwrap();
    step(&mut stale_flow);
    assert_eq!(
        stale_flow.schedule_allocation_flush(stale, t(0), 0),
        Err(ResourceAllocationAdmissionError::Flow(
            kairo_ecs_des::FlowError::InvalidResource
        ))
    );
    let replacement = stale_flow.create_resource(1).unwrap();
    assert_ne!(stale, replacement);
    assert_eq!(
        stale_flow.resource_allocation_mode(replacement),
        Ok(FlowAllocationMode::Eager)
    );

    let mut remove_first = FlowRuntime::new();
    let remove_resource = remove_first
        .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
        .unwrap();
    remove_first.remove_resource(remove_resource).unwrap();
    assert_eq!(
        remove_first.schedule_allocation_flush(remove_resource, t(1), 0),
        Err(ResourceAllocationAdmissionError::Flow(
            kairo_ecs_des::FlowError::ResourceInUse
        ))
    );
    let mut flush_first = FlowRuntime::new();
    let flush_resource = flush_first
        .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
        .unwrap();
    flush_first
        .schedule_allocation_flush(flush_resource, t(1), 0)
        .unwrap();
    assert_eq!(
        flush_first.remove_resource(flush_resource),
        Err(kairo_ecs_des::FlowError::InvalidState)
    );

    let mut past = FlowRuntime::new();
    let past_resource = past
        .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
        .unwrap();
    let past_eager = past.create_resource(1).unwrap();
    past.schedule_resource_grant_enabled(past_eager, false, t(5), 0)
        .unwrap();
    past.run_for(1).unwrap();
    assert_eq!(
        past.schedule_allocation_flush(past_resource, t(0), 0),
        Err(ResourceAllocationAdmissionError::Flow(
            kairo_ecs_des::FlowError::PastCommand
        ))
    );

    let mut retried = FlowRuntime::new();
    let retried_resource = retried
        .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
        .unwrap();
    let mut control = FlowRuntime::new();
    let control_resource = control
        .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
        .unwrap();
    assert_eq!(
        retried.schedule_allocation_flush(retried_resource, t(2), 0),
        control.schedule_allocation_flush(control_resource, t(2), 0)
    );
    assert_eq!(
        retried.schedule_allocation_flush(retried_resource, t(2), 1),
        Err(ResourceAllocationAdmissionError::DuplicateFlush)
    );
    let retry = retried
        .schedule_allocation_flush(retried_resource, t(3), 0)
        .unwrap();
    let expected = control
        .schedule_allocation_flush(control_resource, t(3), 0)
        .unwrap();
    assert_eq!(
        retry, expected,
        "a rejected admission did not consume event identity"
    );

    // A same-tick duplicate becomes admissible only after the prior flush
    // commits, and its identity still matches an untouched control.
    retried.run_for(1).unwrap();
    control.run_for(1).unwrap();
    assert_eq!(
        retried.schedule_allocation_flush(retried_resource, t(2), 0),
        control.schedule_allocation_flush(control_resource, t(2), 0)
    );
}

#[test]
fn deferred_precommit_halt_preserves_pending_flush() {
    let mut flow = FlowRuntime::with_config(FlowConfig {
        max_same_tick_flow_transitions: NonZeroU64::new(1).unwrap(),
    });
    let resource = flow
        .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
        .unwrap();
    let owner = flow.spawn_actor().unwrap();
    let work = flow
        .create_work(owner, SimDuration::from_ticks(10), "halt", ())
        .unwrap();
    let request = flow
        .acquire(resource)
        .owner(owner)
        .timed_work(work)
        .submit()
        .unwrap();
    assert!(has_transition(&step(&mut flow), request, 0, L::Queued));
    flow.schedule_allocation_flush(resource, t(0), i32::MAX)
        .unwrap();
    let before = flow.budget_snapshot();
    assert_eq!(
        flow.step(),
        Err(kairo_ecs_des::FlowError::SameTickBudgetExceeded {
            at_ticks: 0,
            limit: 1
        })
    );
    assert_eq!(flow.budget_snapshot().scheduler, before.scheduler);
    assert_eq!(flow.request(request).unwrap().state, RequestState::Queued);
    assert_eq!(flow.request(request).unwrap().lease, None);
    assert_eq!(flow.work_progress(work).unwrap().state, WorkState::Pending);
    assert_eq!(
        flow.step(),
        Err(kairo_ecs_des::FlowError::SameTickBudgetExceeded {
            at_ticks: 0,
            limit: 1
        })
    );
    assert_eq!(flow.request(request).unwrap().state, RequestState::Queued);
}

thread_local! {
    static ENCODER_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static DECODER_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn encode_u64(value: &u64, remaining: usize) -> Result<Vec<u8>, FlowCheckpointCodecError> {
    ENCODER_CALLS.set(ENCODER_CALLS.get() + 1);
    if remaining < 8 {
        return Err(FlowCheckpointCodecError("payload limit".into()));
    }
    Ok(value.to_le_bytes().to_vec())
}

fn decode_u64(
    bytes: &[u8],
    _: &kairo_ecs_des::FlowCheckpointRebindV1,
) -> Result<u64, FlowCheckpointCodecError> {
    DECODER_CALLS.set(DECODER_CALLS.get() + 1);
    let bytes: [u8; 8] = bytes
        .try_into()
        .map_err(|_| FlowCheckpointCodecError("bad u64".into()))?;
    Ok(u64::from_le_bytes(bytes))
}

#[test]
fn deferred_legacy_capture_rejects_before_codecs() {
    let limits = FlowCheckpointLimits::default();
    let mut codecs = FlowCheckpointCodecs::new();
    codecs
        .register_context("payload", 1, encode_u64, decode_u64)
        .unwrap();
    let mut empty_deferred = FlowRuntime::new();
    empty_deferred
        .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
        .unwrap();
    ENCODER_CALLS.set(0);
    DECODER_CALLS.set(0);
    assert!(matches!(
        empty_deferred.capture_checkpoint(&codecs, limits),
        Err(FlowCheckpointError::InvalidState(_))
    ));
    assert!(matches!(
        empty_deferred.capture_checkpoint_v2(&codecs, limits),
        Err(FlowCheckpointError::InvalidState(_))
    ));
    assert_eq!(ENCODER_CALLS.get(), 0);
    let routed = empty_deferred
        .capture_checkpoint_image(&codecs, limits)
        .unwrap();
    let kairo_ecs_des::FlowCheckpointImage::V3(routed) = routed else {
        panic!("Empty Deferred resource still requires V3");
    };
    assert_eq!(routed.allocation_modes.len(), 1);
    assert_eq!(routed.allocation_modes[0].1, FlowAllocationMode::Deferred);
    assert!(routed.commands.is_empty());
    assert_eq!(ENCODER_CALLS.get(), 0);
    assert_eq!(DECODER_CALLS.get(), 0);

    let mut deferred = FlowRuntime::new();
    let resource = deferred
        .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
        .unwrap();
    let owner = deferred.spawn_actor().unwrap();
    deferred
        .create_work(owner, SimDuration::from_ticks(3), "payload", 0x1234_u64)
        .unwrap();
    let flush = deferred
        .schedule_allocation_flush(resource, t(4), 0)
        .unwrap();

    ENCODER_CALLS.set(0);
    assert!(matches!(
        deferred.capture_checkpoint(&codecs, limits),
        Err(FlowCheckpointError::InvalidState(_))
    ));
    assert_eq!(ENCODER_CALLS.get(), 0);
    assert_eq!(DECODER_CALLS.get(), 0);
    assert!(matches!(
        deferred.capture_checkpoint_v2(&codecs, limits),
        Err(FlowCheckpointError::InvalidState(_))
    ));
    assert_eq!(ENCODER_CALLS.get(), 0);
    assert_eq!(DECODER_CALLS.get(), 0);
    assert_eq!(ENCODER_CALLS.get(), 0);
    assert_eq!(DECODER_CALLS.get(), 0);
    let routed = deferred.capture_checkpoint_image(&codecs, limits).unwrap();
    let kairo_ecs_des::FlowCheckpointImage::V3(routed) = routed else {
        panic!("Pending Deferred Flush requires V3");
    };
    assert_eq!(
        routed.allocation_modes,
        vec![(resource, FlowAllocationMode::Deferred)]
    );
    assert_eq!(
        routed.commands,
        vec![(
            flush,
            kairo_ecs_des::FlowCommandV3::AllocationFlush(resource)
        )]
    );
    assert_eq!(ENCODER_CALLS.get(), 1);
    assert_eq!(DECODER_CALLS.get(), 0);
    DECODER_CALLS.set(0);
    let (restored, _) = kairo_ecs_des::FlowCheckpointImage::V3(routed.clone())
        .restore_with_rebind(&codecs, limits)
        .unwrap();
    assert_eq!(DECODER_CALLS.get(), 1);
    assert_eq!(
        restored.resource_allocation_mode(resource),
        Ok(FlowAllocationMode::Deferred)
    );
    assert_eq!(
        restored
            .capture_checkpoint_v3(&codecs, limits)
            .unwrap()
            .commands,
        routed.commands
    );
    ENCODER_CALLS.set(0);
    DECODER_CALLS.set(0);

    let mut eager = FlowRuntime::new();
    let eager_owner = eager.spawn_actor().unwrap();
    let eager_resource = eager.create_resource(1).unwrap();
    eager
        .create_work(
            eager_owner,
            SimDuration::from_ticks(3),
            "payload",
            0x5678_u64,
        )
        .unwrap();
    ENCODER_CALLS.set(0);
    let legacy = eager.capture_checkpoint(&codecs, limits).unwrap();
    let direct_bytes = legacy.encode_wire_v1(Default::default()).unwrap();
    let routed = eager.capture_checkpoint_image(&codecs, limits).unwrap();
    let kairo_ecs_des::FlowCheckpointImage::V1(routed) = routed else {
        panic!("Eager state retains V1 routing");
    };
    assert_eq!(
        routed.encode_wire_v1(Default::default()).unwrap(),
        direct_bytes
    );
    assert!(ENCODER_CALLS.get() > 0);
    DECODER_CALLS.set(0);
    let restored = FlowRuntime::restore_checkpoint(legacy, &codecs, limits).unwrap();
    assert!(DECODER_CALLS.get() > 0);
    assert_eq!(
        restored.resource_allocation_mode(eager_resource),
        Ok(FlowAllocationMode::Eager)
    );

    let v2 = eager.capture_checkpoint_v2(&codecs, limits).unwrap();
    let restored_v2 = FlowRuntime::restore_checkpoint_v2(v2, &codecs, limits).unwrap();
    assert_eq!(
        restored_v2.resource_allocation_mode(eager_resource),
        Ok(FlowAllocationMode::Eager)
    );
}
