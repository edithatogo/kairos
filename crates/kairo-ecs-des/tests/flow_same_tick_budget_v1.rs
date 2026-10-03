use std::num::NonZeroU64;

use kairo_ecs_des::{
    FlowConfig, FlowDispatch, FlowError, FlowRuntime, LifecycleTransition as L, RequestState,
    WorkHandlers, WorkProgress,
};
use kairo_ecs_types::{SimDuration, SimTime};

fn t(n: u128) -> SimTime {
    SimTime::from_ticks(n)
}
fn flow(limit: u64) -> FlowRuntime {
    FlowRuntime::with_config(FlowConfig {
        max_same_tick_flow_transitions: NonZeroU64::new(limit).unwrap(),
    })
}
fn next(f: &mut FlowRuntime) -> FlowDispatch {
    f.step().unwrap().unwrap()
}
fn rows(d: &FlowDispatch, expected: &[L]) {
    assert_eq!(
        d.records.iter().map(|r| r.transition).collect::<Vec<_>>(),
        expected
    );
    for (ordinal, r) in d.records.iter().enumerate() {
        assert_eq!(r.at, d.at);
        assert_eq!(r.causal_event_id, d.event);
        assert_eq!(r.transition_ordinal, ordinal as u32);
    }
}
fn budget(f: &FlowRuntime, tick: u128, consumed: u64) {
    let s = f.budget_snapshot();
    assert_eq!(s.tick, Some(t(tick)));
    assert_eq!(s.consumed, consumed);
    assert!(s.halted.is_none());
}
fn exceeded(f: &mut FlowRuntime, at: u128, limit: u64) {
    assert_eq!(
        f.step().unwrap_err(),
        FlowError::SameTickBudgetExceeded {
            at_ticks: at,
            limit
        }
    );
}
fn on_cancel(context: &mut Vec<WorkProgress>, progress: &WorkProgress) {
    context.push(progress.clone());
}
fn callback_setup() -> (FlowRuntime, kairo_ecs_des::WorkId, kairo_ecs_des::RequestId) {
    let mut f = flow(2);
    let owner = f.spawn_actor().unwrap();
    let resource = f.create_resource(1).unwrap();
    f.register_work_handlers(
        "callback",
        WorkHandlers {
            on_cancel: Some(on_cancel),
            ..WorkHandlers::default()
        },
    )
    .unwrap();
    let work = f
        .create_work(
            owner,
            SimDuration::from_ticks(10),
            "callback",
            Vec::<WorkProgress>::new(),
        )
        .unwrap();
    let request = f
        .acquire(resource)
        .owner(owner)
        .at(t(0))
        .timed_work(work)
        .submit()
        .unwrap();
    rows(&next(&mut f), &[L::Queued, L::Granted]);
    (f, work, request)
}

#[test]
fn defaults_and_constructor_only_override() {
    for f in [FlowRuntime::new(), FlowRuntime::default()] {
        let s = f.budget_snapshot();
        assert_eq!(s.limit.get(), 100_000);
        assert_eq!(s.tick, None);
        assert_eq!(s.consumed, 0);
        assert_eq!(s.halted, None);
        assert_eq!(s.scheduler, kairo_ecs_core::SchedulerStats::default());
    }
    assert_eq!(flow(7).budget_snapshot().limit.get(), 7);
    assert!(NonZeroU64::new(0).is_none());
}

#[test]
fn timed_admission_uses_effective_dispatch_timestamp() {
    let mut f = flow(2);
    let owner = f.spawn_actor().unwrap();
    let r = f.create_resource(1).unwrap();
    let work = f
        .create_work(owner, SimDuration::from_ticks(2), "plain", ())
        .unwrap();
    f.acquire(r)
        .owner(owner)
        .at(t(5))
        .timed_work(work)
        .submit()
        .unwrap();
    assert_eq!(f.now(), t(0));
    let dispatch = next(&mut f);
    rows(&dispatch, &[L::Queued, L::Granted]);
    assert_eq!(dispatch.at, t(5));
    assert_eq!(f.now(), t(5));
    assert_eq!(f.work_progress(work).unwrap().completion_at, Some(t(7)));
    budget(&f, 5, 2);
}

#[test]
fn first_admission_overflow_retains_head_and_permanently_halts() {
    let mut f = flow(1);
    let owner = f.spawn_actor().unwrap();
    let resource = f.create_resource(1).unwrap();
    let request = f.submit(resource, owner, t(5)).unwrap();
    let before = f.budget_snapshot();
    let membership = f.resource(resource).unwrap();
    let pending = f.request(request).unwrap();
    exceeded(&mut f, 5, 1);
    let halted = f.budget_snapshot();
    let halt = halted.halted.unwrap();
    assert_eq!(halt.at, t(5));
    assert_eq!(halt.consumed, 0);
    assert_eq!(halt.required_cost, 2);
    assert_eq!(halt.pending.at, t(5));
    assert_eq!(halt.pending.sequence, 0);
    assert_eq!(halt.pending.kind, 4000);
    assert_eq!(halted.scheduler, before.scheduler);
    assert_eq!(halted.tick, None);
    assert_eq!(halted.consumed, 0);
    assert_eq!(f.now(), t(0));
    assert_eq!(pending.state, RequestState::Pending);
    for _ in 0..3 {
        exceeded(&mut f, 5, 1);
        assert_eq!(f.budget_snapshot(), halted);
        assert_eq!(f.request(request).unwrap(), pending);
        assert_eq!(f.resource(resource).unwrap(), membership);
    }
    for count in [0, 1] {
        assert_eq!(
            f.run_for(count).unwrap_err(),
            FlowError::SameTickBudgetExceeded {
                at_ticks: 5,
                limit: 1
            }
        );
    }
    assert_eq!(f.spawn_actor().unwrap_err(), FlowError::RunHalted);
    assert_eq!(f.create_resource(0).unwrap_err(), FlowError::RunHalted);
    assert_eq!(
        f.submit(resource, owner, t(6)).unwrap_err(),
        FlowError::RunHalted
    );
    assert_eq!(
        f.create_work(owner, SimDuration::from_ticks(1), "blocked", ())
            .unwrap_err(),
        FlowError::RunHalted
    );
    assert_eq!(f.budget_snapshot(), halted);
}

#[test]
fn release_and_replacement_are_one_atomic_budget_plan() {
    let mut f = flow(4);
    let owner = f.spawn_actor().unwrap();
    let r = f.create_resource(1).unwrap();
    let dummy = f.create_resource(0).unwrap();
    let a = f.submit(r, owner, t(0)).unwrap();
    let b = f.submit(r, owner, t(0)).unwrap();
    rows(&next(&mut f), &[L::Queued, L::Granted]);
    rows(&next(&mut f), &[L::Queued]);
    let lease = f.request(a).unwrap().lease.unwrap();
    f.release(lease, t(0)).unwrap();
    f.submit(dummy, owner, t(1)).unwrap();
    let before = f.budget_snapshot();
    let resource = f.resource(r).unwrap();
    let requests = [f.request(a).unwrap(), f.request(b).unwrap()];
    exceeded(&mut f, 0, 4);
    let halt = f.budget_snapshot().halted.unwrap();
    assert_eq!((halt.consumed, halt.required_cost), (3, 2));
    assert_eq!(halt.pending.kind, 4000);
    assert_eq!(f.budget_snapshot().scheduler, before.scheduler);
    assert_eq!(f.resource(r).unwrap(), resource);
    assert_eq!([f.request(a).unwrap(), f.request(b).unwrap()], requests);
    exceeded(&mut f, 0, 4);
    assert_eq!(f.now(), t(0));
    assert_eq!(f.release(lease, t(1)).unwrap_err(), FlowError::RunHalted);
}

#[test]
fn exact_limit_and_normal_later_tick_reset_respect_max_events() {
    let mut f = flow(2);
    let owner = f.spawn_actor().unwrap();
    let r = f.create_resource(1).unwrap();
    let dummy = f.create_resource(0).unwrap();
    let a = f.submit(r, owner, t(0)).unwrap();
    next(&mut f);
    budget(&f, 0, 2);
    f.release(f.request(a).unwrap().lease.unwrap(), t(1))
        .unwrap();
    f.submit(dummy, owner, t(2)).unwrap();
    let one = f.run_for(1).unwrap();
    assert_eq!(one.dispatches.len(), 1);
    rows(&one.dispatches[0], &[L::Released]);
    assert_eq!(f.now(), t(1));
    budget(&f, 1, 1);
    let two = f.run_for(1).unwrap();
    assert_eq!(two.dispatches.len(), 1);
    rows(&two.dispatches[0], &[L::Queued]);
    assert_eq!(f.now(), t(2));
    budget(&f, 2, 1);
}

#[test]
fn delivered_cancel_notification_counts_once_after_terminal_commit() {
    let (mut f, work, request) = callback_setup();
    f.cancel(request, t(1)).unwrap();
    rows(&next(&mut f), &[L::Cancelled]);
    assert!(f
        .work_context::<Vec<WorkProgress>>(work)
        .unwrap()
        .is_empty());
    budget(&f, 1, 1);
    let origin = f.work_progress(work).unwrap();
    let notification = next(&mut f);
    rows(&notification, &[]);
    assert!(notification.error.is_none());
    assert_eq!(notification.at, t(1));
    assert_eq!(
        f.work_context::<Vec<WorkProgress>>(work).unwrap(),
        &vec![origin]
    );
    budget(&f, 1, 2);
}

#[test]
fn blocked_notification_preserves_context_and_pending_token() {
    let (mut f, work, request) = callback_setup();
    let owner = f.spawn_actor().unwrap();
    let dummy = f.create_resource(0).unwrap();
    f.cancel(request, t(1)).unwrap();
    f.submit(dummy, owner, t(1)).unwrap();
    rows(&next(&mut f), &[L::Cancelled]);
    rows(&next(&mut f), &[L::Queued]);
    let before = f.budget_snapshot();
    let progress = f.work_progress(work).unwrap();
    exceeded(&mut f, 1, 2);
    let after = f.budget_snapshot();
    let halt = after.halted.unwrap();
    assert_eq!(
        (halt.consumed, halt.required_cost, halt.pending.kind),
        (2, 1, 4003)
    );
    assert_eq!(after.scheduler, before.scheduler);
    assert_eq!(f.work_progress(work).unwrap(), progress);
    assert_eq!(f.request(request).unwrap().state, RequestState::Cancelled);
    assert!(f
        .work_context::<Vec<WorkProgress>>(work)
        .unwrap()
        .is_empty());
    exceeded(&mut f, 1, 2);
    assert_eq!(f.budget_snapshot(), after);
    assert!(f
        .work_context::<Vec<WorkProgress>>(work)
        .unwrap()
        .is_empty());
}

#[test]
fn predicted_obsolete_completion_has_zero_cost_and_natural_tick_reset() {
    let (mut f, work, request) = callback_setup();
    f.cancel(request, t(1)).unwrap();
    next(&mut f);
    next(&mut f);
    budget(&f, 1, 2);
    let context = f.work_context::<Vec<WorkProgress>>(work).unwrap().clone();
    let stale = next(&mut f);
    assert_eq!(stale.at, t(10));
    rows(&stale, &[]);
    assert!(stale.error.is_none());
    assert_eq!(f.now(), t(10));
    budget(&f, 10, 0);
    assert_eq!(f.work_context::<Vec<WorkProgress>>(work).unwrap(), &context);
    assert_eq!(f.request(request).unwrap().state, RequestState::Cancelled);
    assert!(f.step().unwrap().is_none());
}

#[test]
fn due_boundary_and_rejected_cancel_eight_independent_ordering_cases() {
    for (early_cancel, priority, cancel_head, sequence) in [
        (true, 0, true, 1),
        (false, 0, false, 2),
        (true, 1, false, 3),
        (false, -1, true, 3),
    ] {
        for dummy_count in [1, 2] {
            let mut f = flow(3);
            let owner = f.spawn_actor().unwrap();
            let r = f.create_resource(1).unwrap();
            let dummy = f.create_resource(0).unwrap();
            let work = f
                .create_work(owner, SimDuration::from_ticks(2), "plain", ())
                .unwrap();
            let a = f
                .acquire(r)
                .owner(owner)
                .at(t(0))
                .timed_work(work)
                .submit()
                .unwrap();
            if early_cancel {
                f.cancel_with_scheduler_priority(a, t(2), priority).unwrap();
            }
            let b = f.submit(r, owner, t(0)).unwrap();
            rows(&next(&mut f), &[L::Queued, L::Granted]);
            rows(&next(&mut f), &[L::Queued]);
            if !early_cancel {
                f.cancel_with_scheduler_priority(a, t(2), priority).unwrap();
            }
            for _ in 0..dummy_count {
                f.acquire(dummy)
                    .owner(owner)
                    .at(t(2))
                    .scheduler_priority(-2)
                    .submit()
                    .unwrap();
            }
            for _ in 0..dummy_count {
                rows(&next(&mut f), &[L::Queued]);
            }
            budget(&f, 2, dummy_count);
            let before = f.budget_snapshot();
            let resource = f.resource(r).unwrap();
            let requests = [f.request(a).unwrap(), f.request(b).unwrap()];
            let progress = f.work_progress(work).unwrap();
            if dummy_count == 2 {
                exceeded(&mut f, 2, 3);
                let after = f.budget_snapshot();
                let halt = after.halted.unwrap();
                assert_eq!((halt.consumed, halt.required_cost), (2, 2));
                assert_eq!(halt.pending.kind, if cancel_head { 4000 } else { 4001 });
                assert_eq!(halt.pending.sequence, sequence);
                assert_eq!(
                    halt.pending.priority,
                    if cancel_head { priority } else { 0 }
                );
                assert_eq!(after.scheduler, before.scheduler);
                assert_eq!(after.tick, before.tick);
                assert_eq!(after.consumed, before.consumed);
                assert_eq!(f.resource(r).unwrap(), resource);
                assert_eq!([f.request(a).unwrap(), f.request(b).unwrap()], requests);
                assert_eq!(f.work_progress(work).unwrap(), progress);
                assert_eq!(f.work_context::<()>(work).unwrap(), &());
                exceeded(&mut f, 2, 3);
                assert_eq!(f.budget_snapshot(), after);
            } else {
                let boundary = next(&mut f);
                rows(&boundary, &[L::Completed, L::Granted]);
                assert_eq!(boundary.records[0].request, a);
                assert_eq!(boundary.records[1].request, b);
                assert_eq!(
                    boundary.error,
                    if cancel_head {
                        Some(FlowError::TerminalRequest)
                    } else {
                        None
                    }
                );
                assert_eq!(f.request(a).unwrap().state, RequestState::Completed);
                assert_eq!(f.request(b).unwrap().state, RequestState::Active);
                budget(&f, 2, 3);
                let last = next(&mut f);
                rows(&last, &[]);
                assert_eq!(
                    last.error,
                    if cancel_head {
                        None
                    } else {
                        Some(FlowError::TerminalRequest)
                    }
                );
                budget(&f, 2, 3);
                assert!(f.step().unwrap().is_none());
            }
        }
    }
}
