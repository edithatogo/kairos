use kairo_ecs_core::Scheduler;
use kairo_ecs_des::{HazardInterruptionPolicy, SurvivalClock};
use kairo_ecs_rng::{CauseId, DeterministicStream, HazardRate, TieBreaker};
use kairo_ecs_types::{EventKind, SimTime, StepOutcome};

const CAUSE: CauseId = CauseId(5);

fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() <= 1e-12, "{actual} vs {expected}");
}

fn new_clock(policy: HazardInterruptionPolicy) -> (SurvivalClock, Scheduler, DeterministicStream) {
    (
        SurvivalClock::new(88, None, policy, TieBreaker::LowestCauseId, 1000.0),
        Scheduler::new(),
        DeterministicStream::from_run_seed(11902),
    )
}

fn weibull() -> HazardRate {
    HazardRate::Weibull {
        shape: 2.0,
        scale: 2.0,
    }
}

#[test]
fn reading_a_running_clock_later_does_not_postpone_its_event() {
    let (mut clock, _, mut rng) = new_clock(HazardInterruptionPolicy::RetainResidual);
    clock.set_channel(CAUSE, 0, weibull(), 2.0, &mut rng);
    clock.channels.get_mut(&CAUSE).unwrap().target_threshold = 4.0;
    // At start t=2 the Weibull clock needs age 4 to consume four units.
    for now in [2.0, 3.0, 5.5] {
        let (cause, at) = clock.compute_earliest_event(now).unwrap();
        assert_eq!(cause, CAUSE);
        close(at, 6.0);
    }
}

#[test]
fn pause_and_checkpoint_keep_weibull_age_and_budget() {
    let (mut clock, mut scheduler, mut rng) = new_clock(HazardInterruptionPolicy::RetainResidual);
    clock.set_channel(CAUSE, 0, weibull(), 2.0, &mut rng);
    clock.channels.get_mut(&CAUSE).unwrap().target_threshold = 4.0;
    close(
        clock
            .reschedule(&mut scheduler, 2.0, None, 0)
            .unwrap()
            .delta_time,
        4.0,
    );
    clock.pause(&mut scheduler, 3.0);
    close(clock.channels[&CAUSE].accumulated_hazard, 0.25);
    assert_eq!(scheduler.stats().pending_events, 0);
    let checkpoint = scheduler.checkpoint();

    for (mut clock, mut scheduler) in [
        (
            clock.clone(),
            Scheduler::from_checkpoint(&checkpoint).unwrap(),
        ),
        (clock, scheduler),
    ] {
        // Active age was one at pause. During a seven-unit hold it stays one.
        // Three further units take the process to age four and exhaust H=4.
        let result = clock.resume(&mut scheduler, 10.0, None, 0).unwrap();
        close(result.delta_time, 3.0);
        let StepOutcome::Dispatched(event) = scheduler.step() else {
            panic!("expected scheduled hazard");
        };
        assert_eq!(event.at, SimTime::from_ticks(13_000));
        assert_eq!(event.kind, EventKind::Custom(CAUSE.0));
        assert_eq!(clock.on_event_dispatched(), Some(result));
        assert_eq!(scheduler.step(), StepOutcome::Empty);
    }
}

#[test]
fn replacing_a_channel_during_hold_respects_the_selected_policy() {
    for policy in [
        HazardInterruptionPolicy::RetainResidual,
        HazardInterruptionPolicy::Restart,
    ] {
        let (mut clock, mut scheduler, mut rng) = new_clock(policy);
        clock.set_channel(CAUSE, 0, weibull(), 0.0, &mut rng);
        clock.channels.get_mut(&CAUSE).unwrap().target_threshold = 4.0;
        clock.reschedule(&mut scheduler, 0.0, None, 0).unwrap();
        clock.pause(&mut scheduler, 1.0);
        let before = rng.word_pos();
        clock.set_channel(CAUSE, 0, HazardRate::Constant { rate: 0.5 }, 4.0, &mut rng);
        let expected = if policy == HazardInterruptionPolicy::RetainResidual {
            assert_eq!(rng.word_pos(), before);
            (4.0 - 0.25) / 0.5
        } else {
            assert!(rng.word_pos() > before);
            close(clock.channels[&CAUSE].accumulated_hazard, 0.0);
            clock.channels[&CAUSE].target_threshold / 0.5
        };
        // Independent censoring oracle with a cutoff before the winning time.
        let censor_at = 8.0 + expected / 2.0;
        let result = clock
            .resume(&mut scheduler, 8.0, Some(censor_at), 0)
            .unwrap();
        assert!(result.is_censored);
        close(result.delta_time, expected / 2.0);
        let StepOutcome::Dispatched(event) = scheduler.step() else {
            panic!("expected scheduled censoring");
        };
        assert_eq!(
            event.at,
            SimTime::from_ticks((censor_at * 1000.0).round() as u128)
        );
    }
}
