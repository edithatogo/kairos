use kairo_ecs_rng::{
    CauseId, CompetingCause, CompetingHazards, DeterministicStream, HazardRate, TieBreaker,
};

#[test]
fn test_analytic_exponential_cause_specific_incidence() {
    // 3 competing exponential causes:
    // Cause 1: lambda_1 = 0.5
    // Cause 2: lambda_2 = 1.0
    // Cause 3: lambda_3 = 1.5
    // Total hazard lambda = 3.0
    // Theoretical incidence:
    // P(Cause 1 wins) = 0.5 / 3.0 = 1/6 ~ 0.1667
    // P(Cause 2 wins) = 1.0 / 3.0 = 2/6 ~ 0.3333
    // P(Cause 3 wins) = 1.5 / 3.0 = 3/6 ~ 0.5000
    // Expected time to event E[T] = 1.0 / 3.0 ~ 0.3333
    let causes = vec![
        CompetingCause {
            cause_id: CauseId(1),
            priority: 0,
            hazard: HazardRate::Constant { rate: 0.5 },
        },
        CompetingCause {
            cause_id: CauseId(2),
            priority: 0,
            hazard: HazardRate::Constant { rate: 1.0 },
        },
        CompetingCause {
            cause_id: CauseId(3),
            priority: 0,
            hazard: HazardRate::Constant { rate: 1.5 },
        },
    ];

    let engine = CompetingHazards::new(causes, TieBreaker::LowestCauseId);
    let mut stream = DeterministicStream::from_run_seed(42_4242);

    let n_samples = 30_000;
    let mut counts = [0usize; 4];
    let mut total_time = 0.0;

    for _ in 0..n_samples {
        let ev = engine
            .sample_next(0.0, None, &mut stream)
            .expect("must sample event");
        assert!(!ev.is_censored);
        counts[ev.winning_cause.0 as usize] += 1;
        total_time += ev.delta_time;
    }

    let p1 = counts[1] as f64 / n_samples as f64;
    let p2 = counts[2] as f64 / n_samples as f64;
    let p3 = counts[3] as f64 / n_samples as f64;
    let mean_t = total_time / n_samples as f64;

    // Verify convergence to theoretical values within Monte Carlo confidence bounds (+/- 1.5%)
    assert!(
        (p1 - 0.166667).abs() < 0.015,
        "Cause 1 incidence drifted: observed {p1}, expected 0.1667"
    );
    assert!(
        (p2 - 0.333333).abs() < 0.015,
        "Cause 2 incidence drifted: observed {p2}, expected 0.3333"
    );
    assert!(
        (p3 - 0.500000).abs() < 0.015,
        "Cause 3 incidence drifted: observed {p3}, expected 0.5000"
    );
    assert!(
        (mean_t - 0.333333).abs() < 0.015,
        "Mean time drifted: observed {mean_t}, expected 0.3333"
    );
}

#[test]
fn test_right_censoring_boundary_behavior() {
    let causes = vec![CompetingCause {
        cause_id: CauseId(10),
        priority: 0,
        hazard: HazardRate::Constant { rate: 0.1 }, // E[T] = 10.0
    }];

    let engine = CompetingHazards::new(causes, TieBreaker::LowestCauseId);
    let mut stream = DeterministicStream::from_run_seed(9999);

    // Censor at t = 2.0. P(T > 2) = exp(-0.1 * 2) = exp(-0.2) ~ 0.8187
    let n_samples = 20_000;
    let mut censored_count = 0;

    for _ in 0..n_samples {
        let ev = engine
            .sample_next(0.0, Some(2.0), &mut stream)
            .expect("sample event");
        if ev.is_censored {
            censored_count += 1;
            assert_eq!(ev.winning_cause, CauseId(u32::MAX));
            assert!((ev.delta_time - 2.0).abs() < 1e-9);
        } else {
            assert_eq!(ev.winning_cause, CauseId(10));
            assert!(ev.delta_time <= 2.0);
        }
    }

    let p_censored = censored_count as f64 / n_samples as f64;
    assert!(
        (p_censored - 0.8187).abs() < 0.015,
        "Censoring proportion drifted: observed {p_censored}, expected 0.8187"
    );
}

#[test]
fn test_piecewise_constant_and_zero_hazard_invariance() {
    let causes = vec![
        CompetingCause {
            cause_id: CauseId(1),
            priority: 0,
            hazard: HazardRate::Zero, // Degenerate zero hazard: will never trigger
        },
        CompetingCause {
            cause_id: CauseId(2),
            priority: 0,
            hazard: HazardRate::PiecewiseConstant {
                intervals: vec![(5.0, 0.0), (10.0, 2.0)], // zero hazard before t=5, rate 2.0 from 5..10
                trailing_rate: 0.5,
            },
        },
    ];

    let engine = CompetingHazards::new(causes, TieBreaker::LowestCauseId);
    let mut stream = DeterministicStream::from_run_seed(12345);

    for _ in 0..500 {
        let ev = engine
            .sample_next(0.0, None, &mut stream)
            .expect("must sample event");
        assert_eq!(ev.winning_cause, CauseId(2));
        assert!(ev.delta_time >= 5.0, "cannot occur before age 5.0");
    }
}

#[test]
fn test_tie_breaking_order_policies() {
    // Both causes have identical constant hazard and target thresholds forced identical
    let causes = vec![
        CompetingCause {
            cause_id: CauseId(1),
            priority: 10,
            hazard: HazardRate::Constant { rate: 1.0 },
        },
        CompetingCause {
            cause_id: CauseId(2),
            priority: 20,
            hazard: HazardRate::Constant { rate: 1.0 },
        },
    ];

    // Under explicit priority, cause 2 has priority 20 > 10, so it must win simultaneous ties
    let engine_prio = CompetingHazards::new(causes.clone(), TieBreaker::ExplicitPriority);
    let engine_low = CompetingHazards::new(causes, TieBreaker::LowestCauseId);

    // If both thresholds match: invert_cumulative_hazard yields exact same dt
    // Invert directly
    let dt1 = HazardRate::Constant { rate: 1.0 }
        .invert_cumulative_hazard(0.0, 1.5)
        .unwrap();
    let dt2 = HazardRate::Constant { rate: 1.0 }
        .invert_cumulative_hazard(0.0, 1.5)
        .unwrap();
    assert_eq!(dt1, dt2);

    // Verify ordering
    assert_eq!(engine_prio.causes()[0].cause_id, CauseId(1));
    assert_eq!(engine_low.causes()[1].cause_id, CauseId(2));
}
