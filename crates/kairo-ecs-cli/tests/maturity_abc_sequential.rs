use kairo_ecs_cli::ensemble::{AbcPopulation, AbcSmcCalibrator, AbcSmcConfig, AbcSmcError};
use kairo_ecs_rng::rand_distr::StandardUniform;
use kairo_ecs_rng::DeterministicStream;

fn config(particles: usize) -> AbcSmcConfig {
    AbcSmcConfig {
        particles,
        tolerances: vec![2.0, 1.0, 0.0],
        kernel_scale: 0.2,
        max_attempts: particles * 30,
        min_ess_fraction: 0.4,
    }
}

fn successes(p: f64, rng: &mut DeterministicStream) -> u32 {
    (0..2)
        .map(|_| u32::from(rng.sample::<f64, _>(StandardUniform) < p))
        .sum()
}

fn expectation(population: &AbcPopulation, f: impl Fn(f64) -> f64) -> f64 {
    population
        .particles
        .iter()
        .map(|x| x.weight * f(x.parameters[0]))
        .sum()
}

#[test]
fn tightened_tolerances_converge_to_the_beta_posterior_and_predictive_rate() {
    let exact = [0.5, 0.625, 0.75];
    let mut mean_errors = [0.0_f64; 3];
    let (mut predictive_error, mut small_sample_squared_error, mut large_sample_squared_error) =
        (0.0, 0.0, 0.0);
    let trials = 12;
    for seed in 1100..1100 + trials {
        let discrepancy =
            |p: &[f64], rng: &mut DeterministicStream| f64::from(2 - successes(p[0], rng));
        let small = AbcSmcCalibrator::new(config(48), vec![(0., 1.)])
            .unwrap()
            .run(seed, discrepancy)
            .unwrap();
        let large = AbcSmcCalibrator::new(config(384), vec![(0., 1.)])
            .unwrap()
            .run(seed, discrepancy)
            .unwrap();
        for (stage, population) in large.iter().enumerate() {
            assert_eq!(population.tolerance, 2.0 - stage as f64);
            assert_eq!(population.particles.len(), 384);
            assert!(population.attempts >= population.particles.len());
            assert!((0.4 * 384.0..=384.0).contains(&population.effective_sample_size));
            let total_weight: f64 = population.particles.iter().map(|x| x.weight).sum();
            assert!((total_weight - 1.0).abs() < 1e-12);
            assert!(population
                .particles
                .iter()
                .all(|x| x.discrepancy <= population.tolerance));
            mean_errors[stage] += expectation(population, |p| p) - exact[stage];
        }
        small_sample_squared_error += (expectation(&small[2], |p| p) - 0.75).powi(2);
        large_sample_squared_error += (expectation(&large[2], |p| p) - 0.75).powi(2);
        predictive_error += expectation(&large[2], |p| p * p) - 0.6;
    }
    let biases = mean_errors.map(|error| error / trials as f64);
    let predictive_bias = predictive_error / trials as f64;
    eprintln!("ABC posterior mean biases {biases:?}, predictive bias {predictive_bias:.5}");
    assert!(biases.iter().all(|bias| bias.abs() <= 0.02));
    assert!(predictive_bias.abs() <= 0.025);
    assert!(large_sample_squared_error < small_sample_squared_error);
}

#[test]
fn later_weights_match_the_prior_over_the_previous_gaussian_mixture() {
    // A non-unit, two-dimensional prior catches scaling by absolute units
    // and accidental one-dimensional density evaluation.
    let calibrator = AbcSmcCalibrator::new(config(36), vec![(-3.0, 5.0), (100.0, 110.0)]).unwrap();
    let simulate = |p: &[f64], _: &mut DeterministicStream| {
        if p[0] >= 0.0 && p[1] >= 102.0 {
            0.0
        } else {
            1.0
        }
    };
    let populations = calibrator.run(723, simulate).unwrap();
    assert_eq!(populations, calibrator.run(723, simulate).unwrap());
    assert!(populations[0]
        .particles
        .iter()
        .all(|x| (x.weight - 1.0 / 36.0).abs() < 1e-15));
    for pair in populations.windows(2) {
        let [previous, current] = pair else {
            unreachable!()
        };
        let raw_weights: Vec<_> = current
            .particles
            .iter()
            .map(|x| {
                let pdf = |dx: f64, sd: f64| {
                    (-dx.powi(2) / (2.0 * sd * sd)).exp()
                        / (sd * (2.0 * std::f64::consts::PI).sqrt())
                };
                let mixture: f64 = previous
                    .particles
                    .iter()
                    .map(|a| {
                        a.weight
                            * pdf(x.parameters[0] - a.parameters[0], 1.6)
                            * pdf(x.parameters[1] - a.parameters[1], 2.0)
                    })
                    .sum();
                // Uniform density of the 8-by-10 prior rectangle.
                0.0125 / mixture
            })
            .collect();
        let sum: f64 = raw_weights.iter().sum();
        assert!(current
            .particles
            .iter()
            .any(|x| (x.weight - 1.0 / 36.0).abs() > 0.001));
        for (raw, x) in raw_weights.iter().zip(&current.particles) {
            assert!((x.weight - raw / sum).abs() < 1e-12);
        }
    }
}

#[test]
fn simulation_based_ranks_match_the_analytic_posterior_cdfs() {
    let mut truth_rng = DeterministicStream::from_run_seed(650091);
    let calibrator = AbcSmcCalibrator::new(config(192), vec![(0.0, 1.0)]).unwrap();
    let mut squared_error = 0.0;
    let mut quartiles = [0; 4];
    let mut observations = [0; 3];
    for seed in 7000..7064 {
        let p: f64 = truth_rng.sample(StandardUniform);
        let observed = successes(p, &mut truth_rng);
        observations[observed as usize] += 1;
        let fit = calibrator
            .run(seed, |theta, rng| {
                f64::from(successes(theta[0], rng).abs_diff(observed))
            })
            .unwrap();
        let rank = expectation(&fit[2], |theta| f64::from(theta <= p));
        let reference = match observed {
            0 => 1.0 - (1.0 - p).powi(3),
            1 => 3.0 * p.powi(2) - 2.0 * p.powi(3),
            2 => p.powi(3),
            _ => unreachable!(),
        };
        squared_error += (rank - reference).powi(2);
        quartiles[((rank * 4.0) as usize).min(3)] += 1;
    }
    let rmse = (squared_error / 64.0).sqrt();
    eprintln!("SBC rank quartiles {quartiles:?}, observed success counts {observations:?}, CDF RMSE {rmse:.5}");
    assert!(observations.iter().all(|count| *count >= 12));
    assert!(quartiles.iter().all(|count| (7..=25).contains(count)));
    assert!(rmse <= 0.065);
}

#[test]
fn configuration_attempt_budget_invalid_discrepancy_and_ess_fail_explicitly() {
    let mut cfg = config(16);
    cfg.tolerances = vec![1.0, 1.0];
    assert!(matches!(
        AbcSmcCalibrator::new(cfg, vec![(0., 1.)]),
        Err(AbcSmcError::Configuration(_))
    ));
    let mut cfg = config(16);
    cfg.max_attempts = 16;
    let abc = AbcSmcCalibrator::new(cfg, vec![(0., 1.)]).unwrap();
    assert_eq!(
        abc.run(5, |_, _| 3.0),
        Err(AbcSmcError::ProposalBudget {
            generation: 0,
            attempts: 16,
            accepted: 0
        })
    );
    assert_eq!(
        abc.run(5, |_, _| f64::NAN),
        Err(AbcSmcError::InvalidDiscrepancy {
            generation: 0,
            attempt: 1
        })
    );

    let mut cfg = config(16);
    cfg.min_ess_fraction = 1.0;
    let abc = AbcSmcCalibrator::new(cfg, vec![(0., 1.)]).unwrap();
    assert!(matches!(
        abc.run(6, |_, _| 0.0),
        Err(AbcSmcError::LowEffectiveSampleSize { generation: 1, .. })
    ));
}
