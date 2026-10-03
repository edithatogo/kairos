use kairo_ecs_des::{
    FlowBatchReceipt, FlowCallbackCause, FlowCallbackSnapshot, FlowCommandSink, FlowError,
    FlowOwnedCommand, FlowRuntime, FlowWorldView,
};
use kairo_ecs_types::{EntityId, EventKind, SimDuration, SimTime};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
const VIEW: EventKind = EventKind::custom(7400);
const LEGACY: EventKind = EventKind::custom(7401);
fn t(n: u128) -> SimTime {
    SimTime::from_ticks(n)
}
struct Probe {
    actor: EntityId,
    other: EntityId,
    rows: Rc<RefCell<Vec<(SimTime, bool, bool)>>>,
    follow: bool,
}
fn observe<'a>(
    c: &'a mut Probe,
    s: &'a FlowCallbackSnapshot,
    v: FlowWorldView<'a>,
    sink: &'a mut FlowCommandSink,
) {
    assert_eq!(v.now(), s.delivery.at);
    assert_eq!(s.origin, s.delivery.id);
    assert_eq!(s.origin_ordinal, None);
    assert_eq!(s.cause, FlowCallbackCause::Domain { kind: VIEW });
    c.rows
        .borrow_mut()
        .push((v.now(), v.is_alive(c.actor), v.is_alive(c.other)));
    if c.follow {
        c.follow = false;
        sink.emit(FlowOwnedCommand::Domain {
            work: s.work,
            kind: VIEW,
            at: t(4),
            scheduler_priority: 0,
        })
        .unwrap();
    }
}
#[test]
fn real_world_time_hrtb_and_deferred_command_delivery() {
    let mut f = FlowRuntime::new();
    let callback: for<'a> fn(
        &'a mut Probe,
        &'a FlowCallbackSnapshot,
        FlowWorldView<'a>,
        &'a mut FlowCommandSink,
    ) = observe;
    f.register_domain_view_hook("view", VIEW, callback).unwrap();
    let actor = f.spawn_actor().unwrap();
    let other = f.spawn_actor().unwrap();
    let rows = Rc::new(RefCell::new(Vec::new()));
    let work = f
        .create_work(
            actor,
            SimDuration::ZERO,
            "view",
            Probe {
                actor,
                other,
                rows: rows.clone(),
                follow: true,
            },
        )
        .unwrap();
    f.schedule_domain(work, VIEW, t(3), 0).unwrap();
    f.despawn_actor(other).unwrap();
    let d = f.step().unwrap().unwrap();
    assert_eq!(d.at, t(0));
    assert!(d.error.is_none());
    let d = f.step().unwrap().unwrap();
    assert_eq!(d.at, t(3));
    assert!(d.error.is_none());
    assert!(
        matches!(&d.callback_batches[..],[FlowBatchReceipt::Accepted(commands)] if commands.len()==1)
    );
    assert_eq!(&rows.borrow()[..], &[(t(3), true, false)]);
    let d = f.step().unwrap().unwrap();
    assert_eq!(d.at, t(4));
    assert!(d.error.is_none());
    assert_eq!(
        &rows.borrow()[..],
        &[(t(3), true, false), (t(4), true, false)]
    );
    assert!(f.step().unwrap().is_none());
}
fn view_count<'a>(
    c: &'a mut Rc<Cell<u32>>,
    _: &'a FlowCallbackSnapshot,
    _: FlowWorldView<'a>,
    _: &'a mut FlowCommandSink,
) {
    c.set(c.get() + 1);
}
fn old_count(c: &mut Rc<Cell<u32>>, _: &FlowCallbackSnapshot, _: &mut FlowCommandSink) {
    c.set(c.get() + 10);
}
fn unit_view<'a>(
    _: &'a mut (),
    _: &'a FlowCallbackSnapshot,
    _: FlowWorldView<'a>,
    _: &'a mut FlowCommandSink,
) {
}
#[test]
fn legacy_view_descriptor_duplicates_both_orders_and_compatible_kind_coexistence() {
    for view_first in [false, true] {
        let mut f = FlowRuntime::new();
        if view_first {
            f.register_domain_view_hook("key", VIEW, view_count)
                .unwrap();
            assert_eq!(
                f.register_domain_hook("key", VIEW, old_count),
                Err(FlowError::InvalidWork)
            );
        } else {
            f.register_domain_hook("key", VIEW, old_count).unwrap();
            assert_eq!(
                f.register_domain_view_hook("key", VIEW, view_count),
                Err(FlowError::InvalidWork)
            );
        }
    }
    let mut f = FlowRuntime::new();
    f.register_domain_view_hook("key", VIEW, view_count)
        .unwrap();
    f.register_domain_hook("key", LEGACY, old_count).unwrap();
    assert_eq!(
        f.register_domain_view_hook("key", EventKind::custom(7402), unit_view),
        Err(FlowError::InvalidWork)
    );
    let actor = f.spawn_actor().unwrap();
    let calls = Rc::new(Cell::new(0u32));
    let work = f
        .create_work(actor, SimDuration::ZERO, "key", calls.clone())
        .unwrap();
    f.schedule_domain(work, VIEW, t(1), 0).unwrap();
    f.schedule_domain(work, LEGACY, t(2), 0).unwrap();
    assert!(f.step().unwrap().unwrap().error.is_none());
    assert_eq!(calls.get(), 1);
    assert!(f.step().unwrap().unwrap().error.is_none());
    assert_eq!(calls.get(), 11);
    assert!(f.step().unwrap().is_none());
}
#[test]
fn reserved_kinds_and_public_wrong_context_reject_without_scheduling() {
    let mut f = FlowRuntime::new();
    let before = f.budget_snapshot();
    for code in 4000..=4003 {
        assert_eq!(
            f.register_domain_view_hook("reserved", EventKind::custom(code), unit_view),
            Err(FlowError::ReservedEventKind)
        );
    }
    assert_eq!(f.budget_snapshot(), before);
    f.register_domain_view_hook("key", VIEW, view_count)
        .unwrap();
    let actor = f.spawn_actor().unwrap();
    assert!(matches!(
        f.create_work(actor, SimDuration::ZERO, "key", ()),
        Err(FlowError::InvalidWork)
    ));
    assert_eq!(f.budget_snapshot(), before);
    assert!(f.step().unwrap().is_none());
    let work = f
        .create_work(actor, SimDuration::ZERO, "key", Rc::new(Cell::new(0u32)))
        .unwrap();
    for code in 4000..=4003 {
        assert_eq!(
            f.schedule_domain(work, EventKind::custom(code), t(1), 0),
            Err(FlowError::ReservedEventKind)
        );
    }
    assert_eq!(f.budget_snapshot(), before);
    assert!(f.step().unwrap().is_none());
}
#[test]
fn view_registration_after_same_key_work_preserves_existing_gate() {
    let mut f = FlowRuntime::new();
    f.register_domain_view_hook("key", VIEW, view_count)
        .unwrap();
    let actor = f.spawn_actor().unwrap();
    f.create_work(actor, SimDuration::ZERO, "key", Rc::new(Cell::new(0u32)))
        .unwrap();
    assert_eq!(
        f.register_domain_view_hook("key", LEGACY, view_count),
        Err(FlowError::InvalidWork)
    );
    assert_eq!(
        f.register_domain_hook("key", LEGACY, old_count),
        Err(FlowError::InvalidWork)
    );
}
#[test]
fn removed_work_view_event_is_consumed_stale_without_callback() {
    let mut f = FlowRuntime::new();
    f.register_domain_view_hook("key", VIEW, view_count)
        .unwrap();
    let actor = f.spawn_actor().unwrap();
    let calls = Rc::new(Cell::new(0u32));
    let work = f
        .create_work(actor, SimDuration::ZERO, "key", calls.clone())
        .unwrap();
    f.schedule_domain(work, VIEW, t(3), 0).unwrap();
    f.despawn_actor(actor).unwrap();
    assert!(f.step().unwrap().unwrap().error.is_none());
    let d = f.step().unwrap().unwrap();
    assert_eq!(d.at, t(3));
    assert!(d.error.is_none());
    assert!(d.records.is_empty());
    assert!(d.callback_batches.is_empty());
    assert_eq!(calls.get(), 0);
    assert!(f.step().unwrap().is_none());
}
fn poison<'a>(
    c: &'a mut Rc<Cell<u32>>,
    s: &'a FlowCallbackSnapshot,
    v: FlowWorldView<'a>,
    sink: &'a mut FlowCommandSink,
) {
    c.set(c.get() + 1);
    sink.emit(FlowOwnedCommand::Domain {
        work: s.work,
        kind: VIEW,
        at: t(4),
        scheduler_priority: 0,
    })
    .unwrap();
    sink.emit(FlowOwnedCommand::Domain {
        work: s.work,
        kind: EventKind::custom(4000),
        at: v.now(),
        scheduler_priority: 0,
    })
    .unwrap();
}
#[test]
fn real_view_callback_preserves_atomic_batch_rejection_and_nonrollback_context() {
    let mut f = FlowRuntime::new();
    f.register_domain_view_hook("key", VIEW, poison).unwrap();
    let actor = f.spawn_actor().unwrap();
    let calls = Rc::new(Cell::new(0u32));
    let work = f
        .create_work(actor, SimDuration::ZERO, "key", calls.clone())
        .unwrap();
    f.schedule_domain(work, VIEW, t(2), 0).unwrap();
    let d = f.step().unwrap().unwrap();
    assert!(d.error.is_none());
    assert!(
        matches!(&d.callback_batches[..],[FlowBatchReceipt::Rejected(r)] if r.error==FlowError::ReservedEventKind && r.failed_ticket.is_some())
    );
    assert_eq!(calls.get(), 1);
    assert!(f.step().unwrap().is_none());
}
