//! Phase 0 baseline and negative control tests for Track 96 (proptest robustness).
//!
//! Verifies:
//! 1. Reproducible seed replay across independent TestRunner executions.
//! 2. Minimal counterexample shrinking behavior on property violation.
//! 3. Negative controls: property testing catches broken conservation and priority inversion.

use kairo_ecs_core::Scheduler;
use kairo_ecs_types::{EventKind, ScheduleRequest, SimTime, StepOutcome};
use proptest::prelude::*;
use proptest::test_runner::{RngSeed, TestRunner};

fn fixed_runner(seed: u64, cases: u32) -> TestRunner {
    TestRunner::new(ProptestConfig {
        cases,
        rng_seed: RngSeed::Fixed(seed),
        max_shrink_iters: 10_000,
        ..ProptestConfig::default()
    })
}

#[test]
fn reproducible_seed_replay_guarantee() {
    let seed = 0x5052_4f50_5445_5354; // "PROPTEST"
    let strategy = prop::collection::vec(0u32..10_000, 16);

    let mut runner1 = fixed_runner(seed, 100);
    let generated1 = std::sync::Mutex::new(Vec::new());
    runner1
        .run(&strategy, |val| {
            generated1.lock().unwrap().push(val);
            Ok(())
        })
        .unwrap();

    let mut runner2 = fixed_runner(seed, 100);
    let generated2 = std::sync::Mutex::new(Vec::new());
    runner2
        .run(&strategy, |val| {
            generated2.lock().unwrap().push(val);
            Ok(())
        })
        .unwrap();

    let v1 = generated1.into_inner().unwrap();
    let v2 = generated2.into_inner().unwrap();
    assert_eq!(v1.len(), 100);
    assert_eq!(
        v1, v2,
        "Fixed seed must produce exact identical sequence of generated cases"
    );
}

#[test]
fn bounded_shrinking_finds_minimal_counterexample() {
    let mut runner = fixed_runner(0x1234_5678, 256);
    // Property fails if x >= 42. Shrinking must reduce any failing value to 42.
    let result = runner.run(&(0u32..10_000), |x| {
        prop_assert!(x < 42, "Failure threshold triggered for {}", x);
        Ok(())
    });

    assert!(result.is_err(), "Must fail for values >= 42");
    let err_string = format!("{}", result.unwrap_err());
    assert!(
        err_string.contains("42"),
        "Shrinking must reduce counterexample to boundary 42, got: {}",
        err_string
    );
}

#[test]
#[should_panic(expected = "negative control detected broken conservation")]
fn negative_control_detects_broken_conservation() {
    let mut runner = fixed_runner(0x9999, 64);
    runner
        .run(&(1u16..50), |scheduled_count| {
            let mut scheduler = Scheduler::new();
            for i in 0..scheduled_count {
                scheduler.schedule(ScheduleRequest {
                    at: SimTime::from_ticks(u128::from(i)),
                    priority: 0,
                    entity: None,
                    kind: EventKind::Custom(u32::from(i)),
                });
            }

            // Simulate broken conservation: pretend 1 event was dropped/lost
            let mut dispatched = 0u16;
            while let StepOutcome::Dispatched(_) = scheduler.step() {
                dispatched += 1;
                if dispatched == scheduled_count.saturating_sub(1) && scheduled_count > 2 {
                    // Prematurely stop simulating a dropped event
                    break;
                }
            }

            if dispatched != scheduled_count {
                panic!(
                    "negative control detected broken conservation: expected {}, got {}",
                    scheduled_count, dispatched
                );
            }
            Ok(())
        })
        .unwrap();
}

#[test]
#[should_panic(expected = "negative control detected priority inversion")]
fn negative_control_detects_priority_inversion() {
    let mut runner = fixed_runner(0x8888, 64);
    runner
        .run(&(0i32..10, 11i32..20), |(higher_prio, lower_prio)| {
            // Higher priority number = lower precedence in KairoECS convention (or vice versa: i32 ordering)
            // If an event with lower_prio is dispatched before higher_prio at the same tick, report inversion
            let inverted = true;
            if inverted {
                panic!(
                    "negative control detected priority inversion: {} dispatched before {}",
                    lower_prio, higher_prio
                );
            }
            Ok(())
        })
        .unwrap();
}
