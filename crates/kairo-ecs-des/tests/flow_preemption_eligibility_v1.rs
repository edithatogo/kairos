use kairo_ecs_des::{
    FlowDispatch, FlowError, FlowRuntime, LifecycleTransition, PreemptionStrategy, RequestState,
    WorkState,
};
use kairo_ecs_types::{SimDuration, SimTime};

const MAX_DISPATCHES: usize = 128;

fn t(ticks: u64) -> SimTime {
    SimTime::from_ticks(ticks.into())
}

fn d(ticks: u64) -> SimDuration {
    SimDuration::from_ticks(ticks.into())
}

fn run_to_empty(flow: &mut FlowRuntime) -> Result<Vec<FlowDispatch>, FlowError> {
    let mut dispatches = Vec::new();
    for _ in 0..MAX_DISPATCHES {
        match flow.step()? {
            Some(dispatch) => {
                assert!(
                    dispatch.error.is_none(),
                    "dispatch reported an error: {dispatch:?}"
                );
                dispatches.push(dispatch);
            }
            None => return Ok(dispatches),
        }
    }
    panic!("dispatch budget exhausted after {MAX_DISPATCHES} scheduler steps");
}

fn assert_dispatch_invariants(dispatches: &[FlowDispatch]) {
    for dispatch in dispatches {
        assert!(
            dispatch.error.is_none(),
            "dispatch reported an error: {dispatch:?}"
        );
        for record in &dispatch.records {
            assert_eq!(record.causal_event_id, dispatch.event);
        }
        for pair in dispatch.records.windows(2) {
            assert_eq!(
                pair[1].transition_ordinal,
                pair[0]
                    .transition_ordinal
                    .checked_add(1)
                    .expect("ordinal overflow"),
                "transition ordinals must be contiguous within a dispatch"
            );
        }
    }
}

fn records(dispatches: &[FlowDispatch]) -> Vec<&kairo_ecs_des::LifecycleRecord> {
    dispatches
        .iter()
        .flat_map(|dispatch| dispatch.records.iter())
        .collect()
}

fn has_transition_at(
    records: &[&kairo_ecs_des::LifecycleRecord],
    request: kairo_ecs_des::RequestId,
    at: u64,
    transition: LifecycleTransition,
) -> bool {
    records.iter().any(|record| {
        record.request == request
            && record.at.ticks() == u128::from(at)
            && record.transition == transition
    })
}

fn terminal(record: &&kairo_ecs_des::LifecycleRecord) -> bool {
    matches!(
        record.transition,
        LifecycleTransition::Completed
            | LifecycleTransition::Aborted
            | LifecycleTransition::Cancelled
            | LifecycleTransition::TimedOut
            | LifecycleTransition::Released
    )
}

#[test]
fn nested_suspend_cycles_and_live_progress_inspection() -> Result<(), FlowError> {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor()?;
    let resource = flow.create_resource(1)?;
    let inspection_resource = flow.create_resource(1)?;
    let low = flow.create_work(owner, d(10), "nested.low.v1", 0_u32)?;
    let low_request = flow
        .acquire(resource)
        .owner(owner)
        .at(t(0))
        .priority(9)
        .deadline(t(1))
        .timed_work(low)
        .preemptible(PreemptionStrategy::Suspend)
        .submit()?;

    let urgent_one = flow.create_work(owner, d(3), "nested.urgent.one.v1", 0_u32)?;
    let urgent_one_request = flow
        .acquire(resource)
        .owner(owner)
        .at(t(4))
        .priority(1)
        .timed_work(urgent_one)
        .can_preempt(true)
        .submit()?;
    let urgent_two = flow.create_work(owner, d(2), "nested.urgent.two.v1", 0_u32)?;
    let urgent_two_request = flow
        .acquire(resource)
        .owner(owner)
        .at(t(9))
        .priority(1)
        .timed_work(urgent_two)
        .can_preempt(true)
        .submit()?;

    let low_grant = flow.step()?.expect("low work should grant at tick 0");
    assert_eq!(low_grant.at, t(0));
    let inspection = flow
        .acquire(inspection_resource)
        .owner(owner)
        .at(t(2))
        .submit()?;
    let inspection_dispatch = flow
        .step()?
        .expect("independent inspection request should dispatch at tick 2");
    assert_eq!(inspection_dispatch.at, t(2));
    assert_eq!(flow.request(inspection)?.state, RequestState::Active);

    // Repeated observation is read-only while the low work is active.
    let first = flow.work_progress(low)?;
    let second = flow.work_progress(low)?;
    assert_eq!(first.useful_elapsed, d(2));
    assert_eq!(first.cumulative_busy, d(2));
    assert_eq!(first.remaining, d(8));
    assert_eq!(second, first);

    let mut dispatches = vec![low_grant, inspection_dispatch];
    dispatches.extend(run_to_empty(&mut flow)?);
    assert_dispatch_invariants(&dispatches);
    let all_records = records(&dispatches);

    assert!(has_transition_at(
        &all_records,
        low_request,
        4,
        LifecycleTransition::Preempted
    ));
    assert!(!all_records.iter().any(|record| {
        record.request == low_request && record.transition == LifecycleTransition::TimedOut
    }));
    assert!(has_transition_at(
        &all_records,
        low_request,
        7,
        LifecycleTransition::Resumed
    ));
    assert!(has_transition_at(
        &all_records,
        low_request,
        9,
        LifecycleTransition::Preempted
    ));
    assert!(has_transition_at(
        &all_records,
        low_request,
        11,
        LifecycleTransition::Resumed
    ));
    assert!(has_transition_at(
        &all_records,
        low_request,
        15,
        LifecycleTransition::Completed
    ));
    assert!(has_transition_at(
        &all_records,
        urgent_one_request,
        7,
        LifecycleTransition::Completed
    ));
    assert!(has_transition_at(
        &all_records,
        urgent_two_request,
        11,
        LifecycleTransition::Completed
    ));
    assert!(!all_records.iter().any(|record| {
        record.request == low_request
            && matches!(record.at, time if time == t(10) || time == t(13))
            && record.transition == LifecycleTransition::Completed
    }));

    let low_terminals: Vec<_> = all_records
        .iter()
        .filter(|record| record.request == low_request && terminal(record))
        .collect();
    assert_eq!(low_terminals.len(), 1);
    assert_eq!(
        all_records
            .iter()
            .filter(|record| record.request == low_request
                && record.transition == LifecycleTransition::Granted)
            .count(),
        1
    );
    assert_eq!(
        all_records
            .iter()
            .filter(|record| record.request == low_request
                && record.transition == LifecycleTransition::Preempted)
            .count(),
        2
    );
    assert_eq!(
        all_records
            .iter()
            .filter(|record| record.request == low_request
                && record.transition == LifecycleTransition::Resumed)
            .count(),
        2
    );
    assert_eq!(
        all_records
            .iter()
            .filter(|record| record.request == low_request
                && record.transition == LifecycleTransition::Restarted)
            .count(),
        0
    );
    let progress = flow.work_progress(low)?;
    assert_eq!(progress.state, WorkState::Completed);
    assert_eq!(progress.original_duration, d(10));
    assert_eq!(progress.useful_elapsed, d(10));
    assert_eq!(progress.cumulative_busy, d(10));
    assert_eq!(progress.remaining, d(0));
    assert_eq!(progress.attempt_revision, 0);
    assert_eq!(progress.execution_revision, 3);
    Ok(())
}

#[test]
fn capacity_two_selects_latest_equal_priority_victim_then_next() -> Result<(), FlowError> {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor()?;
    let resource = flow.create_resource(2)?;
    let first_work = flow.create_work(owner, d(20), "capacity.first.v1", 0_u32)?;
    let first_request = flow
        .acquire(resource)
        .owner(owner)
        .at(t(0))
        .priority(9)
        .timed_work(first_work)
        .preemptible(PreemptionStrategy::Abort)
        .submit()?;
    let second_work = flow.create_work(owner, d(20), "capacity.second.v1", 0_u32)?;
    let second_request = flow
        .acquire(resource)
        .owner(owner)
        .at(t(1))
        .priority(9)
        .timed_work(second_work)
        .preemptible(PreemptionStrategy::Abort)
        .submit()?;
    let urgent_one = flow.create_work(owner, d(2), "capacity.urgent.one.v1", 0_u32)?;
    let urgent_one_request = flow
        .acquire(resource)
        .owner(owner)
        .at(t(3))
        .priority(1)
        .timed_work(urgent_one)
        .can_preempt(true)
        .submit()?;
    let urgent_two = flow.create_work(owner, d(1), "capacity.urgent.two.v1", 0_u32)?;
    let urgent_two_request = flow
        .acquire(resource)
        .owner(owner)
        .at(t(4))
        .priority(0)
        .timed_work(urgent_two)
        .can_preempt(true)
        .submit()?;

    let dispatches = run_to_empty(&mut flow)?;
    assert_dispatch_invariants(&dispatches);
    let all_records = records(&dispatches);
    assert!(has_transition_at(
        &all_records,
        second_request,
        3,
        LifecycleTransition::Preempted
    ));
    assert!(!all_records.iter().any(|record| {
        record.request == first_request
            && record.at == t(3)
            && record.transition == LifecycleTransition::Preempted
    }));
    assert!(has_transition_at(
        &all_records,
        first_request,
        4,
        LifecycleTransition::Preempted
    ));
    assert!(has_transition_at(
        &all_records,
        urgent_one_request,
        3,
        LifecycleTransition::Granted
    ));
    assert!(has_transition_at(
        &all_records,
        urgent_two_request,
        4,
        LifecycleTransition::Granted
    ));
    assert_eq!(flow.work_progress(first_work)?.state, WorkState::Aborted);
    assert_eq!(flow.work_progress(second_work)?.state, WorkState::Aborted);
    assert_eq!(flow.work_progress(second_work)?.cumulative_busy, d(2));
    assert_eq!(flow.work_progress(first_work)?.cumulative_busy, d(4));
    for (request, at) in [(second_request, 3), (first_request, 4)] {
        let aborts: Vec<_> = all_records
            .iter()
            .filter(|record| {
                record.request == request && record.transition == LifecycleTransition::Aborted
            })
            .collect();
        assert_eq!(aborts.len(), 1);
        assert_eq!(aborts[0].at, t(at));
    }
    assert!(!all_records.iter().any(|record| {
        (record.request == first_request && record.at == t(20)
            || record.request == second_request && record.at == t(21))
            && record.transition == LifecycleTransition::Completed
    }));
    Ok(())
}

fn assert_not_preempted(
    holder_preemptible: bool,
    waiter_priority: i32,
    waiter_can_preempt: bool,
) -> Result<(), FlowError> {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor()?;
    let resource = flow.create_resource(1)?;
    let holder = flow.create_work(owner, d(10), "eligibility.holder.v1", 0_u32)?;
    let mut holder_builder = flow
        .acquire(resource)
        .owner(owner)
        .at(t(0))
        .priority(5)
        .timed_work(holder);
    if holder_preemptible {
        holder_builder = holder_builder.preemptible(PreemptionStrategy::Suspend);
    }
    let holder_request = holder_builder.submit()?;

    let waiter = flow.create_work(owner, d(1), "eligibility.waiter.v1", 0_u32)?;
    let mut waiter_builder = flow
        .acquire(resource)
        .owner(owner)
        .at(t(1))
        .priority(waiter_priority)
        .timed_work(waiter);
    if waiter_can_preempt {
        waiter_builder = waiter_builder.can_preempt(true);
    }
    let waiter_request = waiter_builder.submit()?;

    let first = flow.step()?.expect("holder grant should dispatch");
    assert_eq!(first.at, t(0));
    let second = flow.step()?.expect("waiter should be considered at tick 1");
    assert_eq!(second.at, t(1));
    assert_dispatch_invariants(&[first.clone(), second.clone()]);
    assert_eq!(flow.request(waiter_request)?.state, RequestState::Queued);
    assert_eq!(flow.work_progress(holder)?.state, WorkState::Active);
    assert!(!first
        .records
        .iter()
        .chain(second.records.iter())
        .any(|record| {
            record.request == holder_request && record.transition == LifecycleTransition::Preempted
        }));
    Ok(())
}

#[test]
fn equal_priority_missing_capability_and_nonpreemptible_holder_are_ineligible(
) -> Result<(), FlowError> {
    assert_not_preempted(true, 5, true)?;
    assert_not_preempted(true, 1, false)?;
    assert_not_preempted(false, 1, true)?;
    Ok(())
}

#[test]
fn zero_duration_work_completes_at_grant_without_duplicate_terminal() -> Result<(), FlowError> {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor()?;
    let resource = flow.create_resource(1)?;
    let zero = flow.create_work(owner, d(0), "zero.duration.v1", 0_u32)?;
    let zero_request = flow
        .acquire(resource)
        .owner(owner)
        .at(t(0))
        .priority(9)
        .timed_work(zero)
        .preemptible(PreemptionStrategy::Suspend)
        .submit()?;
    let urgent = flow.create_work(owner, d(1), "zero.urgent.v1", 0_u32)?;
    let urgent_request = flow
        .acquire(resource)
        .owner(owner)
        .at(t(0))
        .priority(1)
        .timed_work(urgent)
        .can_preempt(true)
        .submit()?;

    let dispatches = run_to_empty(&mut flow)?;
    assert_dispatch_invariants(&dispatches);
    let all_records = records(&dispatches);
    let zero_terminal: Vec<_> = all_records
        .iter()
        .filter(|record| record.request == zero_request && terminal(record))
        .collect();
    assert_eq!(zero_terminal.len(), 1);
    assert_eq!(zero_terminal[0].transition, LifecycleTransition::Completed);
    assert_eq!(zero_terminal[0].at, t(0));
    let zero_grants: Vec<_> = all_records
        .iter()
        .filter(|record| {
            record.request == zero_request && record.transition == LifecycleTransition::Granted
        })
        .collect();
    assert_eq!(zero_grants.len(), 1);
    let grant_position = all_records
        .iter()
        .position(|record| {
            record.request == zero_request && record.transition == LifecycleTransition::Granted
        })
        .expect("grant record exists");
    let completion_position = all_records
        .iter()
        .position(|record| {
            record.request == zero_request && record.transition == LifecycleTransition::Completed
        })
        .expect("zero-duration completion record exists");
    assert!(grant_position < completion_position);
    assert!(!all_records.iter().any(|record| {
        record.request == zero_request && record.transition == LifecycleTransition::Preempted
    }));
    assert!(has_transition_at(
        &all_records,
        urgent_request,
        0,
        LifecycleTransition::Granted
    ));
    let zero_progress = flow.work_progress(zero)?;
    assert_eq!(zero_progress.state, WorkState::Completed);
    assert_eq!(zero_progress.original_duration, d(0));
    assert_eq!(zero_progress.useful_elapsed, d(0));
    assert_eq!(zero_progress.remaining, d(0));
    assert_eq!(zero_progress.cumulative_busy, d(0));
    Ok(())
}

fn assert_completion_wins_insert_order(
    preemption_submitted_before_grant: bool,
) -> Result<(), FlowError> {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor()?;
    let resource = flow.create_resource(1)?;
    let low = flow.create_work(owner, d(3), "same.tick.low.v1", 0_u32)?;
    let low_request = flow
        .acquire(resource)
        .owner(owner)
        .at(t(0))
        .priority(9)
        .timed_work(low)
        .preemptible(PreemptionStrategy::Suspend)
        .submit()?;

    let urgent = flow.create_work(owner, d(2), "same.tick.urgent.v1", 0_u32)?;
    let mut dispatches = Vec::new();
    let urgent_request = if preemption_submitted_before_grant {
        let request = flow
            .acquire(resource)
            .owner(owner)
            .at(t(3))
            .priority(1)
            .timed_work(urgent)
            .can_preempt(true)
            .submit()?;
        let initial = flow
            .step()?
            .expect("low grant inserts completion after urgent request");
        assert_eq!(initial.at, t(0));
        dispatches.push(initial);
        request
    } else {
        let initial = flow
            .step()?
            .expect("low grant inserts completion before urgent request");
        assert_eq!(initial.at, t(0));
        dispatches.push(initial);
        flow.acquire(resource)
            .owner(owner)
            .at(t(3))
            .priority(1)
            .timed_work(urgent)
            .can_preempt(true)
            .submit()?
    };

    dispatches.extend(run_to_empty(&mut flow)?);
    assert_dispatch_invariants(&dispatches);
    let all_records = records(&dispatches);
    assert!(has_transition_at(
        &all_records,
        low_request,
        3,
        LifecycleTransition::Completed
    ));
    assert!(!all_records.iter().any(|record| {
        record.request == low_request && record.transition == LifecycleTransition::Preempted
    }));
    assert!(has_transition_at(
        &all_records,
        urgent_request,
        3,
        LifecycleTransition::Granted
    ));
    assert!(has_transition_at(
        &all_records,
        urgent_request,
        5,
        LifecycleTransition::Completed
    ));
    let low_terminal: Vec<_> = all_records
        .iter()
        .filter(|record| record.request == low_request && terminal(record))
        .collect();
    assert_eq!(low_terminal.len(), 1);
    let low_completion_dispatch = dispatches
        .iter()
        .find(|dispatch| {
            dispatch.records.iter().any(|record| {
                record.request == low_request && record.transition == LifecycleTransition::Completed
            })
        })
        .expect("low completion dispatch exists");
    assert_eq!(
        low_terminal[0].causal_event_id,
        low_completion_dispatch.event
    );
    let completion_shares_admission_dispatch =
        low_completion_dispatch.records.iter().any(|record| {
            record.request == urgent_request && record.transition == LifecycleTransition::Granted
        });
    assert_eq!(
        completion_shares_admission_dispatch,
        preemption_submitted_before_grant
    );
    Ok(())
}

#[test]
fn completion_at_interruption_tick_wins_both_token_insertion_orders() -> Result<(), FlowError> {
    assert_completion_wins_insert_order(false)?;
    assert_completion_wins_insert_order(true)?;
    Ok(())
}

#[test]
fn active_inspection_at_due_tick_caps_effort_without_committing_completion() -> Result<(), FlowError>
{
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor()?;
    let target = flow.create_resource(1)?;
    let unrelated = flow.create_resource(1)?;
    let work = flow.create_work(owner, d(3), "inspection.due.v1", 0_u32)?;
    let request = flow
        .acquire(target)
        .owner(owner)
        .at(t(0))
        .timed_work(work)
        .submit()?;
    let grant = flow.step()?.expect("timed work grants at tick zero");
    assert_eq!(grant.at, t(0));

    // Dispatch an unrelated command at the completion tick before its priority-0
    // completion token, then inspect without targeting the active work's resource.
    let unrelated_work = flow.create_work(owner, d(1), "inspection.unrelated.v1", 0_u32)?;
    flow.acquire(unrelated)
        .owner(owner)
        .at(t(3))
        .scheduler_priority(-1)
        .timed_work(unrelated_work)
        .submit()?;
    let unrelated_dispatch = flow
        .step()?
        .expect("higher-priority unrelated command dispatches at due tick");
    assert_eq!(unrelated_dispatch.at, t(3));
    assert_dispatch_invariants(&[grant, unrelated_dispatch]);
    let at_due = flow.work_progress(work)?;
    assert_eq!(at_due.state, WorkState::Active);
    assert_eq!(at_due.useful_elapsed, d(3));
    assert_eq!(at_due.remaining, d(0));
    assert_eq!(at_due.cumulative_busy, d(3));
    assert_eq!(at_due.completion_at, Some(t(3)));

    let completion = flow.step()?.expect("target completion dispatch follows");
    assert_eq!(completion.at, t(3));
    assert!(completion.records.iter().any(|record| {
        record.request == request && record.transition == LifecycleTransition::Completed
    }));
    assert_dispatch_invariants(&[completion]);
    assert_eq!(flow.work_progress(work)?.state, WorkState::Completed);
    Ok(())
}

#[test]
fn eligible_waiter_scan_skips_a_then_b_preempts_and_a_gets_next_unit() -> Result<(), FlowError> {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor()?;
    let resource = flow.create_resource(1)?;
    let holder = flow.create_work(owner, d(20), "scan.holder.v1", 0_u32)?;
    let holder_request = flow
        .acquire(resource)
        .owner(owner)
        .at(t(0))
        .priority(9)
        .timed_work(holder)
        .preemptible(PreemptionStrategy::Abort)
        .submit()?;
    let waiter_a = flow.create_work(owner, d(1), "scan.a.v1", 0_u32)?;
    let request_a = flow
        .acquire(resource)
        .owner(owner)
        .at(t(1))
        .priority(1)
        .timed_work(waiter_a)
        .submit()?;
    let waiter_b = flow.create_work(owner, d(2), "scan.b.v1", 0_u32)?;
    let request_b = flow
        .acquire(resource)
        .owner(owner)
        .at(t(3))
        .priority(2)
        .timed_work(waiter_b)
        .can_preempt(true)
        .submit()?;

    let dispatches = run_to_empty(&mut flow)?;
    let all_records = records(&dispatches);
    assert!(has_transition_at(
        &all_records,
        holder_request,
        3,
        LifecycleTransition::Aborted
    ));
    assert!(has_transition_at(
        &all_records,
        request_b,
        3,
        LifecycleTransition::Granted
    ));
    assert!(has_transition_at(
        &all_records,
        request_b,
        5,
        LifecycleTransition::Completed
    ));
    assert!(has_transition_at(
        &all_records,
        request_a,
        5,
        LifecycleTransition::Granted
    ));
    assert_eq!(flow.request(request_a)?.state, RequestState::Active);
    Ok(())
}

#[test]
fn suspended_work_can_be_cancelled_without_later_resume() -> Result<(), FlowError> {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor()?;
    let resource = flow.create_resource(1)?;
    let low = flow.create_work(owner, d(10), "cancel.suspended.low.v1", 0_u32)?;
    let low_request = flow
        .acquire(resource)
        .owner(owner)
        .at(t(0))
        .priority(9)
        .timed_work(low)
        .preemptible(PreemptionStrategy::Suspend)
        .submit()?;
    let urgent = flow.create_work(owner, d(5), "cancel.suspended.urgent.v1", 0_u32)?;
    flow.acquire(resource)
        .owner(owner)
        .at(t(3))
        .priority(1)
        .timed_work(urgent)
        .can_preempt(true)
        .submit()?;
    let rescue = flow.create_work(owner, d(1), "cancel.suspended.rescue.v1", 0_u32)?;
    let rescue_request = flow
        .acquire(resource)
        .owner(owner)
        .at(t(5))
        .priority(2)
        .timed_work(rescue)
        .submit()?;

    let grant = flow.step()?.expect("low work should grant");
    assert_eq!(grant.at, t(0));
    let preemption = flow.step()?.expect("urgent work should suspend low at 3");
    assert_eq!(preemption.at, t(3));
    flow.cancel(low_request, t(4))?;
    let mut dispatches = vec![grant, preemption];
    dispatches.extend(run_to_empty(&mut flow)?);
    assert_dispatch_invariants(&dispatches);
    let all_records = records(&dispatches);
    assert!(has_transition_at(
        &all_records,
        low_request,
        4,
        LifecycleTransition::Cancelled
    ));
    assert!(!all_records.iter().any(|record| {
        record.request == low_request && record.transition == LifecycleTransition::Resumed
    }));
    let low_terminals: Vec<_> = all_records
        .iter()
        .filter(|record| record.request == low_request && terminal(record))
        .collect();
    assert_eq!(low_terminals.len(), 1);
    assert_eq!(
        all_records
            .iter()
            .filter(|record| record.request == low_request
                && record.transition == LifecycleTransition::Preempted)
            .count(),
        1
    );
    let progress = flow.work_progress(low)?;
    assert_eq!(progress.state, WorkState::Cancelled);
    assert_eq!(progress.useful_elapsed, d(3));
    assert_eq!(progress.remaining, d(7));
    assert_eq!(progress.cumulative_busy, d(3));
    assert_eq!(progress.attempt_revision, 0);
    assert_eq!(progress.execution_revision, 1);
    assert_eq!(flow.request(rescue_request)?.state, RequestState::Completed);
    assert_eq!(flow.work_progress(rescue)?.state, WorkState::Completed);
    assert!(has_transition_at(
        &all_records,
        rescue_request,
        8,
        LifecycleTransition::Granted
    ));
    assert!(has_transition_at(
        &all_records,
        rescue_request,
        9,
        LifecycleTransition::Completed
    ));
    assert_eq!(
        all_records
            .iter()
            .filter(|record| record.request == rescue_request
                && record.transition == LifecycleTransition::Granted)
            .count(),
        1
    );
    Ok(())
}
