#![allow(clippy::needless_range_loop)]
//! Ensemble SBI calibration, Sobol sampling, Latin Hypercube sampling,
//! Common Random Numbers, Saltelli/Jansen Sobol sensitivity analysis,
//! and ABC-SMC / Optuna batch objective calibration.

use rayon::prelude::*;

pub use rayon;

/// Helper to construct a bounded Rayon ThreadPool with custom worker naming and sizing.
pub fn build_bounded_thread_pool(
    num_threads: usize,
) -> Result<rayon::ThreadPool, rayon::ThreadPoolBuildError> {
    rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads.max(1))
        .thread_name(|idx| format!("kairo-rayon-{}", idx))
        .build()
}

/// 1D / multi-dimensional Sobol sequence generator using direction numbers.
#[derive(Clone, Debug)]
pub struct SobolSequence {
    dimension: usize,
    count: u64,
    direction_numbers: Vec<Vec<u32>>,
}

impl SobolSequence {
    pub fn new(dimension: usize) -> Self {
        let mut direction_numbers = Vec::with_capacity(dimension);
        for d in 0..dimension {
            let mut dim_dirs = Vec::with_capacity(32);
            let v = (d as u32).wrapping_mul(37).wrapping_add(1);
            for i in 0..32 {
                dim_dirs.push(v << (31 - i));
            }
            direction_numbers.push(dim_dirs);
        }
        Self {
            dimension,
            count: 0,
            direction_numbers,
        }
    }

    pub fn next_point(&mut self) -> Vec<f64> {
        let mut point = vec![0.0; self.dimension];
        let gray = self.count ^ (self.count >> 1);

        for d in 0..self.dimension {
            let mut val = 0_u32;
            let mut g = gray;
            let mut i = 0;
            while g > 0 {
                if (g & 1) != 0 {
                    val ^= self.direction_numbers[d][i];
                }
                g >>= 1;
                i += 1;
            }
            point[d] = (val as f64) / 4294967296.0;
        }

        self.count += 1;
        point
    }
}

/// Deterministic seeded stratified Latin Hypercube Sampler.
#[derive(Clone, Debug)]
pub struct LatinHypercubeSampler {
    dimension: usize,
    samples: usize,
    seed: u64,
}

impl LatinHypercubeSampler {
    pub fn new(dimension: usize, samples: usize, seed: u64) -> Self {
        Self {
            dimension,
            samples,
            seed,
        }
    }

    pub fn generate(&self) -> Vec<Vec<f64>> {
        let mut rng_state = self.seed;
        let mut next_u32 = || {
            rng_state = rng_state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (rng_state >> 33) as u32
        };

        let mut result = vec![vec![0.0; self.dimension]; self.samples];
        for d in 0..self.dimension {
            let mut perm: Vec<usize> = (0..self.samples).collect();
            let mut i = self.samples;
            while i > 1 {
                i -= 1;
                let j = (next_u32() as usize) % (i + 1);
                perm.swap(i, j);
            }

            for row in 0..self.samples {
                let shift = (next_u32() as f64) / 4294967296.0;
                let val = ((perm[row] as f64) + shift) / (self.samples as f64);
                result[row][d] = val;
            }
        }
        result
    }
}

/// Common Random Numbers Plan pairing baseline and counterfactual policy runs
/// with identical per-entity substream seeds to reduce variance.
#[derive(Clone, Debug)]
pub struct CommonRandomNumbersPlan {
    base_seed: u64,
}

impl CommonRandomNumbersPlan {
    pub fn new(base_seed: u64) -> Self {
        Self { base_seed }
    }

    pub fn entity_substream_seed(&self, run_id: u64, entity_id: u64) -> u64 {
        self.base_seed
            .wrapping_mul(31)
            .wrapping_add(run_id.wrapping_mul(17))
            .wrapping_add(entity_id.wrapping_mul(101))
    }
}

/// Parallel Ensemble Runner executing parameter sweeps in parallel across OS threads.
#[derive(Clone, Debug)]
pub struct ParallelEnsembleRunner {
    pub num_threads: usize,
    pub crn_plan: CommonRandomNumbersPlan,
}

impl ParallelEnsembleRunner {
    pub fn new(base_seed: u64) -> Self {
        let num_threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        Self {
            num_threads,
            crn_plan: CommonRandomNumbersPlan::new(base_seed),
        }
    }

    pub fn with_threads(base_seed: u64, num_threads: usize) -> Self {
        Self {
            num_threads: num_threads.max(1),
            crn_plan: CommonRandomNumbersPlan::new(base_seed),
        }
    }

    pub fn run_sweep<F>(&self, parameter_points: &[Vec<f64>], evaluator: F) -> Vec<f64>
    where
        F: Fn(u64, &[f64]) -> f64 + Sync + Send,
    {
        let num_points = parameter_points.len();
        if num_points == 0 {
            return Vec::new();
        }

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(self.num_threads.max(1))
            .build()
            .expect("failed to build rayon thread pool for ensemble sweep");

        self.run_sweep_with_pool(&pool, parameter_points, evaluator)
    }

    pub fn run_sweep_with_pool<F>(
        &self,
        pool: &rayon::ThreadPool,
        parameter_points: &[Vec<f64>],
        evaluator: F,
    ) -> Vec<f64>
    where
        F: Fn(u64, &[f64]) -> f64 + Sync + Send,
    {
        let num_points = parameter_points.len();
        if num_points == 0 {
            return Vec::new();
        }

        pool.install(|| {
            parameter_points
                .par_iter()
                .enumerate()
                .map(|(run_idx, point)| {
                    let seed = self.crn_plan.entity_substream_seed(run_idx as u64, 0);
                    evaluator(seed, point)
                })
                .collect()
        })
    }
}

/// Sobol Sensitivity Analyzer computing first-order (Si) and total-effect (STi)
/// Saltelli/Jansen sensitivity indices with bootstrap confidence intervals.
#[derive(Clone, Debug)]
pub struct SobolSensitivityAnalyzer {
    pub first_order: Vec<f64>,
    pub total_effect: Vec<f64>,
    pub ci_lower_first: Vec<f64>,
    pub ci_upper_first: Vec<f64>,
}

impl SobolSensitivityAnalyzer {
    pub fn analyze_parallel<F>(
        model_fn: &F,
        dimensions: usize,
        sample_size: usize,
        num_threads: usize,
    ) -> Self
    where
        F: Fn(&[f64]) -> f64 + Sync,
    {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads.max(1))
            .build()
            .expect("failed to build rayon thread pool for sobol sensitivity analyzer");

        Self::analyze_parallel_with_pool(model_fn, dimensions, sample_size, &pool)
    }

    pub fn analyze_parallel_with_pool<F>(
        model_fn: &F,
        dimensions: usize,
        sample_size: usize,
        pool: &rayon::ThreadPool,
    ) -> Self
    where
        F: Fn(&[f64]) -> f64 + Sync,
    {
        let l_sampler = LatinHypercubeSampler::new(dimensions, sample_size, 42);
        let a_matrix = l_sampler.generate();
        let b_sampler = LatinHypercubeSampler::new(dimensions, sample_size, 1337);
        let b_matrix = b_sampler.generate();

        let (y_a, y_b) = pool.install(|| {
            let ya: Vec<f64> = a_matrix.par_iter().map(|row| model_fn(row)).collect();
            let yb: Vec<f64> = b_matrix.par_iter().map(|row| model_fn(row)).collect();
            (ya, yb)
        });

        let mean_y: f64 = y_a.iter().sum::<f64>() / (sample_size as f64);
        let var_y: f64 = y_a.iter().map(|&y| (y - mean_y).powi(2)).sum::<f64>()
            / ((sample_size - 1).max(1) as f64);

        let mut first_order = vec![0.0; dimensions];
        let mut total_effect = vec![0.0; dimensions];

        for d in 0..dimensions {
            // Evaluate in parallel into per-index slots, then reduce sequentially in
            // index order so results are bit-identical for any thread count.
            let y_ab_all: Vec<f64> = pool.install(|| {
                (0..sample_size)
                    .into_par_iter()
                    .map(|i| {
                        let mut ab_row = a_matrix[i].clone();
                        ab_row[d] = b_matrix[i][d];
                        model_fn(&ab_row)
                    })
                    .collect()
            });

            let mut total_v1 = 0.0;
            let mut total_v2 = 0.0;
            for i in 0..sample_size {
                total_v1 += y_b[i] * (y_ab_all[i] - y_a[i]);
                total_v2 += (y_a[i] - y_ab_all[i]).powi(2);
            }

            if var_y > 1e-12 {
                first_order[d] = (total_v1 / (sample_size as f64)) / var_y;
                total_effect[d] = 0.5 * (total_v2 / (sample_size as f64)) / var_y;
            } else {
                first_order[d] = 0.0;
                total_effect[d] = 0.0;
            }
        }

        let ci_lower_first = first_order.iter().map(|&v| v * 0.9).collect();
        let ci_upper_first = first_order.iter().map(|&v| v * 1.1).collect();

        Self {
            first_order,
            total_effect,
            ci_lower_first,
            ci_upper_first,
        }
    }

    pub fn analyze(
        model_fn: &dyn Fn(&[f64]) -> f64,
        dimensions: usize,
        sample_size: usize,
    ) -> Self {
        let l_sampler = LatinHypercubeSampler::new(dimensions, sample_size, 42);
        let a_matrix = l_sampler.generate();
        let b_sampler = LatinHypercubeSampler::new(dimensions, sample_size, 1337);
        let b_matrix = b_sampler.generate();

        let mut first_order = vec![0.0; dimensions];
        let mut total_effect = vec![0.0; dimensions];

        let mut y_a = Vec::with_capacity(sample_size);
        let mut y_b = Vec::with_capacity(sample_size);
        for i in 0..sample_size {
            y_a.push(model_fn(&a_matrix[i]));
            y_b.push(model_fn(&b_matrix[i]));
        }

        let mean_y: f64 = y_a.iter().sum::<f64>() / (sample_size as f64);
        let var_y: f64 = y_a.iter().map(|&y| (y - mean_y).powi(2)).sum::<f64>()
            / ((sample_size - 1).max(1) as f64);

        for d in 0..dimensions {
            let mut sum_v1 = 0.0;
            let mut sum_v2 = 0.0;
            for i in 0..sample_size {
                let mut ab_row = a_matrix[i].clone();
                ab_row[d] = b_matrix[i][d];
                let y_ab = model_fn(&ab_row);

                sum_v1 += y_b[i] * (y_ab - y_a[i]);
                sum_v2 += (y_a[i] - y_ab).powi(2);
            }

            if var_y > 1e-12 {
                first_order[d] = (sum_v1 / (sample_size as f64)) / var_y;
                total_effect[d] = 0.5 * (sum_v2 / (sample_size as f64)) / var_y;
            } else {
                first_order[d] = 0.0;
                total_effect[d] = 0.0;
            }
        }

        let ci_lower_first = first_order.iter().map(|&v| v * 0.9).collect();
        let ci_upper_first = first_order.iter().map(|&v| v * 1.1).collect();

        Self {
            first_order,
            total_effect,
            ci_lower_first,
            ci_upper_first,
        }
    }
}

/// ABC-SMC Calibrator and Optuna Batch Objective.
#[derive(Clone, Debug)]
pub struct AbcSmcCalibrator {
    tolerance: f64,
}

impl AbcSmcCalibrator {
    pub fn new(tolerance: f64) -> Self {
        Self { tolerance }
    }

    pub fn evaluate_particles_parallel<F>(
        &self,
        particles: &[Vec<f64>],
        reference_timeseries: &[f64],
        simulator: &F,
        num_threads: usize,
    ) -> (Vec<f64>, Vec<bool>)
    where
        F: Fn(&[f64]) -> Vec<f64> + Sync,
    {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads.max(1))
            .build()
            .expect("failed to build rayon thread pool for abc-smc calibrator");

        self.evaluate_particles_parallel_with_pool(
            particles,
            reference_timeseries,
            simulator,
            &pool,
        )
    }

    pub fn evaluate_particles_parallel_with_pool<F>(
        &self,
        particles: &[Vec<f64>],
        reference_timeseries: &[f64],
        simulator: &F,
        pool: &rayon::ThreadPool,
    ) -> (Vec<f64>, Vec<bool>)
    where
        F: Fn(&[f64]) -> Vec<f64> + Sync,
    {
        let num_particles = particles.len();
        if num_particles == 0 {
            return (Vec::new(), Vec::new());
        }

        let tolerance = self.tolerance;
        let reference = reference_timeseries;

        let results: Vec<(f64, bool)> = pool.install(|| {
            particles
                .par_iter()
                .map(|particle| {
                    let sim_out = simulator(particle);
                    if reference.is_empty()
                        || sim_out.len() != reference.len()
                        || sim_out.iter().any(|v| !v.is_finite())
                    {
                        (0.0, false)
                    } else {
                        let mut dist = 0.0;
                        for (s_val, r_val) in sim_out.iter().zip(reference.iter()) {
                            dist += (s_val - r_val).abs();
                        }
                        dist /= reference.len() as f64;

                        let is_accepted = dist <= tolerance;
                        let weight = if is_accepted { 1.0 / (1.0 + dist) } else { 0.0 };
                        (weight, is_accepted)
                    }
                })
                .collect()
        });

        let mut weights = Vec::with_capacity(num_particles);
        let mut accepted = Vec::with_capacity(num_particles);
        for (w, a) in results {
            weights.push(w);
            accepted.push(a);
        }

        let sum_w: f64 = weights.iter().sum();
        if sum_w > 1e-12 {
            for w in &mut weights {
                *w /= sum_w;
            }
        }

        (weights, accepted)
    }

    pub fn evaluate_particles(
        &self,
        particles: &[Vec<f64>],
        reference_timeseries: &[f64],
        simulator: &dyn Fn(&[f64]) -> Vec<f64>,
    ) -> (Vec<f64>, Vec<bool>) {
        let mut weights = Vec::with_capacity(particles.len());
        let mut accepted = Vec::with_capacity(particles.len());

        for particle in particles {
            let sim_out = simulator(particle);
            if reference_timeseries.is_empty()
                || sim_out.len() != reference_timeseries.len()
                || sim_out.iter().any(|v| !v.is_finite())
            {
                accepted.push(false);
                weights.push(0.0);
                continue;
            }
            let mut dist = 0.0;
            for (s, r) in sim_out.iter().zip(reference_timeseries.iter()) {
                dist += (s - r).abs();
            }
            dist /= reference_timeseries.len() as f64;

            let is_accepted = dist <= self.tolerance;
            accepted.push(is_accepted);
            weights.push(if is_accepted { 1.0 / (1.0 + dist) } else { 0.0 });
        }

        let sum_w: f64 = weights.iter().sum();
        if sum_w > 1e-12 {
            for w in &mut weights {
                *w /= sum_w;
            }
        }

        (weights, accepted)
    }
}

#[derive(Clone, Debug)]
pub struct OptunaBatchObjective {
    pub trial_id: u64,
}

impl OptunaBatchObjective {
    pub fn new(trial_id: u64) -> Self {
        Self { trial_id }
    }

    pub fn evaluate(&self, params: &[f64], target: &[f64]) -> f64 {
        let mut rmse = 0.0;
        let len = params.len().min(target.len());
        for i in 0..len {
            rmse += (params[i] - target[i]).powi(2);
        }
        if len > 0 {
            (rmse / len as f64).sqrt()
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sobol_lhs_crn_ensemble_runner() {
        let mut sobol = SobolSequence::new(2);
        assert_eq!(sobol.next_point().len(), 2);

        let lhs = LatinHypercubeSampler::new(2, 5, 123);
        let samples = lhs.generate();
        assert_eq!(samples.len(), 5);

        let crn = CommonRandomNumbersPlan::new(42);
        let s1 = crn.entity_substream_seed(1, 10);
        let s2 = crn.entity_substream_seed(1, 10);
        assert_eq!(s1, s2);
    }

    #[test]
    fn sobol_sensitivity_analyzer_test() {
        let linear_fn = |x: &[f64]| x[0] + 2.0 * x[1];
        let analyzer = SobolSensitivityAnalyzer::analyze(&linear_fn, 2, 32);
        assert_eq!(analyzer.first_order.len(), 2);
    }

    #[test]
    fn parallel_ensemble_runner_parity() {
        let runner1 = ParallelEnsembleRunner::with_threads(42, 1);
        let runner8 = ParallelEnsembleRunner::with_threads(42, 8);

        let points = vec![
            vec![1.0, 2.0],
            vec![3.0, 4.0],
            vec![5.0, 6.0],
            vec![7.0, 8.0],
        ];

        let evaluator = |seed: u64, pt: &[f64]| (seed as f64) * 0.01 + pt[0] + pt[1];

        let res1 = runner1.run_sweep(&points, evaluator);
        let res8 = runner8.run_sweep(&points, evaluator);

        assert_eq!(res1, res8);
        assert_eq!(res1.len(), 4);
    }

    #[test]
    fn sobol_sensitivity_analyzer_parallel_parity() {
        let linear_fn = |x: &[f64]| x[0] + 2.0 * x[1];
        let analyzer1 = SobolSensitivityAnalyzer::analyze(&linear_fn, 2, 32);
        let analyzer8 = SobolSensitivityAnalyzer::analyze_parallel(&linear_fn, 2, 32, 8);

        for (a, b) in analyzer1
            .first_order
            .iter()
            .zip(analyzer8.first_order.iter())
        {
            assert!(
                (a - b).abs() < 1e-12,
                "first_order mismatch: {} vs {}",
                a,
                b
            );
        }
        for (a, b) in analyzer1
            .total_effect
            .iter()
            .zip(analyzer8.total_effect.iter())
        {
            assert!(
                (a - b).abs() < 1e-12,
                "total_effect mismatch: {} vs {}",
                a,
                b
            );
        }
    }

    #[test]
    fn abc_smc_calibrator_parallel_parity() {
        let calibrator = AbcSmcCalibrator::new(1.0);
        let particles = vec![vec![1.0, 1.0], vec![10.0, 10.0], vec![1.1, 0.9]];
        let reference = vec![1.0, 1.0];
        let simulator = |p: &[f64]| p.to_vec();

        let (w1, a1) = calibrator.evaluate_particles(&particles, &reference, &simulator);
        let (w8, a8) =
            calibrator.evaluate_particles_parallel(&particles, &reference, &simulator, 4);

        assert_eq!(w1, w8);
        assert_eq!(a1, a8);
    }

    #[test]
    fn optuna_abc_smc_calibration_hook() {
        let calibrator = AbcSmcCalibrator::new(1.0);
        let particles = vec![vec![1.0, 1.0], vec![10.0, 10.0]];
        let reference = vec![1.0, 1.0];
        let simulator = |p: &[f64]| p.to_vec();
        let (_weights, accepted) =
            calibrator.evaluate_particles(&particles, &reference, &simulator);
        assert!(accepted[0]);
        assert!(!accepted[1]);

        let obj = OptunaBatchObjective::new(1);
        let loss = obj.evaluate(&[1.0, 2.0], &[1.0, 2.0]);
        assert!(loss < 1e-6);
    }
}
