use std::num::{NonZeroU64, NonZeroUsize};

use kairo_ecs_des::{
    FlowAcquireCommand, FlowBatchReceipt, FlowCallbackCause, FlowCallbackConfig,
    FlowCallbackSnapshot, FlowCommandSink, FlowCommandTicket, FlowConfig, FlowContinuations,
    FlowDispatch, FlowError, FlowOwnedCommand, FlowRequestRef, FlowRuntime, LeaseId,
    LifecycleTransition as L, RequestId, RequestState, ResourceId, WorkHandlers, WorkId,
    WorkProgress,
};
use kairo_ecs_types::{EntityId, EventId, EventKind, SimDuration, SimTime};

const DOMAIN: u32 = 7000;
fn t(n: u128) -> SimTime {
    SimTime::from_ticks(n)
}
fn kind(n: u32) -> EventKind {
    EventKind::custom(n)
}
fn flow(budget: u64, cap: usize) -> FlowRuntime {
    FlowRuntime::with_configs(
        FlowConfig {
            max_same_tick_flow_transitions: NonZeroU64::new(budget).unwrap(),
        },
        FlowCallbackConfig {
            max_callback_commands: NonZeroUsize::new(cap).unwrap(),
        },
    )
}
fn next(f: &mut FlowRuntime) -> FlowDispatch {
    let d = f.step().expect("legal dispatch").expect("pending dispatch");
    assert!(d.error.is_none(), "unexpected semantic error");
    for (ordinal, r) in d.records.iter().enumerate() {
        assert_eq!(r.at, d.at);
        assert_eq!(r.causal_event_id, d.event);
        assert_eq!(r.transition_ordinal, ordinal as u32);
    }
    d
}
fn rows(d: &FlowDispatch) -> Vec<L> {
    d.records.iter().map(|r| r.transition).collect()
}
fn rejected(d: &FlowDispatch, expected: FlowError, ticket: Option<FlowCommandTicket>) {
    assert_eq!(d.callback_batches.len(), 1);
    match &d.callback_batches[0] {
        FlowBatchReceipt::Rejected(r) => {
            assert_eq!(r.error, expected);
            assert_eq!(r.failed_ticket, ticket);
        }
        FlowBatchReceipt::Accepted(_) => panic!("batch must be rejected"),
    }
}
#[derive(Clone, Copy)]
enum Mode {
    Empty,
    Complete,
    Duplicate,
    Tickets,
    NonAcquire,
    Remember,
    Foreign,
    Reserved(u32),
    Cap(usize, u8),
}
struct Context {
    mode: Mode,
    owner: EntityId,
    resource: ResourceId,
    target: WorkId,
    request: Option<RequestId>,
    lease: Option<LeaseId>,
    second_lease: Option<LeaseId>,
    second_target: Option<WorkId>,
    calls: usize,
    markers: Vec<u8>,
    tickets: Vec<FlowCommandTicket>,
    errors: Vec<FlowError>,
    origin: Option<EventId>,
    ordinal: Option<u32>,
    delivery: Option<EventId>,
    transition: Option<L>,
    captured_busy: u128,
}
fn context(mode: Mode, owner: EntityId, resource: ResourceId, target: WorkId) -> Context {
    Context {
        mode,
        owner,
        resource,
        target,
        request: None,
        lease: None,
        second_lease: None,
        second_target: None,
        calls: 0,
        markers: vec![],
        tickets: vec![],
        errors: vec![],
        origin: None,
        ordinal: None,
        delivery: None,
        transition: None,
        captured_busy: 0,
    }
}
fn acquire(c: &Context, at: SimTime, priority: i32) -> FlowOwnedCommand {
    acquire_target(c, c.target, at, priority)
}
fn acquire_target(c: &Context, target: WorkId, at: SimTime, priority: i32) -> FlowOwnedCommand {
    FlowOwnedCommand::Acquire(FlowAcquireCommand {
        resource: c.resource,
        owner: c.owner,
        work: Some(target),
        at,
        priority_level: 0,
        deadline: None,
        scheduler_priority: priority,
        timed: true,
        can_preempt: false,
        preemptible: None,
    })
}
fn save(c: &mut Context, result: Result<FlowCommandTicket, FlowError>) {
    match result {
        Ok(ticket) => c.tickets.push(ticket),
        Err(e) => c.errors.push(e),
    }
}
fn callback(c: &mut Context, s: &FlowCallbackSnapshot, sink: &mut FlowCommandSink) {
    c.calls += 1;
    c.markers.push(2);
    c.origin = Some(s.origin);
    c.ordinal = s.origin_ordinal;
    c.delivery = Some(s.delivery.id);
    match &s.cause {
        FlowCallbackCause::Work {
            transition,
            progress,
        } => {
            c.transition = Some(*transition);
            c.captured_busy = progress.cumulative_busy.ticks();
        }
        FlowCallbackCause::Domain { kind: k } => {
            assert_eq!(*k, kind(DOMAIN));
        }
    }
    match c.mode {
        Mode::Empty => {}
        Mode::Complete => {
            let cmd = acquire(c, s.delivery.at, 0);
            save(c, sink.emit(cmd));
        }
        Mode::Duplicate => {
            for _ in 0..2 {
                let cmd = acquire(c, s.delivery.at, 0);
                save(c, sink.emit(cmd));
            }
        }
        Mode::Tickets => {
            let cmd = acquire(c, s.delivery.at, 1);
            let first = sink.emit(cmd).expect("issued acquire ticket");
            c.tickets.push(first);
            save(
                c,
                sink.emit(FlowOwnedCommand::Cancel {
                    request: FlowRequestRef::Submitted(first),
                    at: s.delivery.at,
                    scheduler_priority: 0,
                }),
            );
        }
        Mode::NonAcquire => {
            let first = sink
                .emit(FlowOwnedCommand::Domain {
                    work: s.work,
                    kind: kind(DOMAIN),
                    at: t(s.delivery.at.ticks() + 1),
                    scheduler_priority: 0,
                })
                .expect("domain ticket");
            c.tickets.push(first);
            save(
                c,
                sink.emit(FlowOwnedCommand::Cancel {
                    request: FlowRequestRef::Submitted(first),
                    at: s.delivery.at,
                    scheduler_priority: 0,
                }),
            );
        }
        Mode::Remember => {
            let command = acquire(c, t(3), 0);
            save(c, sink.emit(command));
            c.mode = Mode::Foreign;
        }
        Mode::Foreign => {
            let foreign = c.tickets[0];
            save(
                c,
                sink.emit(FlowOwnedCommand::Cancel {
                    request: FlowRequestRef::Submitted(foreign),
                    at: s.delivery.at,
                    scheduler_priority: 0,
                }),
            );
        }
        Mode::Reserved(code) => {
            save(
                c,
                sink.emit(FlowOwnedCommand::Domain {
                    work: s.work,
                    kind: kind(code),
                    at: s.delivery.at,
                    scheduler_priority: 0,
                }),
            );
        }
        Mode::Cap(n, command_kind) => {
            for i in 0..n {
                let cmd = match command_kind {
                    0 => FlowOwnedCommand::Reprioritize {
                        request: FlowRequestRef::Existing(c.request.unwrap()),
                        level: 1,
                        at: t(2),
                        scheduler_priority: 0,
                    },
                    1 => FlowOwnedCommand::Cancel {
                        request: FlowRequestRef::Existing(c.request.unwrap()),
                        at: t(2),
                        scheduler_priority: 0,
                    },
                    2 => acquire_target(
                        c,
                        if i % 2 == 0 {
                            c.target
                        } else {
                            c.second_target.unwrap()
                        },
                        t(2),
                        0,
                    ),
                    3 => FlowOwnedCommand::Release {
                        lease: if i % 2 == 0 {
                            c.lease.unwrap()
                        } else {
                            c.second_lease.unwrap()
                        },
                        at: t(2),
                    },
                    _ => FlowOwnedCommand::Domain {
                        work: s.work,
                        kind: kind(DOMAIN),
                        at: t(2),
                        scheduler_priority: 0,
                    },
                };
                // Intentionally ignore the poison and keep emitting.
                save(c, sink.emit(cmd));
            }
        }
    }
}
fn legacy(c: &mut Context, _: &WorkProgress) {
    c.markers.push(1);
}
fn setup(mode: Mode, cap: usize) -> (FlowRuntime, WorkId, WorkId) {
    let mut f = flow(100_000, cap);
    let owner = f.spawn_actor().unwrap();
    let resource = f.create_resource(1).unwrap();
    let target = f
        .create_work(owner, SimDuration::from_ticks(3), "target", ())
        .unwrap();
    f.register_domain_hook::<Context>("domain", kind(DOMAIN), callback)
        .unwrap();
    let work = f
        .create_work(
            owner,
            SimDuration::from_ticks(1),
            "domain",
            context(mode, owner, resource, target),
        )
        .unwrap();
    (f, work, target)
}
fn fire(f: &mut FlowRuntime, work: WorkId, at: u128) -> FlowDispatch {
    let event = f.schedule_domain(work, kind(DOMAIN), t(at), 0).unwrap();
    let d = next(f);
    assert_eq!(d.event, event);
    assert!(d.records.is_empty());
    d
}

#[test]
fn defaults_preserve_flow_config_shape_and_require_no_context_default() {
    let handlers = FlowContinuations::<Context>::default();
    assert!(
        handlers.on_resume.is_none()
            && handlers.on_restart.is_none()
            && handlers.on_abort.is_none()
            && handlers.on_cancel.is_none()
            && handlers.on_complete.is_none()
    );
    let c = FlowCallbackConfig::default();
    let copied = c;
    assert_eq!(c, copied);
    assert_eq!(c.max_callback_commands.get(), 1024);
    assert!(NonZeroUsize::new(0).is_none());
    let f = FlowRuntime::with_config(FlowConfig {
        max_same_tick_flow_transitions: NonZeroU64::new(7).unwrap(),
    });
    assert_eq!(f.budget_snapshot().limit.get(), 7);
}
#[test]
fn completion_stages_existing_b_work_with_real_receipts_and_causal_snapshot() {
    let mut f = flow(100_000, 8);
    let owner = f.spawn_actor().unwrap();
    let resource = f.create_resource(1).unwrap();
    let b = f
        .create_work(owner, SimDuration::from_ticks(3), "b-without-hook", ())
        .unwrap();
    f.register_work_continuations(
        "a",
        FlowContinuations {
            on_complete: Some(callback),
            ..FlowContinuations::default()
        },
    )
    .unwrap();
    let a = f
        .create_work(
            owner,
            SimDuration::from_ticks(2),
            "a",
            context(Mode::Complete, owner, resource, b),
        )
        .unwrap();
    let qa = f
        .acquire(resource)
        .owner(owner)
        .at(t(0))
        .timed_work(a)
        .submit()
        .unwrap();
    assert_eq!(rows(&next(&mut f)), vec![L::Queued, L::Granted]);
    let completed = next(&mut f);
    assert_eq!(completed.at, t(2));
    assert_eq!(rows(&completed), vec![L::Completed]);
    let notify = next(&mut f);
    assert_eq!(notify.at, t(2));
    assert!(notify.records.is_empty());
    let c = f.work_context::<Context>(a).unwrap();
    assert_eq!(c.calls, 1);
    assert_eq!(c.origin, Some(completed.event));
    assert_eq!(c.ordinal, Some(0));
    assert_eq!(c.delivery, Some(notify.event));
    assert_eq!(c.transition, Some(L::Completed));
    assert_eq!(c.captured_busy, 2);
    let (qb, submit_event) = match &notify.callback_batches[0] {
        FlowBatchReceipt::Accepted(v) => {
            assert_eq!(v.len(), 1);
            assert_eq!(v[0].ticket, c.tickets[0]);
            assert!(v[0].deadline_event.is_none());
            (v[0].request.unwrap(), v[0].event)
        }
        FlowBatchReceipt::Rejected(_) => panic!("valid staged B"),
    };
    assert_eq!(f.work(a).unwrap().request, Some(qa));
    assert_eq!(f.work(b).unwrap().request, Some(qb));
    assert_eq!(f.request(qa).unwrap().state, RequestState::Completed);
    let admit = next(&mut f);
    assert_eq!(admit.event, submit_event);
    assert_eq!(rows(&admit), vec![L::Queued, L::Granted]);
    let end = next(&mut f);
    assert_eq!(end.at, t(5));
    assert_eq!(rows(&end), vec![L::Completed]);
    assert!(f.step().unwrap().is_none());
}
#[test]
fn domain_context_and_reserved_ingress_are_explicit() {
    let (mut f, w, _) = setup(Mode::Empty, 8);
    let d = fire(&mut f, w, 1);
    let c = f.work_context::<Context>(w).unwrap();
    assert_eq!(c.calls, 1);
    assert_eq!(c.origin, Some(d.event));
    assert_eq!(c.ordinal, None);
    match &d.callback_batches[0] {
        FlowBatchReceipt::Accepted(v) => assert!(v.is_empty()),
        _ => panic!("empty delivered batch"),
    }
    assert_eq!(
        f.schedule_domain(w, kind(DOMAIN + 1), t(2), 0).unwrap_err(),
        FlowError::UnregisteredDomainEvent
    );
    for code in 4000..=4003 {
        let (mut f, w, _) = setup(Mode::Reserved(code), 8);
        assert_eq!(
            f.register_domain_hook::<Context>("unused", kind(code), callback)
                .unwrap_err(),
            FlowError::ReservedEventKind
        );
        assert_eq!(
            f.schedule_domain(w, kind(code), t(1), 0).unwrap_err(),
            FlowError::ReservedEventKind
        );
        let d = fire(&mut f, w, 1);
        let ticket = f.work_context::<Context>(w).unwrap().tickets[0];
        rejected(&d, FlowError::ReservedEventKind, Some(ticket));
        assert_eq!(f.work_context::<Context>(w).unwrap().calls, 1);
    }
}
#[test]
fn rejected_batch_has_no_partial_association_but_keeps_context_effect() {
    let (mut f, w, target) = setup(Mode::Duplicate, 8);
    let before = f.budget_snapshot().scheduler;
    let d = fire(&mut f, w, 1);
    let c = f.work_context::<Context>(w).unwrap();
    let bad = c.tickets[1];
    assert_eq!(c.calls, 1);
    assert_eq!(c.markers, vec![2]);
    rejected(&d, FlowError::InvalidWork, Some(bad));
    assert_eq!(f.work(target).unwrap().request, None);
    assert_eq!(
        f.budget_snapshot().scheduler.scheduled_events,
        before.scheduled_events + 1
    );
    let spec = f.work(target).unwrap();
    let q = f.submit_work(c.resource, spec.owner, target, t(2)).unwrap();
    assert_eq!(f.work(target).unwrap().request, Some(q));
    assert!(d.records.is_empty());
    let (mut control, cw, ct) = setup(Mode::Empty, 8);
    fire(&mut control, cw, 1);
    let cc = control.work_context::<Context>(cw).unwrap();
    let cr = cc.resource;
    let co = cc.owner;
    let cq = control.submit_work(cr, co, ct, t(2)).unwrap();
    assert_eq!(q, cq);
    let actual_event = next(&mut f).event;
    let control_event = next(&mut control).event;
    assert_eq!(actual_event, control_event);
}
#[test]
fn local_ticket_resolves_pending_cancel_and_foreign_or_non_acquire_reject() {
    let (mut f, w, target) = setup(Mode::Tickets, 8);
    let d = fire(&mut f, w, 1);
    let (q, cancel_event, submit_event) = match &d.callback_batches[0] {
        FlowBatchReceipt::Accepted(v) => {
            assert_eq!(v.len(), 2);
            (v[0].request.unwrap(), v[1].event, v[0].event)
        }
        _ => panic!("valid local ticket"),
    };
    assert_eq!(f.request(q).unwrap().state, RequestState::Pending);
    assert_eq!(f.work(target).unwrap().request, Some(q));
    let cancel = next(&mut f);
    assert_eq!(cancel.event, cancel_event);
    assert_eq!(rows(&cancel), vec![L::Cancelled]);
    let stale = f.step().unwrap().unwrap();
    assert_eq!(stale.event, submit_event);
    assert!(stale.records.is_empty());
    assert_eq!(stale.error, Some(FlowError::TerminalRequest));
    let (mut f, w, _) = setup(Mode::NonAcquire, 8);
    let d = fire(&mut f, w, 1);
    let bad = f.work_context::<Context>(w).unwrap().tickets[1];
    rejected(&d, FlowError::InvalidCommandTicket, Some(bad));
    let mut f = flow(100_000, 8);
    let owner = f.spawn_actor().unwrap();
    let r = f.create_resource(1).unwrap();
    let target = f
        .create_work(owner, SimDuration::from_ticks(3), "target", ())
        .unwrap();
    let q = f.submit(r, owner, t(10)).unwrap();
    f.register_domain_hook::<Context>("domain", kind(DOMAIN), callback)
        .unwrap();
    let mut c = context(Mode::Remember, owner, r, target);
    c.request = Some(q);
    let w = f
        .create_work(owner, SimDuration::from_ticks(1), "domain", c)
        .unwrap();
    let first = fire(&mut f, w, 1);
    match &first.callback_batches[0] {
        FlowBatchReceipt::Accepted(v) => {
            assert_eq!(v.len(), 1);
            let admitted = v[0].request.unwrap();
            assert_eq!(f.work(target).unwrap().request, Some(admitted));
            assert_eq!(f.request(admitted).unwrap().state, RequestState::Pending);
        }
        _ => panic!("first batch accepted"),
    }
    let second = fire(&mut f, w, 1);
    let c = f.work_context::<Context>(w).unwrap();
    assert_eq!(c.calls, 2);
    rejected(&second, FlowError::InvalidCommandTicket, Some(c.tickets[1]));
}
#[test]
fn legacy_first_coexistence_type_checks_and_exact_delivery_budget() {
    for first in 0..3 {
        let mut f = FlowRuntime::new();
        match first {
            0 => f
                .register_work_handlers::<Context>("matrix", WorkHandlers::default())
                .unwrap(),
            1 => f
                .register_work_continuations::<Context>("matrix", FlowContinuations::default())
                .unwrap(),
            _ => f
                .register_domain_hook::<Context>("matrix", kind(DOMAIN), callback)
                .unwrap(),
        }
        assert_eq!(
            f.register_work_handlers::<()>("matrix", WorkHandlers::default())
                .unwrap_err(),
            FlowError::InvalidWork
        );
        assert_eq!(
            f.register_work_continuations::<()>("matrix", FlowContinuations::default())
                .unwrap_err(),
            FlowError::InvalidWork
        );
        assert_eq!(
            f.register_domain_hook::<()>("matrix", kind(DOMAIN + 1), |_, _, _| {})
                .unwrap_err(),
            FlowError::InvalidWork
        );
        if first != 0 {
            f.register_work_handlers::<Context>("matrix", WorkHandlers::default())
                .unwrap();
        }
        if first != 1 {
            f.register_work_continuations::<Context>("matrix", FlowContinuations::default())
                .unwrap();
        }
        if first != 2 {
            f.register_domain_hook::<Context>("matrix", kind(DOMAIN), callback)
                .unwrap();
        }
        assert_eq!(
            f.register_work_handlers::<Context>("matrix", WorkHandlers::default())
                .unwrap_err(),
            FlowError::InvalidWork
        );
        assert_eq!(
            f.register_work_continuations::<Context>("matrix", FlowContinuations::default())
                .unwrap_err(),
            FlowError::InvalidWork
        );
    }
    for limit in [2, 3] {
        let mut f = flow(limit, 8);
        let owner = f.spawn_actor().unwrap();
        let r = f.create_resource(1).unwrap();
        let target = f
            .create_work(owner, SimDuration::from_ticks(1), "target", ())
            .unwrap();
        f.register_work_handlers(
            "both",
            WorkHandlers {
                on_cancel: Some(legacy),
                ..WorkHandlers::default()
            },
        )
        .unwrap();
        f.register_work_continuations(
            "both",
            FlowContinuations {
                on_cancel: Some(callback),
                ..FlowContinuations::default()
            },
        )
        .unwrap();
        f.register_domain_hook::<Context>("both", kind(DOMAIN), callback)
            .unwrap();
        assert_eq!(
            f.register_work_continuations::<()>("both", FlowContinuations::default())
                .unwrap_err(),
            FlowError::InvalidWork
        );
        assert_eq!(
            f.register_domain_hook::<()>("both", kind(DOMAIN + 1), |_, _, _| {})
                .unwrap_err(),
            FlowError::InvalidWork
        );
        assert_eq!(
            f.register_domain_hook::<Context>("both", kind(DOMAIN), callback)
                .unwrap_err(),
            FlowError::InvalidWork
        );
        f.register_domain_hook::<Context>("both", kind(DOMAIN + 1), callback)
            .unwrap();
        let w = f
            .create_work(
                owner,
                SimDuration::from_ticks(10),
                "both",
                context(Mode::Empty, owner, r, target),
            )
            .unwrap();
        let q = f
            .acquire(r)
            .owner(owner)
            .at(t(0))
            .timed_work(w)
            .submit()
            .unwrap();
        next(&mut f);
        f.cancel(q, t(1)).unwrap();
        let cancel = next(&mut f);
        assert_eq!(rows(&cancel), vec![L::Cancelled]);
        let old = next(&mut f);
        assert!(old.records.is_empty());
        assert!(old.callback_batches.is_empty());
        assert_eq!(f.work_context::<Context>(w).unwrap().markers, vec![1]);
        assert_eq!(f.budget_snapshot().consumed, 2);
        if limit == 2 {
            let before = f.budget_snapshot();
            assert_eq!(
                f.step().unwrap_err(),
                FlowError::SameTickBudgetExceeded {
                    at_ticks: 1,
                    limit: 2
                }
            );
            assert_eq!(f.work_context::<Context>(w).unwrap().markers, vec![1]);
            assert_eq!(f.budget_snapshot().scheduler, before.scheduler);
        } else {
            let new = next(&mut f);
            assert!(new.records.is_empty());
            let c = f.work_context::<Context>(w).unwrap();
            assert_eq!(c.markers, vec![1, 2]);
            assert_eq!(c.origin, Some(cancel.event));
            assert_eq!(c.ordinal, Some(0));
            assert_eq!(c.captured_busy, 1);
            assert_eq!(f.budget_snapshot().consumed, 3);
        }
    }
}
#[test]
fn ignored_poison_is_atomic_for_every_kind_and_default_cap_is_1024() {
    for (cap, count, command_kind) in [
        (2, 2, 0),
        (2, 2, 1),
        (2, 2, 2),
        (2, 2, 3),
        (2, 2, 4),
        (2, 4, 0),
        (2, 4, 1),
        (2, 4, 2),
        (2, 4, 3),
        (2, 4, 4),
        (1024, 1024, 0),
        (1024, 1025, 0),
    ] {
        let mut f = if cap == 1024 {
            FlowRuntime::new()
        } else {
            flow(100_000, cap)
        };
        let owner = f.spawn_actor().unwrap();
        let r = f.create_resource(2).unwrap();
        let target = f
            .create_work(owner, SimDuration::from_ticks(3), "target", ())
            .unwrap();
        let target2 = f
            .create_work(owner, SimDuration::from_ticks(3), "target2", ())
            .unwrap();
        let active = f.submit(r, owner, t(0)).unwrap();
        let active2 = f.submit(r, owner, t(0)).unwrap();
        next(&mut f);
        next(&mut f);
        let lease = f.request(active).unwrap().lease.unwrap();
        let lease2 = f.request(active2).unwrap().lease.unwrap();
        let q = f.submit(r, owner, t(10)).unwrap();
        f.register_domain_hook::<Context>("domain", kind(DOMAIN), callback)
            .unwrap();
        let mut c = context(Mode::Cap(count, command_kind), owner, r, target);
        c.request = Some(q);
        c.lease = Some(lease);
        c.second_lease = Some(lease2);
        c.second_target = Some(target2);
        let w = f
            .create_work(owner, SimDuration::from_ticks(1), "domain", c)
            .unwrap();
        let before = f.budget_snapshot().scheduler;
        let d = fire(&mut f, w, 1);
        let c = f.work_context::<Context>(w).unwrap();
        assert_eq!(c.calls, 1);
        assert_eq!(c.tickets.len(), count.min(cap));
        if count > cap {
            assert_eq!(
                c.errors,
                vec![FlowError::CallbackBatchLimitExceeded; count - cap]
            );
            rejected(&d, FlowError::CallbackBatchLimitExceeded, None);
            assert_eq!(
                f.budget_snapshot().scheduler.scheduled_events,
                before.scheduled_events + 1
            );
            assert_eq!(f.work(target).unwrap().request, None);
        } else {
            assert!(c.errors.is_empty());
            match &d.callback_batches[0] {
                FlowBatchReceipt::Accepted(v) => assert_eq!(v.len(), cap),
                _ => panic!("exact cap"),
            };
            assert_eq!(f.request(q).unwrap().state, RequestState::Pending);
        }
    }
}
#[test]
fn callback_emitted_zero_duration_is_charged_on_its_later_dispatch() {
    let mut f = flow(2, 8);
    let owner = f.spawn_actor().unwrap();
    let r = f.create_resource(1).unwrap();
    let target = f
        .create_work(owner, SimDuration::from_ticks(0), "target", ())
        .unwrap();
    f.register_domain_hook::<Context>("domain", kind(DOMAIN), callback)
        .unwrap();
    let w = f
        .create_work(
            owner,
            SimDuration::from_ticks(1),
            "domain",
            context(Mode::Complete, owner, r, target),
        )
        .unwrap();
    let d = fire(&mut f, w, 1);
    assert!(d.records.is_empty());
    assert_eq!(f.budget_snapshot().consumed, 1);
    assert_eq!(f.work_context::<Context>(w).unwrap().calls, 1);
    let pending = f.work(target).unwrap().request.unwrap();
    let before = f.budget_snapshot().scheduler;
    assert_eq!(
        f.step().unwrap_err(),
        FlowError::SameTickBudgetExceeded {
            at_ticks: 1,
            limit: 2
        }
    );
    assert_eq!(f.request(pending).unwrap().state, RequestState::Pending);
    assert_eq!(f.budget_snapshot().scheduler, before);
    assert_eq!(f.work_context::<Context>(w).unwrap().calls, 1);
}
