//! C2.0 red fixtures for the planned Flow domain/control boundary.
//!
//! These tests are over kairo-ecs-des itself. They intentionally stay outside
//! Cargo's normal test tree until Track03 implements the reviewed API.

use kairo_ecs_des::{
    FlowAcquireCommand, FlowBatchReceipt, FlowCallbackCause, FlowCallbackConfig,
    FlowCallbackSnapshot, FlowCommandSink, FlowDomainControl, FlowError, FlowOwnedCommand,
    FlowRuntime, FlowWorldView, ResourceId,
};
use kairo_ecs_types::{EntityId, EventKind, SimDuration, SimTime};
use std::num::NonZeroUsize;

const PLAN_KIND: EventKind = EventKind::custom(7_810);
const OTHER_KIND: EventKind = EventKind::custom(7_811);

fn t(ticks: u128) -> SimTime {
    SimTime::from_ticks(ticks)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Behavior {
    ErrorAfterEmit,
    PoisonSink,
    InvalidBatch,
    Accept,
    Controls,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct State {
    behavior: Behavior,
    owner: EntityId,
    task: kairo_ecs_des::WorkId,
    resource: ResourceId,
    invalid_resource: Option<ResourceId>,
    commits: u32,
    controls: Vec<(FlowDomainControl, SimTime)>,
}

fn acquire(state: &State, resource: ResourceId, at: SimTime) -> FlowOwnedCommand {
    FlowOwnedCommand::Acquire(FlowAcquireCommand {
        resource,
        owner: state.owner,
        work: Some(state.task),
        at,
        priority_level: 0,
        deadline: None,
        scheduler_priority: 0,
        timed: true,
        can_preempt: false,
        preemptible: None,
    })
}

fn planned<'a>(
    current: &'a State,
    snapshot: &'a FlowCallbackSnapshot,
    view: FlowWorldView<'a>,
    sink: &'a mut FlowCommandSink,
) -> Result<State, FlowError> {
    assert_eq!(view.now(), snapshot.delivery.at);
    let mut next = current.clone();
    next.commits += 1;

    match current.behavior {
        Behavior::ErrorAfterEmit => {
            sink.emit(acquire(current, current.resource, view.now()))?;
            Err(FlowError::InvalidState)
        }
        Behavior::PoisonSink => {
            // The runtime is configured with a one-command bound. Ignore the
            // second emit error deliberately; the poisoned sink must still fail.
            let _ = sink.emit(acquire(current, current.resource, view.now()));
            let _ = sink.emit(acquire(current, current.resource, view.now()));
            Ok(next)
        }
        Behavior::InvalidBatch => {
            sink.emit(acquire(
                current,
                current.invalid_resource.expect("fixture foreign resource"),
                view.now(),
            ))?;
            Ok(next)
        }
        Behavior::Accept => {
            sink.emit(acquire(current, current.resource, view.now()))?;
            Ok(next)
        }
        Behavior::Controls => {
            let FlowCallbackCause::DomainControl { action, .. } = snapshot.cause else {
                return Err(FlowError::InvalidState);
            };
            next.controls.push((action, view.now()));
            Ok(next)
        }
    }
}

struct Fixture {
    flow: FlowRuntime,
    owner: EntityId,
    resource: ResourceId,
    task: kairo_ecs_des::WorkId,
    carrier: kairo_ecs_des::WorkId,
}

fn fixture(
    behavior: Behavior,
    command_limit: usize,
    invalid_resource: Option<ResourceId>,
) -> Fixture {
    let mut flow = FlowRuntime::with_configs(
        Default::default(),
        FlowCallbackConfig {
            max_callback_commands: NonZeroUsize::new(command_limit).unwrap(),
        },
    );
    flow.register_domain_plan_hook("c20.plan", PLAN_KIND, planned)
        .unwrap();
    flow.register_domain_plan_hook("c20.plan", OTHER_KIND, planned)
        .unwrap();
    let owner = flow.spawn_actor().unwrap();
    let resource = flow.create_resource(1).unwrap();
    let task = flow
        .create_work(owner, SimDuration::from_ticks(20), "c20.task", ())
        .unwrap();
    let carrier = flow
        .create_actor_domain_context(
            owner,
            "c20.plan",
            PLAN_KIND,
            State {
                behavior,
                owner,
                task,
                resource,
                invalid_resource,
                commits: 0,
                controls: Vec::new(),
            },
        )
        .unwrap();
    Fixture {
        flow,
        owner,
        resource,
        task,
        carrier,
    }
}

fn rejected(dispatch: &kairo_ecs_des::FlowDispatch, error: FlowError, has_failed_ticket: bool) {
    assert!(
        dispatch.error.is_none(),
        "domain event itself was delivered"
    );
    assert!(matches!(
        dispatch.callback_batches.as_slice(),
        [FlowBatchReceipt::Rejected(rejection)]
            if rejection.error == error && rejection.failed_ticket.is_some() == has_failed_ticket
    ));
}

#[test]
fn planner_error_discards_staged_context_and_acquire_but_consumes_source_delivery() {
    let mut f = fixture(Behavior::ErrorAfterEmit, 4, None);
    let before = f.flow.budget_snapshot();
    f.flow
        .schedule_domain(f.carrier, PLAN_KIND, t(3), 0)
        .unwrap();

    let dispatch = f.flow.step().unwrap().unwrap();

    rejected(&dispatch, FlowError::InvalidState, false);
    assert_eq!(dispatch.at, t(3));
    assert_eq!(f.flow.now(), t(3));
    assert_eq!(f.flow.work_context::<State>(f.carrier).unwrap().commits, 0);
    assert_eq!(f.flow.work(f.task).unwrap().request, None);
    assert!(f.flow.resource(f.resource).unwrap().queued.is_empty());
    assert!(f.flow.step().unwrap().is_none());
    assert_ne!(f.flow.budget_snapshot(), before);
}

#[test]
fn poisoned_sink_discards_context_and_commands_even_if_planner_returns_ok() {
    let mut f = fixture(Behavior::PoisonSink, 1, None);
    f.flow
        .schedule_domain(f.carrier, PLAN_KIND, t(4), 0)
        .unwrap();

    let dispatch = f.flow.step().unwrap().unwrap();

    rejected(&dispatch, FlowError::CallbackBatchLimitExceeded, false);
    assert_eq!(f.flow.work_context::<State>(f.carrier).unwrap().commits, 0);
    assert_eq!(f.flow.work(f.task).unwrap().request, None);
    assert!(f.flow.resource(f.resource).unwrap().queued.is_empty());
}

#[test]
fn invalid_acquire_batch_discards_context_and_does_not_create_a_request() {
    let mut foreign = FlowRuntime::new();
    foreign.spawn_actor().unwrap();
    foreign.spawn_actor().unwrap();
    let foreign_resource = foreign.create_resource(1).unwrap();
    let mut f = fixture(Behavior::InvalidBatch, 4, Some(foreign_resource));
    f.flow
        .schedule_domain(f.carrier, PLAN_KIND, t(5), 0)
        .unwrap();

    let dispatch = f.flow.step().unwrap().unwrap();

    rejected(&dispatch, FlowError::InvalidResource, true);
    assert_eq!(f.flow.work_context::<State>(f.carrier).unwrap().commits, 0);
    assert_eq!(f.flow.work(f.task).unwrap().request, None);
    assert!(f.flow.resource(f.resource).unwrap().queued.is_empty());
}

#[test]
fn accepted_plan_commits_context_and_actual_timed_flow_claim_together() {
    let mut f = fixture(Behavior::Accept, 4, None);
    f.flow
        .schedule_domain(f.carrier, PLAN_KIND, t(6), 0)
        .unwrap();

    let dispatch = f.flow.step().unwrap().unwrap();

    assert!(dispatch.error.is_none());
    let admissions = match dispatch.callback_batches.as_slice() {
        [FlowBatchReceipt::Accepted(admissions)] => admissions,
        other => panic!("expected accepted plan batch, got {other:?}"),
    };
    assert_eq!(admissions.len(), 1);
    let request = admissions[0].request.expect("timed acquire request");
    assert_eq!(f.flow.work_context::<State>(f.carrier).unwrap().commits, 1);
    assert_eq!(f.flow.work(f.task).unwrap().request, Some(request));
    let actual = f.flow.request(request).unwrap();
    assert_eq!(actual.resource, f.resource);
    assert_eq!(actual.owner, f.owner);
    assert_eq!(actual.work, Some(f.task));
    assert!(actual.timed);
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LegacyState {
    task: kairo_ecs_des::WorkId,
    owner: EntityId,
    foreign_resource: ResourceId,
    calls: u32,
}

fn legacy_mutable<'a>(
    state: &'a mut LegacyState,
    snapshot: &'a FlowCallbackSnapshot,
    view: FlowWorldView<'a>,
    sink: &'a mut FlowCommandSink,
) {
    state.calls += 1;
    sink.emit(FlowOwnedCommand::Acquire(FlowAcquireCommand {
        resource: state.foreign_resource,
        owner: state.owner,
        work: Some(state.task),
        at: view.now(),
        priority_level: 0,
        deadline: None,
        scheduler_priority: 0,
        timed: true,
        can_preempt: false,
        preemptible: None,
    }))
    .unwrap();
    assert_eq!(snapshot.delivery.at, view.now());
}

#[test]
fn legacy_mutable_hook_still_retains_context_effect_on_batch_rejection() {
    let mut foreign = FlowRuntime::new();
    foreign.spawn_actor().unwrap();
    foreign.spawn_actor().unwrap();
    let foreign_resource = foreign.create_resource(1).unwrap();

    let mut flow = FlowRuntime::new();
    flow.register_domain_view_hook("c20.legacy", PLAN_KIND, legacy_mutable)
        .unwrap();
    let owner = flow.spawn_actor().unwrap();
    let resource = flow.create_resource(1).unwrap();
    let task = flow
        .create_work(owner, SimDuration::from_ticks(20), "c20.task", ())
        .unwrap();
    let carrier = flow
        .create_actor_domain_context(
            owner,
            "c20.legacy",
            PLAN_KIND,
            LegacyState {
                task,
                owner,
                foreign_resource,
                calls: 0,
            },
        )
        .unwrap();
    flow.schedule_domain(carrier, PLAN_KIND, t(7), 0).unwrap();

    let dispatch = flow.step().unwrap().unwrap();

    assert!(matches!(
        dispatch.callback_batches.as_slice(),
        [FlowBatchReceipt::Rejected(rejection)]
            if rejection.error == FlowError::InvalidResource && rejection.failed_ticket.is_some()
    ));
    assert_eq!(flow.work_context::<LegacyState>(carrier).unwrap().calls, 1);
    assert_eq!(flow.work(task).unwrap().request, None);
    assert!(flow.resource(resource).unwrap().queued.is_empty());
}

#[test]
fn plan_carrier_is_unique_kind_bound_and_delivers_typed_pause_resume_in_order() {
    let mut f = fixture(Behavior::Controls, 4, None);
    let before = f.flow.budget_snapshot();
    assert_eq!(
        f.flow.create_actor_domain_context(
            f.owner,
            "c20.plan",
            OTHER_KIND,
            State {
                behavior: Behavior::Controls,
                owner: f.owner,
                task: f.task,
                resource: f.resource,
                invalid_resource: None,
                commits: 0,
                controls: Vec::new(),
            },
        ),
        Err(FlowError::DuplicateActorDomainContext)
    );
    assert_eq!(
        f.flow.schedule_domain(f.carrier, OTHER_KIND, t(1), 0),
        Err(FlowError::InvalidWork)
    );
    assert_eq!(
        f.flow
            .schedule_domain_control(f.carrier, OTHER_KIND, FlowDomainControl::Pause, t(1), 0,),
        Err(FlowError::InvalidWork)
    );
    assert_eq!(f.flow.budget_snapshot(), before);

    // Reverse insertion order; scheduler time/priority order is authoritative.
    f.flow
        .schedule_domain_control(f.carrier, PLAN_KIND, FlowDomainControl::Resume, t(8), 1)
        .unwrap();
    f.flow
        .schedule_domain_control(f.carrier, PLAN_KIND, FlowDomainControl::Pause, t(8), -1)
        .unwrap();
    for _ in 0..2 {
        let dispatch = f.flow.step().unwrap().unwrap();
        assert!(dispatch.error.is_none());
        assert!(matches!(
            dispatch.callback_batches.as_slice(),
            [FlowBatchReceipt::Accepted(admissions)] if admissions.is_empty()
        ));
    }
    let actual = f.flow.work_context::<State>(f.carrier).unwrap();
    assert_eq!(
        actual.controls,
        vec![
            (FlowDomainControl::Pause, t(8)),
            (FlowDomainControl::Resume, t(8)),
        ]
    );
    assert_eq!(actual.commits, 2);
    assert!(f.flow.step().unwrap().is_none());
}
