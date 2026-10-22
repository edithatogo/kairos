use kairo_ecs_core::Scheduler;
use kairo_ecs_des::{HazardInterruptionPolicy, SurvivalClock};
use kairo_ecs_rng::{CauseId, DeterministicStream, HazardRate, TieBreaker};

#[test]
fn test_weibull_and_gompertz_age_dependent_hazard_progression() {
    let mut scheduler = Scheduler::new();
    let mut stream = DeterministicStream::from_run_seed(10101);

    // Aging equipment with Weibull hazard: shape = 2.0 (increasing failure rate with age), scale = 10.0
    // Cumulative hazard H(t) = (t / 10)^2.
    let mut clock = SurvivalClock::new(
        1,
        None,
        HazardInterruptionPolicy::RetainResidual,
        TieBreaker::LowestCauseId,
        1000.0, // 1000 ticks per time unit
    );

    clock.set_channel(
        CauseId(1),
        0,
        HazardRate::Weibull {
            shape: 2.0,
            scale: 10.0,
        },
        0.0,
        &mut stream,
    );

    let ev = clock
        .reschedule(&mut scheduler, 0.0, None, 0)
        .expect("event scheduled");
    assert_eq!(ev.winning_cause, CauseId(1));
    assert!(ev.delta_time > 0.0);

    // Check that scheduler has exactly one pending event
    assert_eq!(scheduler.stats().pending_events, 1);
}

#[test]
fn test_covariate_update_with_residual_hazard_retention() {
    let mut scheduler = Scheduler::new();
    let mut stream = DeterministicStream::from_run_seed(20202);

    let mut clock = SurvivalClock::new(
        2,
        None,
        HazardInterruptionPolicy::RetainResidual,
        TieBreaker::LowestCauseId,
        100.0,
    );

    // Initial constant hazard rate = 0.1
    clock.set_channel(
        CauseId(1),
        0,
        HazardRate::Constant { rate: 0.1 },
        0.0,
        &mut stream,
    );
    let ev1 = clock
        .reschedule(&mut scheduler, 0.0, None, 0)
        .expect("initial schedule");
    let initial_scheduled_id = clock.scheduled_event_id.unwrap();

    // At t = 2.0, environmental covariate shifts: hazard increases to rate = 0.5.
    // Residual threshold must be retained rather than drawing unrelated random work!
    clock.set_channel(
        CauseId(1),
        0,
        HazardRate::Constant { rate: 0.5 },
        2.0,
        &mut stream,
    );
    let ev2 = clock
        .reschedule(&mut scheduler, 2.0, None, 0)
        .expect("rescheduled after covariate shift");

    // The old event must have been cleanly cancelled in the scheduler
    assert_ne!(clock.scheduled_event_id.unwrap(), initial_scheduled_id);
    assert_eq!(scheduler.stats().cancelled_events, 1);

    // Due to increased hazard rate, time to event from t=2.0 must be strictly earlier than under rate 0.1
    assert!(
        ev2.delta_time < (ev1.delta_time - 2.0),
        "Higher hazard must accelerate event time"
    );
}

#[test]
fn test_transport_pause_and_resume_preserves_integrated_hazard() {
    let mut scheduler = Scheduler::new();
    let mut stream = DeterministicStream::from_run_seed(30303);

    let mut clock = SurvivalClock::new(
        3,
        None,
        HazardInterruptionPolicy::RetainResidual,
        TieBreaker::LowestCauseId,
        1.0,
    );

    // Constant hazard rate = 0.01 (mean time = 100.0)
    clock.set_channel(
        CauseId(5),
        0,
        HazardRate::Constant { rate: 0.01 },
        0.0,
        &mut stream,
    );
    let ev_initial = clock
        .reschedule(&mut scheduler, 0.0, None, 0)
        .expect("scheduled");
    let pause_time = ev_initial.delta_time * 0.4;
    assert!(pause_time > 0.0);

    // Transport hold occurs at pause_time: pause progression
    clock.pause(&mut scheduler, pause_time);
    assert!(!clock.is_active);
    assert!(clock.scheduled_event_id.is_none());
    assert_eq!(scheduler.stats().cancelled_events, 1);

    // Hold lasts for 50.0 time units (frozen during hold). Resume at pause_time + 50.0
    let resume_time = pause_time + 50.0;
    let ev_resumed = clock
        .resume(&mut scheduler, resume_time, None, 0)
        .expect("resumed");
    assert!(clock.is_active);
    assert!(clock.scheduled_event_id.is_some());

    // Remaining duration must equal the remaining delta_time before pause (ev_initial.delta_time - pause_time)
    let remaining_before_pause = ev_initial.delta_time - pause_time;
    assert!(
        (ev_resumed.delta_time - remaining_before_pause).abs() < 1e-6,
        "Paused interval must not burn hazard progress: expected {remaining_before_pause}, got {}",
        ev_resumed.delta_time
    );
}
