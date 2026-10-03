use kairo_ecs_abm::{
    create_flow_agent, register_flow_agent_behavior, schedule_flow_agent_update, FlowAgentBehavior,
    FlowAgentContext, FlowAgentHandle,
};
use kairo_ecs_des::{
    FlowAcquireCommand, FlowBatchReceipt, FlowCallbackConfig, FlowConfig, FlowError,
    FlowOwnedCommand, FlowRuntime, LifecycleTransition, RequestState,
};
use kairo_ecs_rng::DeterministicStream;
use kairo_ecs_types::{EntityId, EventKind, SimDuration, SimTime};
use std::{
    cell::{Cell, RefCell},
    num::{NonZeroU64, NonZeroUsize},
    rc::Rc,
};
const KIND: EventKind = EventKind::custom(7800);
const OTHER: EventKind = EventKind::custom(7801);
const SEED: u64 = 0x917c_a113_08d2_56e9;
fn t(n: u128) -> SimTime {
    SimTime::from_ticks(n)
}
#[derive(Debug, Eq, PartialEq)]
struct Observation {
    actor: EntityId,
    at: SimTime,
    draw: u64,
    peer_alive: Option<bool>,
}
struct State {
    log: Rc<RefCell<Vec<Observation>>>,
    calls: u32,
    peer: Option<EntityId>,
    resource: Option<kairo_ecs_des::ResourceId>,
    task: Option<kairo_ecs_des::WorkId>,
    lease: Rc<Cell<Option<kairo_ecs_des::LeaseId>>>,
    failed: Rc<Cell<Option<kairo_ecs_des::FlowCommandTicket>>>,
}
fn state(log: Rc<RefCell<Vec<Observation>>>) -> State {
    State {
        log,
        calls: 0,
        peer: None,
        resource: None,
        task: None,
        lease: Rc::new(Cell::new(None)),
        failed: Rc::new(Cell::new(None)),
    }
}
#[derive(Clone, Copy)]
enum Action {
    Observe,
    PeerDespawn,
    Duplicate,
    WrongKind,
    Cap,
    Operations,
    CarrierAcquire,
}
struct Behavior {
    action: Action,
}
impl FlowAgentBehavior<State> for Behavior {
    fn update(&mut self, c: FlowAgentContext<'_, State>) {
        assert_eq!(c.view.now(), c.event.delivery.at);
        assert!(c.view.is_alive(c.agent));
        c.state.calls += 1;
        let call = c.state.calls;
        let draw = c.rng.next_u64();
        c.state.log.borrow_mut().push(Observation {
            actor: c.agent,
            at: c.view.now(),
            draw,
            peer_alive: c.state.peer.map(|p| c.view.is_alive(p)),
        });
        let domain = |kind, at| FlowOwnedCommand::Domain {
            work: c.event.work,
            kind,
            at,
            scheduler_priority: 0,
        };
        let despawn = |actor, at| FlowOwnedCommand::DespawnActor {
            actor,
            at,
            scheduler_priority: 0,
        };
        match self.action {
            Action::Observe => {}
            Action::PeerDespawn if call == 1 => {
                c.commands
                    .emit(despawn(c.state.peer.unwrap(), t(2)))
                    .unwrap();
            }
            Action::Duplicate if call == 1 => {
                c.commands.emit(domain(KIND, t(4))).unwrap();
                c.commands.emit(despawn(c.agent, t(4))).unwrap();
                let ticket = c.commands.emit(despawn(c.agent, t(4))).unwrap();
                c.state.failed.set(Some(ticket));
            }
            Action::WrongKind if call == 1 => {
                c.commands.emit(domain(KIND, t(4))).unwrap();
                let ticket = c.commands.emit(domain(OTHER, t(4))).unwrap();
                c.state.failed.set(Some(ticket));
            }
            Action::Cap if call == 1 => {
                for _ in 0..3 {
                    let _ = c.commands.emit(domain(KIND, t(4)));
                }
            }
            Action::Operations if call == 1 => {
                c.commands
                    .emit(FlowOwnedCommand::Acquire(FlowAcquireCommand {
                        resource: c.state.resource.unwrap(),
                        owner: c.agent,
                        work: c.state.task,
                        at: t(1),
                        priority_level: 0,
                        deadline: None,
                        scheduler_priority: 0,
                        timed: true,
                        can_preempt: false,
                        preemptible: None,
                    }))
                    .unwrap();
                c.commands.emit(domain(KIND, t(2))).unwrap();
            }
            Action::Operations if call == 2 => {
                c.commands
                    .emit(FlowOwnedCommand::Release {
                        lease: c.state.lease.get().unwrap(),
                        at: t(2),
                    })
                    .unwrap();
                c.commands.emit(domain(KIND, t(3))).unwrap();
            }
            Action::Operations if call == 3 => {
                c.commands.emit(despawn(c.agent, t(3))).unwrap();
            }
            Action::CarrierAcquire if call == 1 => {
                c.commands
                    .emit(FlowOwnedCommand::Acquire(FlowAcquireCommand {
                        resource: c.state.resource.unwrap(),
                        owner: c.agent,
                        work: Some(c.event.work),
                        at: c.view.now(),
                        priority_level: 0,
                        deadline: None,
                        scheduler_priority: 0,
                        timed: true,
                        can_preempt: false,
                        preemptible: None,
                    }))
                    .unwrap();
            }
            _ => {}
        }
    }
}
fn registered(f: &mut FlowRuntime) {
    register_flow_agent_behavior::<State, Behavior>(f, "agent", KIND).unwrap();
}
fn draws(actor: EntityId, n: usize) -> Vec<u64> {
    let mut s = DeterministicStream::from_entity(SEED, actor);
    (0..n).map(|_| s.next_u64()).collect()
}
fn drain(f: &mut FlowRuntime) -> Vec<kairo_ecs_des::FlowDispatch> {
    let mut out = Vec::new();
    for _ in 0..64 {
        let Some(d) = f.step().unwrap() else {
            return out;
        };
        assert!(d.error.is_none(), "{:?}", d.error);
        out.push(d);
    }
    panic!("bounded fixture failed to drain")
}
#[test]
fn shared_behavior_observes_authoritative_des_time_and_buffered_peer_lifetime() {
    let mut f = FlowRuntime::new();
    registered(&mut f);
    let actor = f.spawn_actor().unwrap();
    let peer = f.spawn_actor().unwrap();
    let resource = f.create_resource(1).unwrap();
    let task = f
        .create_work(actor, SimDuration::from_ticks(4), "task", ())
        .unwrap();
    let request = f
        .acquire(resource)
        .owner(actor)
        .timed_work(task)
        .submit()
        .unwrap();
    let expected = draws(actor, 2);
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut s = state(log.clone());
    s.peer = Some(peer);
    let handle = create_flow_agent(
        &mut f,
        actor,
        "agent",
        KIND,
        SEED,
        s,
        Behavior {
            action: Action::PeerDespawn,
        },
    )
    .unwrap();
    schedule_flow_agent_update(&mut f, handle, t(1), 0).unwrap();
    schedule_flow_agent_update(&mut f, handle, t(3), 0).unwrap();
    let ds = drain(&mut f);
    assert_eq!(
        *log.borrow(),
        vec![
            Observation {
                actor,
                at: t(1),
                draw: expected[0],
                peer_alive: Some(true)
            },
            Observation {
                actor,
                at: t(3),
                draw: expected[1],
                peer_alive: Some(false)
            }
        ]
    );
    assert_eq!(f.now(), t(4));
    assert_eq!(f.request(request).unwrap().state, RequestState::Completed);
    assert!(ds
        .iter()
        .flat_map(|d| &d.records)
        .any(|r| r.request == request
            && r.transition == LifecycleTransition::Completed
            && r.at == t(4)));
}
#[test]
fn entity_stream_draw_positions_ignore_cross_actor_order_and_pause_boundaries() {
    for reverse in [false, true] {
        for pause in [false, true] {
            let mut f = FlowRuntime::new();
            registered(&mut f);
            let actors = [f.spawn_actor().unwrap(), f.spawn_actor().unwrap()];
            let expected = [draws(actors[0], 3), draws(actors[1], 3)];
            let log = Rc::new(RefCell::new(Vec::new()));
            let handles = actors.map(|actor| {
                create_flow_agent(
                    &mut f,
                    actor,
                    "agent",
                    KIND,
                    SEED,
                    state(log.clone()),
                    Behavior {
                        action: Action::Observe,
                    },
                )
                .unwrap()
            });
            for tick in 1..=3 {
                for i in if reverse { [1, 0] } else { [0, 1] } {
                    schedule_flow_agent_update(
                        &mut f,
                        handles[i],
                        t(tick),
                        if reverse { -(i as i32) } else { i as i32 },
                    )
                    .unwrap();
                }
            }
            if pause {
                let first = f.run_for(1).unwrap();
                assert_eq!(first.dispatches.len(), 1);
            }
            drain(&mut f);
            for i in 0..2 {
                let actual = log
                    .borrow()
                    .iter()
                    .filter(|o| o.actor == actors[i])
                    .map(|o| (o.at, o.draw))
                    .collect::<Vec<_>>();
                assert_eq!(
                    actual,
                    (1..=3)
                        .zip(&expected[i])
                        .map(|(n, d)| (t(n), *d))
                        .collect::<Vec<_>>()
                );
            }
        }
    }
}
#[test]
fn recycled_actor_generation_has_fresh_stream_and_old_handle_is_stale() {
    let mut f = FlowRuntime::new();
    registered(&mut f);
    let actor = f.spawn_actor().unwrap();
    let old_draw = draws(actor, 1)[0];
    let oldlog = Rc::new(RefCell::new(Vec::new()));
    let old = create_flow_agent(
        &mut f,
        actor,
        "agent",
        KIND,
        SEED,
        state(oldlog.clone()),
        Behavior {
            action: Action::Observe,
        },
    )
    .unwrap();
    schedule_flow_agent_update(&mut f, old, t(1), 0).unwrap();
    schedule_flow_agent_update(&mut f, old, t(3), 0).unwrap();
    f.despawn_actor_at_with_scheduler_priority(actor, t(2), 0)
        .unwrap();
    f.step().unwrap().unwrap();
    f.step().unwrap().unwrap();
    let next = f.spawn_actor().unwrap();
    assert_eq!(next.index, actor.index);
    assert!(next.generation > actor.generation);
    let before = f.budget_snapshot();
    assert_eq!(
        schedule_flow_agent_update(&mut f, old, t(3), 0),
        Err(FlowError::InvalidEntity)
    );
    assert_eq!(f.budget_snapshot(), before);
    let new_draw = draws(next, 1)[0];
    assert_ne!(new_draw, old_draw);
    let newlog = Rc::new(RefCell::new(Vec::new()));
    let new = create_flow_agent(
        &mut f,
        next,
        "agent",
        KIND,
        SEED,
        state(newlog.clone()),
        Behavior {
            action: Action::Observe,
        },
    )
    .unwrap();
    schedule_flow_agent_update(&mut f, new, t(3), 0).unwrap();
    let stale = f.step().unwrap().unwrap();
    assert_eq!(stale.at, t(3));
    assert!(stale.records.is_empty() && stale.callback_batches.is_empty() && stale.error.is_none());
    assert_eq!(f.budget_snapshot().consumed, 0);
    drain(&mut f);
    assert_eq!(oldlog.borrow().len(), 1);
    assert_eq!(oldlog.borrow()[0].draw, old_draw);
    assert_eq!(newlog.borrow().len(), 1);
    assert_eq!(newlog.borrow()[0].draw, new_draw);
}
struct OtherBehavior;
impl FlowAgentBehavior<State> for OtherBehavior {
    fn update(&mut self, _: FlowAgentContext<'_, State>) {}
}
struct UnitBehavior;
impl FlowAgentBehavior<()> for UnitBehavior {
    fn update(&mut self, _: FlowAgentContext<'_, ()>) {}
}
fn handle_traits<T: Clone + Copy + std::fmt::Debug + Eq>() {}
#[test]
fn registration_type_kind_and_unique_carrier_failures_are_atomic() {
    handle_traits::<FlowAgentHandle>();
    let mut f = FlowRuntime::new();
    registered(&mut f);
    register_flow_agent_behavior::<State, Behavior>(&mut f, "agent", OTHER).unwrap();
    let actor = f.spawn_actor().unwrap();
    let log = Rc::new(RefCell::new(Vec::new()));
    let before = f.budget_snapshot();
    for kind in 4000..=4003 {
        assert_eq!(
            register_flow_agent_behavior::<State, Behavior>(
                &mut f,
                "reserved",
                EventKind::custom(kind)
            ),
            Err(FlowError::ReservedEventKind)
        );
    }
    assert_eq!(
        register_flow_agent_behavior::<State, Behavior>(&mut f, "", KIND),
        Err(FlowError::InvalidWork)
    );
    assert_eq!(
        register_flow_agent_behavior::<State, Behavior>(&mut f, "agent", KIND),
        Err(FlowError::InvalidWork)
    );
    assert_eq!(
        create_flow_agent(&mut f, actor, "agent", KIND, SEED, (), UnitBehavior),
        Err(FlowError::InvalidWork)
    );
    assert_eq!(
        create_flow_agent(
            &mut f,
            actor,
            "agent",
            KIND,
            SEED,
            state(log.clone()),
            OtherBehavior
        ),
        Err(FlowError::InvalidWork)
    );
    assert_eq!(
        create_flow_agent(
            &mut f,
            actor,
            "missing",
            KIND,
            SEED,
            state(log.clone()),
            Behavior {
                action: Action::Observe
            }
        ),
        Err(FlowError::UnregisteredDomainEvent)
    );
    assert_eq!(f.actor_domain_context(actor), Err(FlowError::InvalidWork));
    assert_eq!(f.budget_snapshot(), before);
    let good = create_flow_agent(
        &mut f,
        actor,
        "agent",
        KIND,
        SEED,
        state(log.clone()),
        Behavior {
            action: Action::Observe,
        },
    )
    .unwrap();
    let carrier = f.actor_domain_context(actor).unwrap();
    let before = f.budget_snapshot();
    assert_eq!(
        create_flow_agent(
            &mut f,
            actor,
            "agent",
            OTHER,
            SEED,
            state(log.clone()),
            Behavior {
                action: Action::Observe
            }
        ),
        Err(FlowError::DuplicateActorDomainContext)
    );
    assert_eq!(
        register_flow_agent_behavior::<State, Behavior>(&mut f, "late", KIND),
        Err(FlowError::InvalidWork)
    );
    assert_eq!(f.budget_snapshot(), before);
    assert_eq!(f.actor_domain_context(actor).unwrap(), carrier);
    assert!(log.borrow().is_empty());
    assert_eq!(
        f.schedule_domain(carrier, OTHER, t(1), 0),
        Err(FlowError::InvalidWork)
    );
    schedule_flow_agent_update(&mut f, good, t(1), 0).unwrap();
    drain(&mut f);
    assert_eq!(log.borrow()[0].draw, draws(actor, 1)[0]);
}
#[test]
fn rejected_callback_batches_keep_one_state_change_and_one_draw_without_replay() {
    for action in [Action::Duplicate, Action::WrongKind, Action::Cap] {
        let mut f = FlowRuntime::with_configs(
            FlowConfig::default(),
            FlowCallbackConfig {
                max_callback_commands: NonZeroUsize::new(if matches!(action, Action::Duplicate) {
                    3
                } else {
                    2
                })
                .unwrap(),
            },
        );
        registered(&mut f);
        let actor = f.spawn_actor().unwrap();
        let expected = draws(actor, 2);
        let log = Rc::new(RefCell::new(Vec::new()));
        let s = state(log.clone());
        let failed = s.failed.clone();
        let handle =
            create_flow_agent(&mut f, actor, "agent", KIND, SEED, s, Behavior { action }).unwrap();
        schedule_flow_agent_update(&mut f, handle, t(1), 0).unwrap();
        let before = f.budget_snapshot().scheduler;
        let d = f.step().unwrap().unwrap();
        assert_eq!(
            f.budget_snapshot().scheduler.scheduled_events,
            before.scheduled_events
        );
        assert_eq!(
            f.budget_snapshot().scheduler.dispatched_events,
            before.dispatched_events + 1
        );
        let FlowBatchReceipt::Rejected(rejection) = &d.callback_batches[0] else {
            panic!("mustreject")
        };
        let error = match action {
            Action::Duplicate => FlowError::DuplicateActorDespawn,
            Action::WrongKind => FlowError::InvalidWork,
            _ => FlowError::CallbackBatchLimitExceeded,
        };
        assert_eq!(rejection.error, error);
        assert_eq!(log.borrow().len(), 1);
        assert_eq!(log.borrow()[0].draw, expected[0]);
        assert!(f.step().unwrap().is_none());
        assert!(f.actor_domain_context(actor).is_ok());
        if matches!(action, Action::Cap) {
            assert!(rejection.failed_ticket.is_none());
        } else {
            assert_eq!(rejection.failed_ticket, failed.get());
            assert!(failed.get().is_some());
        }
        schedule_flow_agent_update(&mut f, handle, t(2), 0).unwrap();
        drain(&mut f);
        assert_eq!(log.borrow().len(), 2);
        assert_eq!(log.borrow()[1].draw, expected[1]);
    }
}
#[test]
fn preconsume_budget_failure_preserves_behavior_draw_position_and_pending_head() {
    let mut f = FlowRuntime::with_config(FlowConfig {
        max_same_tick_flow_transitions: NonZeroU64::new(1).unwrap(),
    });
    registered(&mut f);
    let actor = f.spawn_actor().unwrap();
    let expected = draws(actor, 1)[0];
    let log = Rc::new(RefCell::new(Vec::new()));
    let handle = create_flow_agent(
        &mut f,
        actor,
        "agent",
        KIND,
        SEED,
        state(log.clone()),
        Behavior {
            action: Action::Observe,
        },
    )
    .unwrap();
    schedule_flow_agent_update(&mut f, handle, t(0), 0).unwrap();
    let pending = schedule_flow_agent_update(&mut f, handle, t(0), 0).unwrap();
    f.step().unwrap().unwrap();
    let before = f.budget_snapshot();
    assert_eq!(log.borrow().len(), 1);
    assert_eq!(log.borrow()[0].draw, expected);
    for _ in 0..3 {
        assert_eq!(
            f.step().unwrap_err(),
            FlowError::SameTickBudgetExceeded {
                at_ticks: 0,
                limit: 1
            }
        );
        assert_eq!(log.borrow().len(), 1);
        assert_eq!(f.budget_snapshot().scheduler, before.scheduler);
        assert_eq!(f.budget_snapshot().halted.unwrap().pending.id, pending);
    }
}
#[test]
fn behavior_commands_use_real_task_leases_and_never_acquire_the_carrier() {
    let mut f = FlowRuntime::new();
    registered(&mut f);
    let actor = f.spawn_actor().unwrap();
    let resource = f.create_resource(1).unwrap();
    let task = f
        .create_work(actor, SimDuration::from_ticks(4), "task", ())
        .unwrap();
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut s = state(log.clone());
    s.resource = Some(resource);
    s.task = Some(task);
    let lease = s.lease.clone();
    let expected = draws(actor, 3);
    let handle = create_flow_agent(
        &mut f,
        actor,
        "agent",
        KIND,
        SEED,
        s,
        Behavior {
            action: Action::Operations,
        },
    )
    .unwrap();
    schedule_flow_agent_update(&mut f, handle, t(1), 0).unwrap();
    let first = f.step().unwrap().unwrap();
    let FlowBatchReceipt::Accepted(admitted) = &first.callback_batches[0] else {
        panic!("valid batch")
    };
    let request = admitted[0].request.unwrap();
    assert_eq!(f.request(request).unwrap().state, RequestState::Pending);
    f.step().unwrap().unwrap();
    lease.set(f.request(request).unwrap().lease);
    assert_eq!(
        f.work(task).unwrap().original_duration,
        SimDuration::from_ticks(4)
    );
    f.step().unwrap().unwrap();
    f.step().unwrap().unwrap();
    assert_eq!(f.request(request).unwrap().state, RequestState::Released);
    f.step().unwrap().unwrap();
    f.step().unwrap().unwrap();
    drain(&mut f);
    assert_eq!(f.request(request).unwrap().state, RequestState::Released);
    assert_eq!(
        log.borrow().iter().map(|o| o.draw).collect::<Vec<_>>(),
        expected
    );
    let mut f = FlowRuntime::new();
    registered(&mut f);
    let actor = f.spawn_actor().unwrap();
    let resource = f.create_resource(1).unwrap();
    let mut s = state(Rc::new(RefCell::new(Vec::new())));
    s.resource = Some(resource);
    let handle = create_flow_agent(
        &mut f,
        actor,
        "agent",
        KIND,
        SEED,
        s,
        Behavior {
            action: Action::CarrierAcquire,
        },
    )
    .unwrap();
    let carrier = f.actor_domain_context(actor).unwrap();
    schedule_flow_agent_update(&mut f, handle, t(1), 0).unwrap();
    let d = f.step().unwrap().unwrap();
    let FlowBatchReceipt::Rejected(r) = &d.callback_batches[0] else {
        panic!("carrier acquire mustreject")
    };
    assert_eq!(r.error, FlowError::InvalidWork);
    assert!(f.work(carrier).unwrap().request.is_none());
    assert_eq!(f.resource(resource).unwrap().available, 1);
    assert!(f.step().unwrap().is_none());
}
