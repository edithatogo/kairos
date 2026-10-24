#![forbid(unsafe_code)]

use kairo_ecs_rng::{
    draw_standard_threshold, draw_uniform_open01, CauseId, CompetingCause, CompetingHazards,
    DeterministicStream, HazardRate, TieBreaker,
};

#[test]
fn test_draw_uniform_open01_strictly_open_and_unclamped() {
    let mut stream = DeterministicStream::from_run_seed(2001);
    let n = 100_000;
    let mut min_val = 1.0f64;
    let mut max_val = 0.0f64;

    for _ in 0..n {
        let u = draw_uniform_open01(&mut stream);
        assert!(u > 0.0, "draw_uniform_open01 must be strictly > 0.0");
        assert!(u < 1.0, "draw_uniform_open01 must be strictly < 1.0");
        if u < min_val {
            min_val = u;
        }
        if u > max_val {
            max_val = u;
        }
    }

    // Verify support coverage without artificial truncation
    assert!(
        min_val < 0.001,
        "expected samples close to 0.0, min was {min_val}"
    );
    assert!(
        max_val > 0.999,
        "expected samples close to 1.0, max was {max_val}"
    );
}

#[test]
fn test_draw_standard_threshold_exponential_density() {
    let mut stream = DeterministicStream::from_run_seed(2002);
    let n = 50_000;
    let mut sum = 0.0;
    let mut sum_sq = 0.0;
    let mut tail_exceedances = 0usize;

    for _ in 0..n {
        let w = draw_standard_threshold(&mut stream);
        assert!(w >= 0.0, "Standard exponential threshold must be >= 0.0");
        sum += w;
        sum_sq += w * w;
        // P(W > 3.0) = exp(-3.0) ~ 0.049787
        if w > 3.0 {
            tail_exceedances += 1;
        }
    }

    let mean = sum / n as f64;
    let variance = (sum_sq / n as f64) - (mean * mean);
    let tail_p = tail_exceedances as f64 / n as f64;

    // Standard Exp(1): Mean = 1.0, Variance = 1.0, P(W > 3) ~ 0.04979
    assert!(
        (mean - 1.0).abs() < 0.02,
        "Threshold mean drifted: observed {mean}, expected 1.0"
    );
    assert!(
        (variance - 1.0).abs() < 0.04,
        "Threshold variance drifted: observed {variance}, expected 1.0"
    );
    assert!(
        (tail_p - 0.04979).abs() < 0.005,
        "Threshold tail probability drifted: observed {tail_p}, expected 0.04979"
    );
}

#[test]
fn test_residual_survival_clock_invariance_under_covariate_shift() {
    // Model an entity whose baseline hazard rate shifts from lambda_1 = 0.2 to lambda_2 = 0.8
    // at intervention time t_shift = 2.0.
    // Residual threshold policy: remaining threshold W_rem = W_total - H(t_shift).
    let mut stream = DeterministicStream::from_run_seed(2003);
    let n = 30_000;
    let t_shift = 2.0;
    let lambda_1 = 0.2;
    let lambda_2 = 0.8;

    let mut survived_to_shift = 0usize;
    let mut total_event_time_given_survival = 0.0;

    for _ in 0..n {
        let w = draw_standard_threshold(&mut stream);
        let h_shift = lambda_1 * t_shift; // 0.4
        if w > h_shift {
            survived_to_shift += 1;
            // Residual threshold after shift
            let w_rem = w - h_shift;
            let delta_t_after = w_rem / lambda_2;
            total_event_time_given_survival += t_shift + delta_t_after;
        }
    }

    let p_surv = survived_to_shift as f64 / n as f64;
    // Theoretical survival to t_shift: S(2.0) = exp(-0.2 * 2.0) = exp(-0.4) ~ 0.67032
    assert!(
        (p_surv - 0.67032).abs() < 0.015,
        "Survival to shift drifted: observed {p_surv}, expected 0.67032"
    );

    // Given survival to t_shift, remaining lifetime Delta T ~ Exp(lambda_2 = 0.8)
    // Mean Delta T = 1 / 0.8 = 1.25. Total mean event time = 2.0 + 1.25 = 3.25.
    let mean_time_given_surv = total_event_time_given_survival / survived_to_shift as f64;
    assert!(
        (mean_time_given_surv - 3.25).abs() < 0.03,
        "Mean event time given survival drifted: observed {mean_time_given_surv}, expected 3.25"
    );
}

#[test]
fn test_weibull_and_gompertz_competing_hazards_parity() {
    let causes = vec![
        CompetingCause {
            cause_id: CauseId(1),
            priority: 0,
            hazard: HazardRate::Weibull {
                shape: 2.0, // aging / increasing hazard: H(t) = (t / scale)^2
                scale: 10.0,
            },
        },
        CompetingCause {
            cause_id: CauseId(2),
            priority: 0,
            hazard: HazardRate::Gompertz { eta: 0.05, b: 0.1 },
        },
    ];

    let engine = CompetingHazards::new(causes, TieBreaker::LowestCauseId);
    let mut stream = DeterministicStream::from_run_seed(2004);

    let n = 20_000;
    let mut cause1_wins = 0usize;
    let mut cause2_wins = 0usize;

    for _ in 0..n {
        let ev = engine
            .sample_next(0.0, None, &mut stream)
            .expect("must sample event");
        assert!(!ev.is_censored);
        assert!(ev.delta_time > 0.0);
        match ev.winning_cause.0 {
            1 => cause1_wins += 1,
            2 => cause2_wins += 1,
            _ => unreachable!(),
        }
    }

    assert!(cause1_wins > 0);
    assert!(cause2_wins > 0);
    assert_eq!(cause1_wins + cause2_wins, n);
}
