use kairo_ecs_des::{
    FlowCheckpointCodecs, FlowCheckpointError, FlowCheckpointLimits, FlowRuntime,
    LifecycleTransition, PreemptionStrategy, RequestState,
};
use kairo_ecs_types::{SimDuration, SimTime};

fn tick(value: u128) -> SimTime {
    SimTime::from_ticks(value)
}

#[test]
fn pending_disable_closes_completion_collision_regardless_of_scheduler_order() {
    for (disable_priority, submit_priority) in
        [(-10, 10), (10, -10), (i32::MAX, i32::MIN), (10, 10)]
    {
        let mut flow = FlowRuntime::new();
        let resource = flow.create_resource(1).unwrap();
        let first_owner = flow.spawn_actor().unwrap();
        let next_owner = flow.spawn_actor().unwrap();
        let first = flow
            .create_work(first_owner, SimDuration::from_ticks(3), "first", ())
            .unwrap();
        let next = flow
            .create_work(next_owner, SimDuration::from_ticks(2), "next", ())
            .unwrap();
        let first_request = flow
            .acquire(resource)
            .owner(first_owner)
            .timed_work(first)
            .submit()
            .unwrap();
        flow.run_for(1).unwrap();
        let next_request = flow
            .acquire(resource)
            .owner(next_owner)
            .timed_work(next)
            .at(tick(3))
            .scheduler_priority(submit_priority)
            .submit()
            .unwrap();
        flow.schedule_resource_grant_enabled(resource, false, tick(3), disable_priority)
            .unwrap();
        flow.schedule_resource_grant_enabled(resource, true, tick(5), 0)
            .unwrap();

        let run = flow.run_for(16).unwrap();
        let first_completion = run
            .dispatches
            .iter()
            .flat_map(|d| &d.records)
            .find(|r| r.request == first_request && r.transition == LifecycleTransition::Completed)
            .expect("first work completes");
        assert_eq!(first_completion.at, tick(3));
        assert!(!run.dispatches.iter().flat_map(|d| &d.records).any(|r| {
            r.request == next_request
                && r.transition == LifecycleTransition::Granted
                && r.at == tick(3)
        }));
        let next_grant = run
            .dispatches
            .iter()
            .flat_map(|d| &d.records)
            .find(|r| r.request == next_request && r.transition == LifecycleTransition::Granted)
            .expect("enable should grant the queued request");
        assert_eq!(next_grant.at, tick(5));
        assert_eq!(flow.resource_grant_enabled(resource), Ok(true));
        assert_eq!(
            flow.request(next_request).unwrap().state,
            RequestState::Completed
        );
    }
}

#[test]
fn closure_drains_active_low_work_before_preempting_and_suspending() {
    let mut flow = FlowRuntime::new();
    let resource = flow.create_resource(1).unwrap();
    let low_owner = flow.spawn_actor().unwrap();
    let urgent_owner = flow.spawn_actor().unwrap();
    let low_work = flow
        .create_work(low_owner, SimDuration::from_ticks(10), "low", ())
        .unwrap();
    let urgent_work = flow
        .create_work(urgent_owner, SimDuration::from_ticks(2), "urgent", ())
        .unwrap();
    let low = flow
        .acquire(resource)
        .owner(low_owner)
        .timed_work(low_work)
        .at(tick(1))
        .priority(10)
        .preemptible(PreemptionStrategy::Suspend)
        .submit()
        .unwrap();
    let urgent = flow
        .acquire(resource)
        .owner(urgent_owner)
        .timed_work(urgent_work)
        .at(tick(3))
        .priority(1)
        .can_preempt(true)
        .submit()
        .unwrap();
    flow.schedule_resource_grant_enabled(resource, false, tick(3), i32::MAX)
        .unwrap();
    flow.schedule_resource_grant_enabled(resource, true, tick(5), 0)
        .unwrap();
    let run = flow.run_for(32).unwrap();
    let rows: Vec<_> = run.dispatches.iter().flat_map(|d| &d.records).collect();
    assert!(!rows.iter().any(|r| {
        r.at < tick(5)
            && ((r.request == low && r.transition == LifecycleTransition::Preempted)
                || (r.request == urgent && r.transition == LifecycleTransition::Granted))
    }));
    for (request, transition, at) in [
        (low, LifecycleTransition::Granted, 1),
        (low, LifecycleTransition::Preempted, 5),
        (urgent, LifecycleTransition::Granted, 5),
        (urgent, LifecycleTransition::Completed, 7),
        (low, LifecycleTransition::Resumed, 7),
        (low, LifecycleTransition::Completed, 13),
    ] {
        assert!(rows
            .iter()
            .any(|r| { r.request == request && r.transition == transition && r.at == tick(at) }));
    }
    let preempted = rows
        .iter()
        .find(|r| r.request == low && r.transition == LifecycleTransition::Preempted)
        .unwrap();
    assert_eq!(preempted.at, tick(5));
    assert_eq!(preempted.snapshot.work, Some(low_work));
    let progress = preempted.snapshot.progress.as_ref().unwrap();
    assert_eq!(progress.original_duration, SimDuration::from_ticks(10));
    assert_eq!(progress.useful_elapsed, SimDuration::from_ticks(4));
    assert_eq!(progress.remaining, SimDuration::from_ticks(6));
    let final_progress = flow.work_progress(low_work).unwrap();
    assert_eq!(
        final_progress.original_duration,
        SimDuration::from_ticks(10)
    );
    assert_eq!(final_progress.useful_elapsed, SimDuration::from_ticks(10));
    assert_eq!(final_progress.remaining, SimDuration::ZERO);
}

#[test]
fn admission_frontier_duplicate_identity_and_remove_conflicts_are_explicit() {
    let mut flow = FlowRuntime::new();
    let resource = flow.create_resource(1).unwrap();
    let stale = flow.create_resource(1).unwrap();
    assert!(flow
        .schedule_resource_grant_enabled(resource, false, tick(0), 0)
        .is_ok());
    assert_eq!(
        flow.schedule_resource_grant_enabled(resource, true, tick(0), 1),
        Err(kairo_ecs_des::ResourceGrantAdmissionError::DuplicateBoundary)
    );
    flow.run_for(1).unwrap();
    flow.remove_resource(stale).unwrap();
    flow.run_for(1).unwrap();
    assert_eq!(
        flow.schedule_resource_grant_enabled(stale, false, tick(1), 0),
        Err(kairo_ecs_des::ResourceGrantAdmissionError::Flow(
            kairo_ecs_des::FlowError::InvalidResource
        ))
    );
    assert_eq!(
        flow.schedule_resource_grant_enabled(resource, true, tick(0), 0),
        Err(kairo_ecs_des::ResourceGrantAdmissionError::FrontierReached)
    );

    let mut remove_first = FlowRuntime::new();
    let remove_resource = remove_first.create_resource(1).unwrap();
    remove_first.remove_resource(remove_resource).unwrap();
    assert_eq!(
        remove_first.schedule_resource_grant_enabled(remove_resource, false, tick(1), 0),
        Err(kairo_ecs_des::ResourceGrantAdmissionError::Flow(
            kairo_ecs_des::FlowError::ResourceInUse
        ))
    );
    let mut gate_first = FlowRuntime::new();
    let gate_resource = gate_first.create_resource(1).unwrap();
    gate_first
        .schedule_resource_grant_enabled(gate_resource, false, tick(1), 0)
        .unwrap();
    assert_eq!(
        gate_first.remove_resource(gate_resource),
        Err(kairo_ecs_des::FlowError::ResourceInUse)
    );
}

#[test]
fn enable_dispatch_wakes_waiter_and_no_gate_control_remains_enabled() {
    let mut flow = FlowRuntime::new();
    let resource = flow.create_resource(1).unwrap();
    let owner = flow.spawn_actor().unwrap();
    let work = flow
        .create_work(owner, SimDuration::from_ticks(4), "work", ())
        .unwrap();
    let request = flow
        .acquire(resource)
        .owner(owner)
        .timed_work(work)
        .at(tick(1))
        .scheduler_priority(-10)
        .submit()
        .unwrap();
    flow.schedule_resource_grant_enabled(resource, false, tick(1), 0)
        .unwrap();
    flow.schedule_resource_grant_enabled(resource, true, tick(2), 0)
        .unwrap();
    let run = flow.run_for(16).unwrap();
    assert!(run
        .dispatches
        .iter()
        .flat_map(|d| &d.records)
        .any(|r| { r.request == request && r.transition == LifecycleTransition::Granted }));
    assert_eq!(flow.resource_grant_enabled(resource), Ok(true));
}

#[test]
fn suspended_work_does_not_resume_at_closure_completion_and_resumes_on_enable() {
    let mut flow = FlowRuntime::new();
    let resource = flow.create_resource(1).unwrap();
    let low_owner = flow.spawn_actor().unwrap();
    let urgent_owner = flow.spawn_actor().unwrap();
    let low_work = flow
        .create_work(low_owner, SimDuration::from_ticks(10), "low", ())
        .unwrap();
    let urgent_work = flow
        .create_work(urgent_owner, SimDuration::from_ticks(2), "urgent", ())
        .unwrap();
    let low = flow
        .acquire(resource)
        .owner(low_owner)
        .timed_work(low_work)
        .priority(10)
        .preemptible(PreemptionStrategy::Suspend)
        .submit()
        .unwrap();
    let urgent = flow
        .acquire(resource)
        .owner(urgent_owner)
        .timed_work(urgent_work)
        .at(tick(4))
        .priority(1)
        .can_preempt(true)
        .submit()
        .unwrap();
    flow.schedule_resource_grant_enabled(resource, false, tick(6), 0)
        .unwrap();
    flow.schedule_resource_grant_enabled(resource, true, tick(8), 0)
        .unwrap();
    let run = flow.run_for(32).unwrap();
    let rows: Vec<_> = run.dispatches.iter().flat_map(|d| &d.records).collect();
    let preempted = rows
        .iter()
        .find(|r| r.request == low && r.transition == LifecycleTransition::Preempted)
        .unwrap();
    assert_eq!(preempted.at, tick(4));
    let progress = preempted.snapshot.progress.as_ref().unwrap();
    assert_eq!(progress.useful_elapsed, SimDuration::from_ticks(4));
    assert_eq!(progress.remaining, SimDuration::from_ticks(6));
    assert!(rows.iter().any(|r| {
        r.request == urgent && r.transition == LifecycleTransition::Completed && r.at == tick(6)
    }));
    assert!(!rows.iter().any(|r| {
        r.request == low && r.transition == LifecycleTransition::Resumed && r.at == tick(6)
    }));
    assert!(rows.iter().any(|r| {
        r.request == low && r.transition == LifecycleTransition::Resumed && r.at == tick(8)
    }));
}

#[test]
fn ready_urgent_submission_precedes_enable_without_transient_low_resume() {
    let mut flow = FlowRuntime::new();
    let resource = flow.create_resource(1).unwrap();
    let low_owner = flow.spawn_actor().unwrap();
    let first_urgent_owner = flow.spawn_actor().unwrap();
    let ready_owner = flow.spawn_actor().unwrap();
    let low_work = flow
        .create_work(low_owner, SimDuration::from_ticks(10), "low-ready", ())
        .unwrap();
    let first_urgent_work = flow
        .create_work(
            first_urgent_owner,
            SimDuration::from_ticks(2),
            "urgent-first",
            (),
        )
        .unwrap();
    let ready_work = flow
        .create_work(ready_owner, SimDuration::from_ticks(2), "urgent-ready", ())
        .unwrap();
    let low = flow
        .acquire(resource)
        .owner(low_owner)
        .timed_work(low_work)
        .at(tick(1))
        .priority(10)
        .preemptible(PreemptionStrategy::Suspend)
        .submit()
        .unwrap();
    let first_urgent = flow
        .acquire(resource)
        .owner(first_urgent_owner)
        .timed_work(first_urgent_work)
        .at(tick(3))
        .priority(1)
        .can_preempt(true)
        .submit()
        .unwrap();
    flow.schedule_resource_grant_enabled(resource, false, tick(4), 0)
        .unwrap();
    flow.schedule_resource_grant_enabled(resource, true, tick(5), 10)
        .unwrap();
    let first = flow.run_for(4).unwrap();
    let first_rows: Vec<_> = first.dispatches.iter().flat_map(|d| &d.records).collect();
    assert!(first_rows.iter().any(|r| {
        r.request == low && r.transition == LifecycleTransition::Preempted && r.at == tick(3)
    }));
    assert_eq!(flow.request(low).unwrap().state, RequestState::Suspended);
    assert!(first_rows.iter().any(|r| {
        r.request == first_urgent && r.transition == LifecycleTransition::Granted && r.at == tick(3)
    }));

    let ready = flow
        .acquire(resource)
        .owner(ready_owner)
        .timed_work(ready_work)
        .at(tick(5))
        .scheduler_priority(-10)
        .priority(0)
        .can_preempt(true)
        .submit()
        .unwrap();
    let mut rows: Vec<_> = first_rows.into_iter().cloned().collect();
    for _ in 0..16 {
        let dispatch = flow.step().unwrap().expect("scheduled dispatch");
        let is_at_five = dispatch.at == tick(5);
        rows.extend(dispatch.records.iter().cloned());
        if is_at_five
            && dispatch
                .records
                .iter()
                .any(|r| r.request == ready && r.transition == LifecycleTransition::Granted)
        {
            break;
        }
    }
    assert!(rows.iter().any(|r| {
        r.request == ready && r.transition == LifecycleTransition::Granted && r.at == tick(5)
    }));
    assert!(!rows.iter().any(|r| {
        r.request == low
            && matches!(
                r.transition,
                LifecycleTransition::Resumed | LifecycleTransition::Preempted
            )
            && r.at == tick(5)
    }));
}

#[test]
fn enable_at_h_observes_active_claim_before_work_completes() {
    let mut flow = FlowRuntime::new();
    let resource = flow.create_resource(1).unwrap();
    let owner = flow.spawn_actor().unwrap();
    let work = flow
        .create_work(owner, SimDuration::from_ticks(10), "enable-active", ())
        .unwrap();
    let request = flow
        .acquire(resource)
        .owner(owner)
        .timed_work(work)
        .at(tick(1))
        .scheduler_priority(-10)
        .submit()
        .unwrap();
    flow.schedule_resource_grant_enabled(resource, false, tick(1), 0)
        .unwrap();
    let enable = flow
        .schedule_resource_grant_enabled(resource, true, tick(5), 0)
        .unwrap();
    flow.run_for(2).unwrap();
    assert_eq!(flow.request(request).unwrap().state, RequestState::Queued);

    let mut observed = Vec::new();
    for _ in 0..16 {
        let dispatch = flow.step().unwrap().expect("scheduled dispatch");
        let done = dispatch.event == enable;
        observed.extend(dispatch.records.iter().cloned());
        if done {
            break;
        }
    }
    assert_eq!(flow.now(), tick(5));
    assert_eq!(flow.request(request).unwrap().state, RequestState::Active);
    let active = flow.resource(resource).unwrap();
    let allocation = active
        .allocations
        .iter()
        .find(|allocation| allocation.request == request)
        .unwrap();
    assert_eq!(allocation.work, Some(work));
    assert_eq!(allocation.granted_at, tick(5));
    assert_eq!(allocation.segment_started_at, tick(5));
    let progress = flow.work_progress(work).unwrap();
    assert_eq!(progress.original_duration, SimDuration::from_ticks(10));
    assert_eq!(progress.useful_elapsed, SimDuration::ZERO);
    assert_eq!(progress.remaining, SimDuration::from_ticks(10));
    assert!(!observed.iter().any(|record| {
        record.request == request && record.transition == LifecycleTransition::Completed
    }));
}

#[test]
fn disabled_resource_still_processes_cancel_release_and_deadline() {
    let mut cancelled = FlowRuntime::new();
    let resource = cancelled.create_resource(1).unwrap();
    let owner = cancelled.spawn_actor().unwrap();
    let work = cancelled
        .create_work(owner, SimDuration::from_ticks(5), "cancel", ())
        .unwrap();
    let request = cancelled
        .acquire(resource)
        .owner(owner)
        .timed_work(work)
        .at(tick(1))
        .scheduler_priority(-10)
        .submit()
        .unwrap();
    cancelled
        .schedule_resource_grant_enabled(resource, false, tick(1), 0)
        .unwrap();
    cancelled.cancel(request, tick(2)).unwrap();
    cancelled.run_for(16).unwrap();
    assert_eq!(
        cancelled.request(request).unwrap().state,
        RequestState::Cancelled
    );

    let mut released = FlowRuntime::new();
    let resource = released.create_resource(1).unwrap();
    let owner = released.spawn_actor().unwrap();
    let work = released
        .create_work(owner, SimDuration::from_ticks(8), "release", ())
        .unwrap();
    let request = released
        .acquire(resource)
        .owner(owner)
        .timed_work(work)
        .submit()
        .unwrap();
    released.run_for(1).unwrap();
    let lease = released.request(request).unwrap().lease.unwrap();
    released
        .schedule_resource_grant_enabled(resource, false, tick(1), 0)
        .unwrap();
    released.release(lease, tick(2)).unwrap();
    released.run_for(16).unwrap();
    assert_eq!(
        released.request(request).unwrap().state,
        RequestState::Released
    );

    let mut deadline = FlowRuntime::new();
    let resource = deadline.create_resource(1).unwrap();
    let owner = deadline.spawn_actor().unwrap();
    let work = deadline
        .create_work(owner, SimDuration::from_ticks(5), "deadline", ())
        .unwrap();
    let request = deadline
        .acquire(resource)
        .owner(owner)
        .timed_work(work)
        .at(tick(1))
        .deadline(tick(2))
        .submit()
        .unwrap();
    deadline
        .schedule_resource_grant_enabled(resource, false, tick(1), 0)
        .unwrap();
    deadline.run_for(16).unwrap();
    assert_eq!(
        deadline.request(request).unwrap().state,
        RequestState::TimedOut
    );
}

#[test]
fn empty_enable_and_disabled_idle_removal_have_no_invented_work_or_stale_gate() {
    let mut flow = FlowRuntime::new();
    let resource = flow.create_resource(1).unwrap();
    flow.schedule_resource_grant_enabled(resource, true, tick(1), 0)
        .unwrap();
    let run = flow.run_for(1).unwrap();
    assert!(run.dispatches.iter().all(|d| d.records.is_empty()));
    assert_eq!(flow.resource_grant_enabled(resource), Ok(true));

    flow.schedule_resource_grant_enabled(resource, false, tick(2), 0)
        .unwrap();
    flow.run_for(1).unwrap();
    assert_eq!(flow.resource_grant_enabled(resource), Ok(false));
    flow.remove_resource(resource).unwrap();
    flow.run_for(1).unwrap();
    let reused = flow.create_resource(1).unwrap();
    assert_ne!(resource, reused);
    assert_eq!(flow.resource_grant_enabled(reused), Ok(true));
}

#[test]
fn v1_capture_rejects_pending_and_disabled_gates_but_keeps_no_gate_control() {
    let codecs = FlowCheckpointCodecs::new();
    let mut pending = FlowRuntime::new();
    let pending_resource = pending.create_resource(1).unwrap();
    pending
        .schedule_resource_grant_enabled(pending_resource, false, tick(1), 0)
        .unwrap();
    assert!(matches!(
        pending.capture_checkpoint(&codecs, FlowCheckpointLimits::default()),
        Err(FlowCheckpointError::InvalidState(_))
    ));

    let mut disabled = FlowRuntime::new();
    let disabled_resource = disabled.create_resource(1).unwrap();
    disabled
        .schedule_resource_grant_enabled(disabled_resource, false, tick(1), 0)
        .unwrap();
    disabled.run_for(1).unwrap();
    assert_eq!(
        disabled.resource_grant_enabled(disabled_resource),
        Ok(false)
    );
    assert!(matches!(
        disabled.capture_checkpoint(&codecs, FlowCheckpointLimits::default()),
        Err(FlowCheckpointError::InvalidState(_))
    ));

    let legacy = FlowRuntime::new();
    assert!(legacy
        .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
        .is_ok());
}
