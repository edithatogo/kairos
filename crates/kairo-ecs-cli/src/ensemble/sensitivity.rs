use kairo_ecs_rng::rand_distr::{StandardUniform, Uniform};
use kairo_ecs_rng::DeterministicStream;
use rayon::prelude::*;

use super::build_bounded_thread_pool;

const DEFAULT_SEED: u64 = 42;
const BOOTSTRAP_REPLICATES: usize = 199;

/// Failure to estimate variance-based indices or their paired bootstrap intervals.
#[derive(Debug, thiserror::Error)]
pub enum SensitivityError {
    #[error("sensitivity analysis requires at least one dimension and two paired rows")]
    InvalidDesign,
    #[error("a model evaluation returned a non-finite value")]
    NonFiniteOutput,
    #[error("pooled variance is zero or unrepresentable in the data or a bootstrap sample")]
    UndefinedVariance,
    #[error("failed to build the sensitivity thread pool: {0}")]
    ThreadPool(#[from] rayon::ThreadPoolBuildError),
}

/// Jansen sensitivity indices and marginal 95% paired-row percentile intervals.
///
/// The design draws independent Uniform[0,1) input coordinates. Transform
/// these coordinates inside a deterministic model for other independent
/// marginals. Each row comprises A, B and their single-coordinate hybrids.
/// Bootstrap resampling preserves those groups. Estimates are unbounded
/// finite-sample statistics, so negative first-order values are possible.
#[derive(Clone, Debug, PartialEq)]
pub struct SobolSensitivityAnalyzer {
    pub first_order: Vec<f64>,
    pub total_effect: Vec<f64>,
    pub ci_lower_first: Vec<f64>,
    pub ci_upper_first: Vec<f64>,
    pub ci_lower_total: Vec<f64>,
    pub ci_upper_total: Vec<f64>,
}

impl SobolSensitivityAnalyzer {
    pub const VERSION: &'static str = "jansen-iid-paired-percentile-v1";

    pub fn analyze<F>(
        model: &F,
        dimensions: usize,
        sample_size: usize,
    ) -> Result<Self, SensitivityError>
    where
        F: Fn(&[f64]) -> f64 + ?Sized,
    {
        Self::analyze_seeded(model, dimensions, sample_size, DEFAULT_SEED)
    }

    pub fn analyze_seeded<F>(
        model: &F,
        dimensions: usize,
        sample_size: usize,
        seed: u64,
    ) -> Result<Self, SensitivityError>
    where
        F: Fn(&[f64]) -> f64 + ?Sized,
    {
        let pairs = sample_pairs(dimensions, sample_size, seed)?;
        let outcomes = pairs.iter().map(|(a, b)| evaluate(model, a, b)).collect();
        summarize(outcomes, dimensions, seed)
    }

    pub fn analyze_parallel<F>(
        model: &F,
        dimensions: usize,
        sample_size: usize,
        num_threads: usize,
    ) -> Result<Self, SensitivityError>
    where
        F: Fn(&[f64]) -> f64 + Sync,
    {
        let pool = build_bounded_thread_pool(num_threads.min(sample_size))?;
        Self::analyze_parallel_with_pool(model, dimensions, sample_size, &pool)
    }

    pub fn analyze_parallel_with_pool<F>(
        model: &F,
        dimensions: usize,
        sample_size: usize,
        pool: &rayon::ThreadPool,
    ) -> Result<Self, SensitivityError>
    where
        F: Fn(&[f64]) -> f64 + Sync,
    {
        let pairs = sample_pairs(dimensions, sample_size, DEFAULT_SEED)?;
        let outcomes = pool.install(|| {
            pairs
                .par_iter()
                .map(|(a, b)| evaluate(model, a, b))
                .collect()
        });
        summarize(outcomes, dimensions, DEFAULT_SEED)
    }
}

type InputPair = (Vec<f64>, Vec<f64>);

fn sample_pairs(
    dimensions: usize,
    sample_size: usize,
    seed: u64,
) -> Result<Vec<InputPair>, SensitivityError> {
    if dimensions == 0 || sample_size < 2 {
        return Err(SensitivityError::InvalidDesign);
    }
    let mut rng = DeterministicStream::from_seed_u64(seed, 0);
    let mut sample_row = || {
        (0..dimensions)
            .map(|_| rng.sample(StandardUniform))
            .collect()
    };
    Ok((0..sample_size)
        .map(|_| (sample_row(), sample_row()))
        .collect())
}

struct Outcomes {
    a: f64,
    b: f64,
    hybrids: Vec<f64>,
}

fn evaluate(model: &(impl Fn(&[f64]) -> f64 + ?Sized), a: &[f64], b: &[f64]) -> Outcomes {
    let mut hybrid = a.to_vec();
    let hybrids = (0..a.len())
        .map(|d| {
            hybrid[d] = b[d];
            let y = model(&hybrid);
            hybrid[d] = a[d];
            y
        })
        .collect();
    Outcomes {
        a: model(a),
        b: model(b),
        hybrids,
    }
}

struct Row {
    mean: f64,
    sum_squared_deviations: f64,
    differences: Vec<[f64; 2]>,
}

fn summarize(
    outcomes: Vec<Outcomes>,
    dimensions: usize,
    seed: u64,
) -> Result<SobolSensitivityAnalyzer, SensitivityError> {
    let mut scale: f64 = 0.0;
    for row in &outcomes {
        for y in [row.a, row.b]
            .into_iter()
            .chain(row.hybrids.iter().copied())
        {
            if !y.is_finite() {
                return Err(SensitivityError::NonFiniteOutput);
            }
            scale = scale.max(y.abs());
        }
    }
    if scale == 0.0 {
        return Err(SensitivityError::UndefinedVariance);
    }

    // Scale and center before aggregating the pooled moments so finite model
    // values cannot overflow a sum of squares.
    let center = outcomes[0].a / scale;
    let centered = |y: f64| y / scale - center;
    let rows: Vec<_> = outcomes
        .into_iter()
        .map(|row| {
            let a = centered(row.a);
            let b = centered(row.b);
            Row {
                mean: (a + b) / 2.0,
                sum_squared_deviations: (a - b).powi(2) / 2.0,
                differences: row
                    .hybrids
                    .into_iter()
                    .map(|y| {
                        let y = centered(y);
                        [(b - y).powi(2), (a - y).powi(2)]
                    })
                    .collect(),
            }
        })
        .collect();

    let n = rows.len();
    let point = estimate(dimensions, rows.iter())?;
    let mut rng = DeterministicStream::from_seed_u64(seed, 1);
    let draw_index = Uniform::new(0, n).map_err(|_| SensitivityError::InvalidDesign)?;
    let mut bootstrap = vec![[Vec::new(), Vec::new()]; dimensions];
    for _ in 0..BOOTSTRAP_REPLICATES {
        let picked = (0..n).map(|_| &rows[rng.sample(draw_index)]);
        for (dimension, [first, total]) in bootstrap.iter_mut().zip(estimate(dimensions, picked)?) {
            dimension[0].push(first);
            dimension[1].push(total);
        }
    }

    let mut result = SobolSensitivityAnalyzer {
        first_order: Vec::with_capacity(dimensions),
        total_effect: Vec::with_capacity(dimensions),
        ci_lower_first: Vec::with_capacity(dimensions),
        ci_upper_first: Vec::with_capacity(dimensions),
        ci_lower_total: Vec::with_capacity(dimensions),
        ci_upper_total: Vec::with_capacity(dimensions),
    };
    for ([first, total], [mut bootstrap_first, mut bootstrap_total]) in
        point.into_iter().zip(bootstrap)
    {
        result.first_order.push(first);
        result.total_effect.push(total);
        bootstrap_first.sort_unstable_by(f64::total_cmp);
        bootstrap_total.sort_unstable_by(f64::total_cmp);
        result
            .ci_lower_first
            .push(quantile(&bootstrap_first, 0.025));
        result
            .ci_upper_first
            .push(quantile(&bootstrap_first, 0.975));
        result
            .ci_lower_total
            .push(quantile(&bootstrap_total, 0.025));
        result
            .ci_upper_total
            .push(quantile(&bootstrap_total, 0.975));
    }
    Ok(result)
}

fn estimate<'a>(
    dimensions: usize,
    rows: impl Iterator<Item = &'a Row>,
) -> Result<Vec<[f64; 2]>, SensitivityError> {
    let mut mean = 0.0;
    let mut sum_squared_deviations = 0.0;
    let mut effects = vec![[0.0, 0.0]; dimensions];
    for (preceding, row) in rows.enumerate() {
        // Merge the moments of each A/B pair with the preceding pairs.
        let previous_pairs = preceding as f64;
        let pairs = previous_pairs + 1.0;
        let delta = row.mean - mean;
        mean += delta / pairs;
        sum_squared_deviations +=
            row.sum_squared_deviations + 2.0 * delta * delta * (previous_pairs / pairs);
        for ([first, total], [b_difference, a_difference]) in
            effects.iter_mut().zip(&row.differences)
        {
            *first += b_difference;
            *total += a_difference;
        }
    }
    if sum_squared_deviations <= 0.0 {
        return Err(SensitivityError::UndefinedVariance);
    }
    // 2 * n * pooled empirical variance = pooled sum of squared deviations.
    for [first, total] in &mut effects {
        *first = 1.0 - *first / sum_squared_deviations;
        *total /= sum_squared_deviations;
        if !first.is_finite() || !total.is_finite() {
            return Err(SensitivityError::UndefinedVariance);
        }
    }
    Ok(effects)
}

fn quantile(sorted: &[f64], probability: f64) -> f64 {
    let index = probability * (sorted.len() - 1) as f64;
    let lower = index.floor() as usize;
    sorted[lower] + index.fract() * (sorted[lower + 1] - sorted[lower])
}
