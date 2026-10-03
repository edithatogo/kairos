use kairo_ecs_des::{
    FlowBatchReceipt, FlowCallbackSnapshot, FlowCommandSink, FlowCommandTicket, FlowError,
    FlowOwnedCommand, FlowRuntime, FlowWorldView, RequestState,
};
use kairo_ecs_types::{EntityId, EventKind, SimDuration, SimTime};
use std::{cell::Cell, rc::Rc};
const KIND: EventKind = EventKind::Custom(7300);
fn t(n: u128) -> SimTime {
    SimTime::from_ticks(n)
}
fn probe<'a>(
    c: &'a mut Rc<Cell<u32>>,
    s: &'a FlowCallbackSnapshot,
    v: FlowWorldView<'a>,
    _: &'a mut FlowCommandSink,
) {
    assert_eq!(v.now(), s.delivery.at);
    c.set(c.get() + 1);
}
#[test]
fn explicitly_registered_carrier_is_unique_per_actor_and_kind_bound() {
    let mut f = FlowRuntime::new();
    f.register_domain_view_hook("agent", KIND, probe).unwrap();
    f.register_domain_view_hook("agent", EventKind::Custom(7301), probe)
        .unwrap();
    let a = f.spawn_actor().unwrap();
    let calls = Rc::new(Cell::new(0u32));
    let w = f
        .create_actor_domain_context(a, "agent", KIND, calls.clone())
        .unwrap();
    assert_eq!(f.actor_domain_context(a).unwrap(), w);
    let before = f.budget_snapshot();
    assert_eq!(
        f.create_actor_domain_context(a, "agent", EventKind::Custom(7301), calls.clone()),
        Err(FlowError::DuplicateActorDomainContext)
    );
    assert_eq!(f.budget_snapshot(), before);
    assert_eq!(
        f.schedule_domain(w, EventKind::Custom(7301), t(1), 0),
        Err(FlowError::InvalidWork)
    );
    assert_eq!(f.budget_snapshot(), before);
    f.schedule_domain(w, KIND, t(1), 0).unwrap();
    let d = f.step().unwrap().unwrap();
    assert!(d.error.is_none());
    assert_eq!(calls.get(), 1);
}
#[test]
fn carrier_role_rejects_manual_and_timed_acquire_without_association() {
    let mut f = FlowRuntime::new();
    f.register_domain_view_hook("agent", KIND, probe).unwrap();
    let a = f.spawn_actor().unwrap();
    let r = f.create_resource(1).unwrap();
    let w = f
        .create_actor_domain_context(a, "agent", KIND, Rc::new(Cell::new(0u32)))
        .unwrap();
    let before = f.budget_snapshot();
    assert_eq!(
        f.acquire(r).owner(a).for_work(w).submit(),
        Err(FlowError::InvalidWork)
    );
    assert_eq!(
        f.acquire(r).owner(a).timed_work(w).submit(),
        Err(FlowError::InvalidWork)
    );
    assert_eq!(f.work(w).unwrap().request, None);
    assert_eq!(f.budget_snapshot(), before);
    let ordinary = f.create_work(a, SimDuration::ZERO, "ordinary", ()).unwrap();
    let q = f.acquire(r).owner(a).timed_work(ordinary).submit().unwrap();
    let d = f.step().unwrap().unwrap();
    assert!(d.error.is_none());
    assert_eq!(f.request(q).unwrap().state, RequestState::Completed);
}
struct Owned {
    actor: EntityId,
    calls: Rc<Cell<u32>>,
}
fn despawn<'a>(
    c: &'a mut Owned,
    s: &'a FlowCallbackSnapshot,
    v: FlowWorldView<'a>,
    sink: &'a mut FlowCommandSink,
) {
    assert_eq!(v.now(), s.delivery.at);
    assert!(v.is_alive(c.actor));
    c.calls.set(c.calls.get() + 1);
    sink.emit(FlowOwnedCommand::DespawnActor {
        actor: c.actor,
        at: v.now(),
        scheduler_priority: 0,
    })
    .unwrap();
}
#[test]
fn callback_actor_despawn_is_buffered_and_cleans_task_carrier_and_stale_update() {
    let mut f = FlowRuntime::new();
    f.register_domain_view_hook("despawn", KIND, despawn)
        .unwrap();
    let a = f.spawn_actor().unwrap();
    let calls = Rc::new(Cell::new(0u32));
    let carrier = f
        .create_actor_domain_context(
            a,
            "despawn",
            KIND,
            Owned {
                actor: a,
                calls: calls.clone(),
            },
        )
        .unwrap();
    let r = f.create_resource(1).unwrap();
    let task = f
        .create_work(a, SimDuration::from_ticks(3), "task", ())
        .unwrap();
    let q = f.acquire(r).owner(a).timed_work(task).submit().unwrap();
    f.schedule_domain(carrier, KIND, t(1), 0).unwrap();
    f.schedule_domain(carrier, KIND, t(2), 0).unwrap();
    f.step().unwrap();
    let d = f.step().unwrap().unwrap();
    assert_eq!(calls.get(), 1);
    assert!(matches!(&d.callback_batches[..],[FlowBatchReceipt::Accepted(rows)] if rows.len()==1));
    assert_eq!(f.request(q).unwrap().state, RequestState::Active);
    assert!(f.work(carrier).is_ok());
    let d = f.step().unwrap().unwrap();
    assert!(d.error.is_none());
    assert_eq!(f.request(q).unwrap().state, RequestState::Cancelled);
    assert!(matches!(f.work(carrier), Err(FlowError::InvalidWork)));
    assert!(matches!(f.work(task), Err(FlowError::InvalidWork)));
    let stale = f.step().unwrap().unwrap();
    assert_eq!(stale.at, t(2));
    assert!(stale.records.is_empty() && stale.callback_batches.is_empty());
    assert!(stale.error.is_none());
    assert_eq!(calls.get(), 1);
    let completion = f.step().unwrap().unwrap();
    assert_eq!(completion.at, t(3));
    assert!(completion.records.is_empty() && completion.error.is_none());
    assert!(f.step().unwrap().is_none());
}
#[test]
fn duplicate_pending_despawn_is_explicit_while_actor_still_live() {
    let mut f = FlowRuntime::new();
    let a = f.spawn_actor().unwrap();
    f.despawn_actor_at_with_scheduler_priority(a, t(1), 0)
        .unwrap();
    let before = f.budget_snapshot();
    assert_eq!(
        f.despawn_actor_at_with_scheduler_priority(a, t(1), 0),
        Err(FlowError::DuplicateActorDespawn)
    );
    assert_eq!(f.budget_snapshot(), before);
    let d = f.step().unwrap().unwrap();
    assert!(d.error.is_none());
    assert_eq!(f.despawn_actor(a), Err(FlowError::InvalidEntity));
}

struct Duplicate {
    actor: EntityId,
    calls: Rc<Cell<u32>>,
    tickets: Rc<Cell<Option<(FlowCommandTicket, FlowCommandTicket)>>>,
}
fn duplicate<'a>(
    c: &'a mut Duplicate,
    _: &'a FlowCallbackSnapshot,
    v: FlowWorldView<'a>,
    sink: &'a mut FlowCommandSink,
) {
    assert!(v.is_alive(c.actor));
    c.calls.set(c.calls.get() + 1);
    let first = sink
        .emit(FlowOwnedCommand::DespawnActor {
            actor: c.actor,
            at: v.now(),
            scheduler_priority: 0,
        })
        .unwrap();
    let second = sink
        .emit(FlowOwnedCommand::DespawnActor {
            actor: c.actor,
            at: v.now(),
            scheduler_priority: 0,
        })
        .unwrap();
    c.tickets.set(Some((first, second)));
}
#[test]
fn same_batch_duplicate_despawn_rejects_atomically_at_second_ticket_but_context_changes_once() {
    let mut f = FlowRuntime::new();
    f.register_domain_view_hook("duplicate", KIND, duplicate)
        .unwrap();
    let actor = f.spawn_actor().unwrap();
    let calls = Rc::new(Cell::new(0u32));
    let tickets = Rc::new(Cell::new(None));
    let carrier = f
        .create_actor_domain_context(
            actor,
            "duplicate",
            KIND,
            Duplicate {
                actor,
                calls: calls.clone(),
                tickets: tickets.clone(),
            },
        )
        .unwrap();
    f.schedule_domain(carrier, KIND, t(1), 0).unwrap();
    let d = f.step().unwrap().unwrap();
    assert!(d.error.is_none());
    let (_, second) = tickets.get().unwrap();
    assert!(
        matches!(&d.callback_batches[..], [FlowBatchReceipt::Rejected(r)] if r.error==FlowError::DuplicateActorDespawn && r.failed_ticket==Some(second))
    );
    assert_eq!(calls.get(), 1);
    assert_eq!(f.actor_domain_context(actor).unwrap(), carrier);
    assert!(f.work(carrier).is_ok());
    assert!(f.step().unwrap().is_none());
    // A fresh ingress succeeds: rejected batch left no pending-despawn reservation.
    f.despawn_actor(actor).unwrap();
    let d = f.step().unwrap().unwrap();
    assert!(d.error.is_none());
    assert!(matches!(
        f.actor_domain_context(actor),
        Err(FlowError::InvalidEntity)
    ));
}
fn noop_unit<'a>(
    _: &'a mut (),
    _: &'a FlowCallbackSnapshot,
    _: FlowWorldView<'a>,
    _: &'a mut FlowCommandSink,
) {
}
#[test]
fn carrier_creation_registration_type_and_actor_failures_leave_no_association() {
    let mut f = FlowRuntime::new();
    f.register_domain_view_hook("typed", KIND, probe).unwrap();
    f.register_domain_view_hook("unit", KIND, noop_unit)
        .unwrap();
    let actor = f.spawn_actor().unwrap();
    let before = f.budget_snapshot();
    assert!(f
        .create_actor_domain_context(actor, "missing", KIND, ())
        .is_err());
    assert!(f
        .create_actor_domain_context(actor, "typed", KIND, ())
        .is_err());
    assert!(f
        .create_actor_domain_context(actor, "unit", EventKind::custom(7301), ())
        .is_err());
    assert!(matches!(
        f.actor_domain_context(actor),
        Err(FlowError::InvalidWork)
    ));
    assert_eq!(f.budget_snapshot(), before);
    let carrier = f
        .create_actor_domain_context(actor, "unit", KIND, ())
        .unwrap();
    assert_eq!(f.actor_domain_context(actor).unwrap(), carrier);
    // Different registration also rejects a second actor-derived carrier.
    assert_eq!(
        f.create_actor_domain_context(actor, "typed", KIND, Rc::new(Cell::new(0u32))),
        Err(FlowError::DuplicateActorDomainContext)
    );
}
#[test]
fn same_tick_update_and_despawn_priority_controls_live_delivery_without_competing_time() {
    for update_first in [false, true] {
        let mut f = FlowRuntime::new();
        f.register_domain_view_hook("agent", KIND, probe).unwrap();
        let actor = f.spawn_actor().unwrap();
        let calls = Rc::new(Cell::new(0u32));
        let carrier = f
            .create_actor_domain_context(actor, "agent", KIND, calls.clone())
            .unwrap();
        f.schedule_domain(carrier, KIND, t(2), if update_first { -1 } else { 1 })
            .unwrap();
        f.despawn_actor_at_with_scheduler_priority(actor, t(2), 0)
            .unwrap();
        for _ in 0..2 {
            let d = f.step().unwrap().unwrap();
            assert_eq!(d.at, t(2));
            assert!(d.error.is_none());
        }
        assert_eq!(calls.get(), u32::from(update_first));
        assert!(matches!(f.work(carrier), Err(FlowError::InvalidWork)));
        assert!(f.step().unwrap().is_none());
    }
}
