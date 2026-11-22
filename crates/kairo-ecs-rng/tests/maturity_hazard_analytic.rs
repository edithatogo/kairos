use kairo_ecs_rng::{
    CauseId, CompetingCause, CompetingHazards, DeterministicStream, HazardRate, TieBreaker,
};

fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() <= 1e-12, "{actual} vs {expected}");
}

#[test]
fn analytic_inverse_hazards_check_initial_age_and_piecewise_boundaries() {
    let exponential = HazardRate::Constant { rate: 0.8 };
    close(exponential.cumulative_hazard(2.0), 1.6);
    close(exponential.invert_cumulative_hazard(4.0, 2.0).unwrap(), 2.5);

    let weibull = HazardRate::Weibull {
        shape: 2.0,
        scale: 2.0,
    };
    close(weibull.cumulative_hazard(4.0), 4.0);
    // For H(t)=t²/4, H(4)-H(3)=7/4 and H(3)-H(0)=9/4.
    close(weibull.invert_cumulative_hazard(3.0, 1.75).unwrap(), 1.0);
    close(weibull.invert_cumulative_hazard(0.0, 2.25).unwrap(), 3.0);

    let piecewise = HazardRate::PiecewiseConstant {
        intervals: vec![(2.0, 0.5), (4.0, 0.0), (5.0, 2.0)],
        trailing_rate: 1.0,
    };
    // From age 1, the next half-unit is exhausted at age 2.
    // There is no hazard until age 4, then two units until age 5.
    for (budget, elapsed) in [(0.25, 0.5), (0.5, 1.0), (1.5, 3.5), (2.5, 4.0), (3.5, 5.0)] {
        close(
            piecewise.invert_cumulative_hazard(1.0, budget).unwrap(),
            elapsed,
        );
    }
    close(piecewise.invert_cumulative_hazard(4.0, 1.0).unwrap(), 0.5);
    close(piecewise.cumulative_hazard(6.0), 4.0);
    assert_eq!(HazardRate::Zero.invert_cumulative_hazard(0.0, 1.0), None);
}

fn verify_incidence(hazards: [HazardRate; 2], survival: impl Fn(f64, f64) -> f64) {
    let age = 0.25;
    let per_seed = 4096;
    let n = f64::from(per_seed * 4);
    let study = CompetingHazards::new(
        hazards
            .into_iter()
            .enumerate()
            .map(|(i, hazard)| CompetingCause {
                cause_id: CauseId(i as u32),
                priority: 0,
                hazard,
            })
            .collect(),
        TieBreaker::LowestCauseId,
    );
    // Delayed entry conditions on survival to age 0.25.
    for horizon in [0.5, 1.0, 2.0] {
        let mut observed = [0; 3];
        for seed in [4021, 5193, 8029, 9017] {
            let mut stream = DeterministicStream::from_run_seed(seed);
            for _ in 0..per_seed {
                let event = study.sample_next(age, Some(horizon), &mut stream).unwrap();
                assert!((0.0..=horizon - age).contains(&event.delta_time));
                let class = if event.is_censored {
                    assert_eq!(event.delta_time, horizon - age);
                    2
                } else {
                    event.winning_cause.0 as usize
                };
                observed[class] += 1;
            }
        }
        let s = survival(age, horizon);
        let expected = [(1.0 - s) / 3.0, 2.0 * (1.0 - s) / 3.0, s];
        for (class, (count, probability)) in observed.into_iter().zip(expected).enumerate() {
            let estimate = f64::from(count) / n;
            let limit = 4.0 * (probability * (1.0 - probability) / n).sqrt() + n.recip();
            eprintln!(
                "age {horizon}, class {class}, expected {probability:.5}, observed {estimate:.5}"
            );
            assert!((estimate - probability).abs() <= limit);
        }
    }
}

#[test]
fn exponential_cause_incidence_and_survival_match_the_closed_form() {
    verify_incidence(
        [
            HazardRate::Constant { rate: 0.4 },
            HazardRate::Constant { rate: 0.8 },
        ],
        |start, t| (-1.2 * (t - start)).exp(),
    );
}

#[test]
fn age_dependent_weibull_cause_incidence_and_survival_match_the_closed_form() {
    // H1(t)=t²/4 and H2(t)=t²/2, so the proportional cause hazards
    // integrate against joint survival in a 1:2 split.
    verify_incidence(
        [
            HazardRate::Weibull {
                shape: 2.0,
                scale: 2.0,
            },
            HazardRate::Weibull {
                shape: 2.0,
                scale: 2.0_f64.sqrt(),
            },
        ],
        |start, t| (-0.75 * (t * t - start * start)).exp(),
    );
}
