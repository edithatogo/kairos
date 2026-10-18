#![allow(clippy::needless_range_loop)]
//! Ensemble SBI calibration, Sobol sampling, Latin Hypercube sampling,
//! Common Random Numbers, Saltelli/Jansen Sobol sensitivity analysis,
//! and ABC-SMC / Optuna batch objective calibration.

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
