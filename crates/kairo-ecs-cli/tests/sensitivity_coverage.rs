use kairo_ecs_cli::ensemble::{SensitivityError, SobolSensitivityAnalyzer};
use std::f64::consts::PI;

const TRIALS: usize = 40;
const PAIR_COUNT: usize = 1024;

#[derive(Default)]
struct ResultSummary {
    covered: usize,
    bias_sum: f64,
    squared_errors: f64,
}

fn check_coverage(model: impl Fn(&[f64]) -> f64, truth: [[f64; 2]; 3]) {
    let mut summary: [[ResultSummary; 2]; 3] = Default::default();
    for seed in 9000..9000 + TRIALS as u64 {
        let result = SobolSensitivityAnalyzer::analyze_seeded(&model, 3, PAIR_COUNT, seed).unwrap();
        for dimension in 0..truth.len() {
            let intervals = [
                (
                    result.first_order[dimension],
                    result.ci_lower_first[dimension],
                    result.ci_upper_first[dimension],
                ),
                (
                    result.total_effect[dimension],
                    result.ci_lower_total[dimension],
                    result.ci_upper_total[dimension],
                ),
            ];
            for (effect, (estimate, lower, upper)) in intervals.into_iter().enumerate() {
                let exact = truth[dimension][effect];
                assert!(lower < upper);
                let entry = &mut summary[dimension][effect];
                entry.covered += usize::from((lower..=upper).contains(&exact));
                entry.bias_sum += estimate - exact;
                entry.squared_errors += (estimate - exact).powi(2);
            }
        }
    }

    for (dimension, effects) in summary.iter().enumerate() {
        for (effect, entry) in effects.iter().enumerate() {
            let bias = entry.bias_sum / TRIALS as f64;
            let rmse = (entry.squared_errors / TRIALS as f64).sqrt();
            eprintln!("dimension {dimension}, effect {effect}: coverage {}/{TRIALS}, bias {bias:.5}, RMSE {rmse:.5}", entry.covered);
            assert!(entry.covered >= 33);
            assert!(bias.abs() <= 0.025);
            assert!(rmse <= 0.085);
        }
    }
}

#[test]
fn ishigami_first_and_total_effects_cover_analytic_variance_components() {
    let (a, b): (f64, f64) = (7.0, 0.1);
    let model = |x: &[f64]| {
        let x1 = PI * (2.0 * x[0] - 1.0);
        let x2 = PI * (2.0 * x[1] - 1.0);
        let x3 = PI * (2.0 * x[2] - 1.0);
        (1.0 + b * x3.powi(4)) * x1.sin() + a * x2.sin().powi(2)
    };
    // Integrate the trigonometric moments and the fourth/eighth powers
    // over independent Uniform[-pi, pi] inputs.
    let v1 = (1.0 + b * PI.powi(4) / 5.0).powi(2) / 2.0;
    let v2 = a.powi(2) / 8.0;
    let v13 = 8.0 * b.powi(2) * PI.powi(8) / 225.0;
    let variance = v1 + v2 + v13;
    check_coverage(
        model,
        [
            [v1 / variance, (v1 + v13) / variance],
            [v2 / variance; 2],
            [0.0, v13 / variance],
        ],
    );
}

#[test]
fn sobol_g_first_and_total_effects_cover_product_variance() {
    let a: [f64; 3] = [0.0, 1.0, 4.0];
    let model = |x: &[f64]| {
        x.iter()
            .zip(a)
            .map(|(x, a)| ((4.0 * x - 2.0).abs() + a) / (1.0 + a))
            .product()
    };
    // Each independent factor has mean 1 and variance 1/(3(1+a)^2).
    let factors = a.map(|a| 1.0 / (3.0 * (1.0 + a).powi(2)));
    let second_moment: f64 = factors.iter().map(|v| 1.0 + v).product();
    let variance = second_moment - 1.0;
    check_coverage(
        model,
        factors.map(|v| [v / variance, v * second_moment / (1.0 + v) / variance]),
    );
}

#[test]
fn undefined_and_nonfinite_models_cannot_produce_intervals() {
    assert!(matches!(
        SobolSensitivityAnalyzer::analyze(&|_| 1.0, 3, 16),
        Err(SensitivityError::UndefinedVariance)
    ));
    assert!(matches!(
        SobolSensitivityAnalyzer::analyze(&|_| f64::INFINITY, 3, 16),
        Err(SensitivityError::NonFiniteOutput)
    ));
    assert!(matches!(
        SobolSensitivityAnalyzer::analyze_parallel(&|_| 1.0, 0, 0, 3),
        Err(SensitivityError::InvalidDesign)
    ));
}
