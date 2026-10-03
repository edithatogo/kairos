use kairo_ecs_des::{
    FlowAcquireCommand, FlowBatchReceipt, FlowCallbackSnapshot, FlowCommandSink, FlowCommandTicket,
    FlowError, FlowOwnedCommand, FlowRuntime, FlowWorldView, RequestState, ResourceId, WorkState,
};
use kairo_ecs_types::{EntityId, EventKind, SimDuration, SimTime};
use std::{cell::Cell, rc::Rc};
const KIND: EventKind = EventKind::custom(7500);
const OTHER: EventKind = EventKind::custom(7501);
fn t(n: u128) -> SimTime {
    SimTime::from_ticks(n)
}
fn count<'a>(
    c: &'a mut Rc<Cell<u32>>,
    s: &'a FlowCallbackSnapshot,
    v: FlowWorldView<'a>,
    _: &'a mut FlowCommandSink,
) {
    assert_eq!(v.now(), s.delivery.at);
    c.set(c.get() + 1);
}
fn legacy(c: &mut Rc<Cell<u32>>, _: &FlowCallbackSnapshot, _: &mut FlowCommandSink) {
    c.set(c.get() + 1);
}
#[test]
fn carrier_is_pending_zero_duration_and_unique_across_registered_keys_and_kinds() {
    let mut f = FlowRuntime::new();
    f.register_domain_view_hook("a", KIND, count).unwrap();
    f.register_domain_view_hook("a", OTHER, count).unwrap();
    f.register_domain_view_hook("b", KIND, count).unwrap();
    let actor = f.spawn_actor().unwrap();
    let calls = Rc::new(Cell::new(0u32));
    let carrier = f
        .create_actor_domain_context(actor, "a", KIND, calls.clone())
        .unwrap();
    assert_eq!(f.actor_domain_context(actor).unwrap(), carrier);
    let spec = f.work(carrier).unwrap();
    assert_eq!(spec.owner, actor);
    assert_eq!(spec.original_duration, SimDuration::ZERO);
    assert_eq!(spec.request, None);
    assert_eq!(f.work_progress(carrier).unwrap().state, WorkState::Pending);
    let before = f.budget_snapshot();
    assert_eq!(
        f.create_actor_domain_context(actor, "a", OTHER, calls.clone()),
        Err(FlowError::DuplicateActorDomainContext)
    );
    assert_eq!(
        f.create_actor_domain_context(actor, "b", KIND, calls.clone()),
        Err(FlowError::DuplicateActorDomainContext)
    );
    assert_eq!(f.budget_snapshot(), before);
    assert_eq!(calls.get(), 0);
    let second = f.spawn_actor().unwrap();
    let separate = f
        .create_actor_domain_context(second, "a", KIND, Rc::new(Cell::new(0u32)))
        .unwrap();
    assert_ne!(carrier, separate);
    assert_eq!(f.actor_domain_context(second).unwrap(), separate);
}
#[test]
fn carrier_requires_exact_view_registration_and_type_without_partial_association() {
    let mut f = FlowRuntime::new();
    f.register_domain_view_hook("view", KIND, count).unwrap();
    f.register_domain_hook("legacy", KIND, legacy).unwrap();
    let actor = f.spawn_actor().unwrap();
    let before = f.budget_snapshot();
    for code in 4000..=4003 {
        assert!(matches!(
            f.create_actor_domain_context(
                actor,
                "view",
                EventKind::custom(code),
                Rc::new(Cell::new(0u32))
            ),
            Err(FlowError::ReservedEventKind)
        ));
    }
    assert!(matches!(
        f.create_actor_domain_context(actor, "missing", KIND, ()),
        Err(FlowError::UnregisteredDomainEvent)
    ));
    assert!(matches!(
        f.create_actor_domain_context(actor, "view", OTHER, Rc::new(Cell::new(0u32))),
        Err(FlowError::UnregisteredDomainEvent)
    ));
    assert!(matches!(
        f.create_actor_domain_context(actor, "view", KIND, ()),
        Err(FlowError::InvalidWork)
    ));
    assert!(matches!(
        f.create_actor_domain_context(actor, "legacy", KIND, Rc::new(Cell::new(0u32))),
        Err(FlowError::InvalidWork)
    ));
    assert!(matches!(
        f.create_actor_domain_context(actor, " ", KIND, ()),
        Err(FlowError::InvalidWork)
    ));
    assert_eq!(f.actor_domain_context(actor), Err(FlowError::InvalidWork));
    assert_eq!(f.budget_snapshot(), before);
    assert!(f.step().unwrap().is_none());
    let carrier = f
        .create_actor_domain_context(actor, "view", KIND, Rc::new(Cell::new(0u32)))
        .unwrap();
    assert_eq!(f.actor_domain_context(actor).unwrap(), carrier);
}
#[test]
fn carrier_manual_and_timed_acquire_reject_but_normal_zero_duration_task_completes() {
    let mut f = FlowRuntime::new();
    f.register_domain_view_hook("view", KIND, count).unwrap();
    let actor = f.spawn_actor().unwrap();
    let resource = f.create_resource(1).unwrap();
    let carrier = f
        .create_actor_domain_context(actor, "view", KIND, Rc::new(Cell::new(0u32)))
        .unwrap();
    let before = f.budget_snapshot();
    assert_eq!(
        f.acquire(resource).owner(actor).for_work(carrier).submit(),
        Err(FlowError::InvalidWork)
    );
    assert_eq!(
        f.acquire(resource)
            .owner(actor)
            .timed_work(carrier)
            .submit(),
        Err(FlowError::InvalidWork)
    );
    assert_eq!(f.work(carrier).unwrap().request, None);
    assert_eq!(f.resource(resource).unwrap().available, 1);
    assert_eq!(f.budget_snapshot(), before);
    assert!(f.step().unwrap().is_none());
    let task = f.create_work(actor, SimDuration::ZERO, "task", ()).unwrap();
    let request = f
        .acquire(resource)
        .owner(actor)
        .timed_work(task)
        .submit()
        .unwrap();
    let d = f.step().unwrap().unwrap();
    assert!(d.error.is_none());
    assert_eq!(f.request(request).unwrap().state, RequestState::Completed);
}
#[derive(Clone, Copy)]
enum Mode {
    Acquire(bool),
    WrongKind,
}
struct Batch {
    actor: EntityId,
    resource: ResourceId,
    mode: Mode,
    calls: Rc<Cell<u32>>,
    second: Rc<Cell<Option<FlowCommandTicket>>>,
}
fn emit<'a>(
    c: &'a mut Batch,
    s: &'a FlowCallbackSnapshot,
    v: FlowWorldView<'a>,
    sink: &'a mut FlowCommandSink,
) {
    assert!(v.is_alive(c.actor));
    c.calls.set(c.calls.get() + 1);
    sink.emit(FlowOwnedCommand::Domain {
        work: s.work,
        kind: KIND,
        at: t(4),
        scheduler_priority: 0,
    })
    .unwrap();
    let command = match c.mode {
        Mode::Acquire(timed) => FlowOwnedCommand::Acquire(FlowAcquireCommand {
            resource: c.resource,
            owner: c.actor,
            work: Some(s.work),
            at: v.now(),
            priority_level: 0,
            deadline: None,
            scheduler_priority: 0,
            timed,
            can_preempt: false,
            preemptible: None,
        }),
        Mode::WrongKind => FlowOwnedCommand::Domain {
            work: s.work,
            kind: OTHER,
            at: t(4),
            scheduler_priority: 0,
        },
    };
    c.second.set(Some(sink.emit(command).unwrap()));
}
#[test]
fn batch_manual_and_timed_carrier_acquire_reject_whole_batch_at_issued_ticket() {
    for mode in [Mode::Acquire(false), Mode::Acquire(true)] {
        rejected_batch(mode);
    }
}
#[test]
fn direct_and_batch_carrier_domain_kind_is_bound_and_atomic() {
    rejected_batch(Mode::WrongKind);
    let mut f = FlowRuntime::new();
    f.register_domain_view_hook("view", KIND, count).unwrap();
    f.register_domain_view_hook("view", OTHER, count).unwrap();
    let actor = f.spawn_actor().unwrap();
    let calls = Rc::new(Cell::new(0u32));
    let carrier = f
        .create_actor_domain_context(actor, "view", KIND, calls.clone())
        .unwrap();
    let before = f.budget_snapshot();
    assert_eq!(
        f.schedule_domain(carrier, OTHER, t(1), 0),
        Err(FlowError::InvalidWork)
    );
    assert_eq!(f.budget_snapshot(), before);
    assert!(f.step().unwrap().is_none());
    f.schedule_domain(carrier, KIND, t(1), 0).unwrap();
    assert!(f.step().unwrap().unwrap().error.is_none());
    assert_eq!(calls.get(), 1);
}
fn rejected_batch(mode: Mode) {
    let mut f = FlowRuntime::new();
    f.register_domain_view_hook("batch", KIND, emit).unwrap();
    f.register_domain_view_hook("batch", OTHER, emit).unwrap();
    let actor = f.spawn_actor().unwrap();
    let resource = f.create_resource(1).unwrap();
    let calls = Rc::new(Cell::new(0u32));
    let second = Rc::new(Cell::new(None));
    let carrier = f
        .create_actor_domain_context(
            actor,
            "batch",
            KIND,
            Batch {
                actor,
                resource,
                mode,
                calls: calls.clone(),
                second: second.clone(),
            },
        )
        .unwrap();
    f.schedule_domain(carrier, KIND, t(1), 0).unwrap();
    let d = f.step().unwrap().unwrap();
    assert!(d.error.is_none());
    assert!(
        matches!(&d.callback_batches[..],[FlowBatchReceipt::Rejected(r)] if r.error==FlowError::InvalidWork && r.failed_ticket==second.get())
    );
    assert_eq!(calls.get(), 1);
    assert_eq!(f.work(carrier).unwrap().request, None);
    assert_eq!(f.resource(resource).unwrap().available, 1);
    assert!(f.step().unwrap().is_none());
}
#[test]
fn ordinary_actor_cleanup_removes_carrier_index_and_old_update_is_stale() {
    let mut f = FlowRuntime::new();
    f.register_domain_view_hook("view", KIND, count).unwrap();
    let actor = f.spawn_actor().unwrap();
    let calls = Rc::new(Cell::new(0u32));
    let carrier = f
        .create_actor_domain_context(actor, "view", KIND, calls.clone())
        .unwrap();
    f.schedule_domain(carrier, KIND, t(3), 0).unwrap();
    f.despawn_actor(actor).unwrap();
    assert!(f.step().unwrap().unwrap().error.is_none());
    assert_eq!(f.actor_domain_context(actor), Err(FlowError::InvalidEntity));
    assert!(matches!(f.work(carrier), Err(FlowError::InvalidWork)));
    assert_eq!(
        f.schedule_domain(carrier, KIND, t(4), 0),
        Err(FlowError::InvalidWork)
    );
    let recycled = f.spawn_actor().unwrap();
    assert_ne!(recycled, actor);
    assert_eq!(recycled.index, actor.index);
    assert_eq!(recycled.generation, actor.generation + 1);
    assert_eq!(
        f.actor_domain_context(recycled),
        Err(FlowError::InvalidWork)
    );
    let fresh = f
        .create_actor_domain_context(recycled, "view", KIND, calls.clone())
        .unwrap();
    f.schedule_domain(fresh, KIND, t(4), 0).unwrap();
    let stale = f.step().unwrap().unwrap();
    assert_eq!(stale.at, t(3));
    assert!(stale.records.is_empty() && stale.callback_batches.is_empty() && stale.error.is_none());
    assert_eq!(calls.get(), 0);
    assert!(f.step().unwrap().unwrap().error.is_none());
    assert_eq!(calls.get(), 1);
    assert!(f.step().unwrap().is_none());
}
