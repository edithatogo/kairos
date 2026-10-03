use kairo_ecs_des::{
    FlowDispatch, FlowError, FlowRuntime, LifecycleTransition, PreemptionStrategy, WorkState,
};
use kairo_ecs_types::{SimDuration, SimTime};

fn t(ticks: u64) -> SimTime {
    SimTime::from_ticks(ticks.into())
}

fn d(ticks: u64) -> SimDuration {
    SimDuration::from_ticks(ticks.into())
}

fn make_context(template: &u32) -> u32 {
    *template + 1
}

fn run_to_empty(flow: &mut FlowRuntime) -> Result<Vec<FlowDispatch>, FlowError> {
    let mut dispatches = Vec::new();
    while let Some(dispatch) = flow.step()? {
        dispatches.push(dispatch);
    }
    Ok(dispatches)
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

fn is_terminal(record: &&kairo_ecs_des::LifecycleRecord) -> bool {
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
fn independent_timed_preemption_timeline_0_4_7() -> Result<(), FlowError> {
    for (strategy, low_terminal, low_terminal_at, low_busy) in [
        (
            PreemptionStrategy::Suspend,
            LifecycleTransition::Completed,
            13,
            10,
        ),
        (
            PreemptionStrategy::Abort,
            LifecycleTransition::Aborted,
            4,
            4,
        ),
        (
            PreemptionStrategy::Restart,
            LifecycleTransition::Completed,
            17,
            14,
        ),
    ] {
        let mut flow = FlowRuntime::new();
        let owner = flow.spawn_actor()?;
        let resource = flow.create_resource(1)?;

        let registration = "q3.secondary.low.v1";
        let low = if strategy == PreemptionStrategy::Restart {
            flow.create_restartable_work(owner, d(10), registration, 0_u32, make_context)?
        } else {
            flow.create_work(owner, d(10), registration, 0_u32)?
        };
        let low_request = flow
            .acquire(resource)
            .owner(owner)
            .at(t(0))
            .priority(9)
            .timed_work(low)
            .preemptible(strategy)
            .submit()?;

        let urgent = flow.create_work(owner, d(3), "q3.secondary.urgent.v1", 0_u32)?;
        let urgent_request = flow
            .acquire(resource)
            .owner(owner)
            .at(t(4))
            .priority(1)
            .timed_work(urgent)
            .can_preempt(true)
            .submit()?;

        let low_grant = flow
            .step()?
            .expect("low-priority request must grant at tick 0");
        assert_eq!(low_grant.at, t(0));
        let urgent_admission = flow.step()?.expect("urgent request must preempt at tick 4");
        assert_eq!(urgent_admission.at, t(4));

        let mut dispatches = vec![low_grant, urgent_admission];
        dispatches.extend(run_to_empty(&mut flow)?);
        let records = records(&dispatches);

        assert!(has_transition_at(
            &records,
            low_request,
            0,
            LifecycleTransition::Granted
        ));
        assert!(has_transition_at(
            &records,
            low_request,
            4,
            LifecycleTransition::Preempted
        ));
        assert!(has_transition_at(
            &records,
            urgent_request,
            4,
            LifecycleTransition::Granted
        ));
        assert!(has_transition_at(
            &records,
            urgent_request,
            7,
            LifecycleTransition::Completed
        ));
        match strategy {
            PreemptionStrategy::Suspend => assert!(has_transition_at(
                &records,
                low_request,
                7,
                LifecycleTransition::Resumed
            )),
            PreemptionStrategy::Restart => assert!(has_transition_at(
                &records,
                low_request,
                7,
                LifecycleTransition::Restarted
            )),
            PreemptionStrategy::Abort => {}
        }
        assert!(has_transition_at(
            &records,
            low_request,
            low_terminal_at,
            low_terminal
        ));

        // The original low-work completion at tick 10 becomes stale after
        // interruption. Draining to an empty scheduler must not emit it.
        assert!(!records.iter().any(|record| {
            record.request == low_request
                && record.at == t(10)
                && record.transition == LifecycleTransition::Completed
        }));
        let low_terminals: Vec<_> = records
            .iter()
            .filter(|record| record.request == low_request && is_terminal(record))
            .collect();
        assert_eq!(low_terminals.len(), 1);
        assert_eq!(low_terminals[0].transition, low_terminal);
        let urgent_terminals: Vec<_> = records
            .iter()
            .filter(|record| record.request == urgent_request && is_terminal(record))
            .collect();
        assert_eq!(urgent_terminals.len(), 1);
        assert_eq!(
            urgent_terminals[0].transition,
            LifecycleTransition::Completed
        );

        let low_progress = flow.work_progress(low)?;
        assert_eq!(low_progress.original_duration, d(10));
        assert_eq!(low_progress.cumulative_busy, d(low_busy));
        match strategy {
            PreemptionStrategy::Abort => {
                assert_eq!(low_progress.state, WorkState::Aborted);
                assert_eq!(low_progress.useful_elapsed, d(4));
                assert_eq!(low_progress.remaining, d(6));
                assert_eq!(low_progress.execution_revision, 1);
            }
            PreemptionStrategy::Suspend | PreemptionStrategy::Restart => {
                assert_eq!(low_progress.state, WorkState::Completed);
                assert_eq!(low_progress.useful_elapsed, d(10));
                assert_eq!(low_progress.remaining, d(0));
                assert_eq!(low_progress.execution_revision, 2);
                assert_eq!(
                    low_progress.attempt_revision,
                    u64::from(strategy == PreemptionStrategy::Restart)
                );
            }
        }
        let urgent_progress = flow.work_progress(urgent)?;
        assert_eq!(urgent_progress.state, WorkState::Completed);
        assert_eq!(urgent_progress.useful_elapsed, d(3));
        assert_eq!(urgent_progress.cumulative_busy, d(3));
    }
    Ok(())
}
