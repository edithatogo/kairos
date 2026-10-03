use std::sync::Mutex;

use kairo_ecs_des::{
    FlowDispatch, FlowError, FlowRuntime, LifecycleTransition, PreemptionStrategy, WorkHandlers,
    WorkProgress, WorkState,
};
use kairo_ecs_types::{SimDuration, SimTime};

#[derive(Clone, Debug, Eq, PartialEq)]
struct Context {
    generation: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CallbackObservation {
    kind: &'static str,
    generation: u32,
    useful_elapsed: u128,
    remaining: u128,
    cumulative_busy: u128,
    attempt_revision: u64,
}

static CALLBACKS: Mutex<Vec<CallbackObservation>> = Mutex::new(Vec::new());

fn observe(kind: &'static str, context: &mut Context, progress: &WorkProgress) {
    CALLBACKS.lock().unwrap().push(CallbackObservation {
        kind,
        generation: context.generation,
        useful_elapsed: progress.useful_elapsed.ticks(),
        remaining: progress.remaining.ticks(),
        cumulative_busy: progress.cumulative_busy.ticks(),
        attempt_revision: progress.attempt_revision,
    });
}

fn on_resume(context: &mut Context, progress: &WorkProgress) {
    observe("resume", context, progress);
}

fn on_restart(context: &mut Context, progress: &WorkProgress) {
    observe("restart", context, progress);
}

fn on_abort(context: &mut Context, progress: &WorkProgress) {
    observe("abort", context, progress);
}

fn fresh_context(template: &Context) -> Context {
    Context {
        generation: template.generation + 1,
    }
}

fn t(ticks: u64) -> SimTime {
    SimTime::from_ticks(ticks.into())
}

fn d(ticks: u64) -> SimDuration {
    SimDuration::from_ticks(ticks.into())
}

fn dispatch_all(flow: &mut FlowRuntime) -> Result<Vec<FlowDispatch>, FlowError> {
    let mut dispatches = Vec::new();
    while let Some(dispatch) = flow.step()? {
        dispatches.push(dispatch);
    }
    Ok(dispatches)
}

fn all_records(dispatches: &[FlowDispatch]) -> Vec<&kairo_ecs_des::LifecycleRecord> {
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
fn timed_work_preemption_primary_oracle_for_all_strategies() -> Result<(), FlowError> {
    for (strategy, expected_low_terminal, expected_low_terminal_at, expected_busy) in [
        (
            PreemptionStrategy::Suspend,
            LifecycleTransition::Completed,
            12,
            10,
        ),
        (
            PreemptionStrategy::Abort,
            LifecycleTransition::Aborted,
            3,
            3,
        ),
        (
            PreemptionStrategy::Restart,
            LifecycleTransition::Completed,
            15,
            13,
        ),
    ] {
        CALLBACKS.lock().unwrap().clear();

        let mut flow = FlowRuntime::new();
        let owner = flow.spawn_actor()?;
        let resource = flow.create_resource(1)?;
        let registration = "q3.primary.low.v1";
        flow.register_work_handlers::<Context>(
            registration,
            WorkHandlers {
                on_resume: Some(on_resume),
                on_restart: Some(on_restart),
                on_abort: Some(on_abort),
                on_cancel: None,
            },
        )?;

        let low = if strategy == PreemptionStrategy::Restart {
            flow.create_restartable_work(
                owner,
                d(10),
                registration,
                Context { generation: 0 },
                fresh_context,
            )?
        } else {
            flow.create_work(owner, d(10), registration, Context { generation: 0 })?
        };
        let low_request = flow
            .acquire(resource)
            .owner(owner)
            .at(t(0))
            .priority(9)
            .timed_work(low)
            .preemptible(strategy)
            .submit()?;

        let urgent = flow.create_work(
            owner,
            d(2),
            "q3.primary.urgent.v1",
            Context { generation: 0 },
        )?;
        let urgent_request = flow
            .acquire(resource)
            .owner(owner)
            .at(t(3))
            .priority(1)
            .timed_work(urgent)
            .can_preempt(true)
            .submit()?;

        // The interruption is committed at tick 3, but its typed callback is a
        // later notification dispatch. In particular, it must not run inline
        // inside arbitration.
        let interruption = flow
            .step()?
            .expect("the low-priority grant at tick 0 must dispatch");
        assert_eq!(interruption.at, t(0));
        let admission = flow
            .step()?
            .expect("the urgent request at tick 3 must dispatch");
        assert_eq!(admission.at, t(3));
        assert!(CALLBACKS.lock().unwrap().is_empty());

        let mut dispatches = vec![interruption, admission];
        dispatches.extend(dispatch_all(&mut flow)?);
        let observations = CALLBACKS.lock().unwrap().clone();
        assert_eq!(observations.len(), 1);
        let callback = &observations[0];
        assert_eq!(
            callback.useful_elapsed,
            if strategy == PreemptionStrategy::Restart {
                0
            } else {
                3
            }
        );
        assert_eq!(
            callback.remaining,
            if strategy == PreemptionStrategy::Restart {
                10
            } else {
                7
            }
        );
        assert_eq!(callback.cumulative_busy, 3);
        assert_eq!(
            callback.kind,
            match strategy {
                PreemptionStrategy::Suspend => "resume",
                PreemptionStrategy::Abort => "abort",
                PreemptionStrategy::Restart => "restart",
            }
        );
        assert_eq!(
            callback.generation,
            if strategy == PreemptionStrategy::Restart {
                1
            } else {
                0
            }
        );
        assert_eq!(
            callback.attempt_revision,
            u64::from(strategy == PreemptionStrategy::Restart)
        );

        let records = all_records(&dispatches);
        assert!(has_transition_at(
            &records,
            low_request,
            0,
            LifecycleTransition::Granted
        ));
        assert!(has_transition_at(
            &records,
            low_request,
            3,
            LifecycleTransition::Preempted
        ));
        assert!(has_transition_at(
            &records,
            urgent_request,
            3,
            LifecycleTransition::Granted
        ));
        assert!(has_transition_at(
            &records,
            urgent_request,
            5,
            LifecycleTransition::Completed
        ));

        match strategy {
            PreemptionStrategy::Suspend => assert!(has_transition_at(
                &records,
                low_request,
                5,
                LifecycleTransition::Resumed
            )),
            PreemptionStrategy::Restart => assert!(has_transition_at(
                &records,
                low_request,
                5,
                LifecycleTransition::Restarted
            )),
            PreemptionStrategy::Abort => {}
        }
        assert!(has_transition_at(
            &records,
            low_request,
            expected_low_terminal_at,
            expected_low_terminal
        ));

        // Processing through an empty scheduler consumes stale original
        // completion tokens without emitting a second terminal transition.
        let low_terminals: Vec<_> = records
            .iter()
            .filter(|record| record.request == low_request && terminal(record))
            .collect();
        assert_eq!(low_terminals.len(), 1);
        assert_eq!(low_terminals[0].transition, expected_low_terminal);
        let urgent_terminals: Vec<_> = records
            .iter()
            .filter(|record| record.request == urgent_request && terminal(record))
            .collect();
        assert_eq!(urgent_terminals.len(), 1);
        assert_eq!(
            urgent_terminals[0].transition,
            LifecycleTransition::Completed
        );

        let low_progress = flow.work_progress(low)?;
        assert_eq!(low_progress.original_duration, d(10));
        assert_eq!(low_progress.cumulative_busy, d(expected_busy));
        match strategy {
            PreemptionStrategy::Abort => {
                assert_eq!(low_progress.state, WorkState::Aborted);
                assert_eq!(low_progress.useful_elapsed, d(3));
                assert_eq!(low_progress.remaining, d(7));
                assert_eq!(flow.work_context::<Context>(low)?.generation, 0);
            }
            PreemptionStrategy::Suspend | PreemptionStrategy::Restart => {
                assert_eq!(low_progress.state, WorkState::Completed);
                assert_eq!(low_progress.useful_elapsed, d(10));
                assert_eq!(low_progress.remaining, d(0));
                assert_eq!(
                    low_progress.attempt_revision,
                    u64::from(strategy == PreemptionStrategy::Restart)
                );
                assert_eq!(low_progress.execution_revision, 2);
                assert_eq!(
                    flow.work_context::<Context>(low)?.generation,
                    if strategy == PreemptionStrategy::Restart {
                        1
                    } else {
                        0
                    }
                );
            }
        }

        let urgent_progress = flow.work_progress(urgent)?;
        assert_eq!(urgent_progress.state, WorkState::Completed);
        assert_eq!(urgent_progress.useful_elapsed, d(2));
        assert_eq!(urgent_progress.cumulative_busy, d(2));
    }
    Ok(())
}
