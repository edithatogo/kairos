use kairo_ecs_rng::rand_distr::{StandardNormal, StandardUniform};
use kairo_ecs_rng::DeterministicStream;

/// Fixed population sizes, discrepancy thresholds and diagonal perturbations.
#[derive(Clone, Debug)]
pub struct AbcSmcConfig {
    pub particles: usize,
    pub tolerances: Vec<f64>,
    /// Gaussian standard deviation in units of the prior width of each parameter.
    pub kernel_scale: f64,
    /// Maximum proposals per population, including out-of-prior proposals.
    pub max_attempts: usize,
    /// Minimum effective sample size divided by the population size, in (0, 1].
    pub min_ess_fraction: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AbcParticle {
    pub parameters: Vec<f64>,
    pub discrepancy: f64,
    pub weight: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AbcPopulation {
    pub tolerance: f64,
    pub particles: Vec<AbcParticle>,
    pub attempts: usize,
    pub effective_sample_size: f64,
}

#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum AbcSmcError {
    #[error("invalid ABC-SMC configuration: {0}")]
    Configuration(&'static str),
    #[error("generation {generation} exhausted its {attempts} proposals after accepting {accepted} particles")]
    ProposalBudget {
        generation: usize,
        attempts: usize,
        accepted: usize,
    },
    #[error("simulator returned a non-finite or negative discrepancy at generation {generation}, proposal {attempt}")]
    InvalidDiscrepancy { generation: usize, attempt: usize },
    #[error("generation {generation} produced unrepresentable importance weights")]
    InvalidWeights { generation: usize },
    #[error("generation {generation} effective sample size {actual} is below {minimum}")]
    LowEffectiveSampleSize {
        generation: usize,
        actual: f64,
        minimum: f64,
    },
}

/// Sequential Monte Carlo ABC with a rectangular uniform prior.
///
/// The first population uses prior draws. Subsequent populations use weighted
/// ancestor resampling and a fixed diagonal Gaussian proposal. The acceptance
/// kernel is an indicator on the caller's nonnegative discrepancy. The
/// simulator receives a separate versioned RNG stream from the proposal stream.
#[derive(Clone, Debug)]
pub struct AbcSmcCalibrator {
    config: AbcSmcConfig,
    prior: Vec<(f64, f64)>,
}

impl AbcSmcCalibrator {
    pub const VERSION: &'static str = "uniform-diagonal-gaussian-abc-smc-v1";

    /// Each bound is (lower, upper) in original model units.
    pub fn new(config: AbcSmcConfig, bounds: Vec<(f64, f64)>) -> Result<Self, AbcSmcError> {
        if config.particles < 2 || config.max_attempts < config.particles {
            return Err(AbcSmcError::Configuration(
                "need at least two particles and enough proposals",
            ));
        }
        if config.tolerances.len() < 2
            || config.tolerances.iter().any(|t| !t.is_finite() || *t < 0.0)
            || config.tolerances.windows(2).any(|t| t[0] <= t[1])
        {
            return Err(AbcSmcError::Configuration(
                "need strictly decreasing finite nonnegative tolerances",
            ));
        }
        if !config.kernel_scale.is_finite() || config.kernel_scale <= f64::EPSILON {
            return Err(AbcSmcError::Configuration(
                "perturbation scale must be finite and exceed unit-interval precision",
            ));
        }
        if !(config.min_ess_fraction > 0.0 && config.min_ess_fraction <= 1.0) {
            return Err(AbcSmcError::Configuration("ESS fraction must be in (0, 1]"));
        }
        let prior: Vec<_> = bounds
            .into_iter()
            .map(|(low, high)| (low, high - low))
            .collect();
        if prior.is_empty()
            || prior
                .iter()
                .any(|(low, width)| !low.is_finite() || !width.is_finite() || *width <= 0.0)
        {
            return Err(AbcSmcError::Configuration(
                "prior needs finite, increasing bounds",
            ));
        }

        Ok(Self { config, prior })
    }

    /// Simulate once per in-support proposal. Returns all complete populations.
    pub fn run(
        &self,
        seed: u64,
        mut discrepancy: impl FnMut(&[f64], &mut DeterministicStream) -> f64,
    ) -> Result<Vec<AbcPopulation>, AbcSmcError> {
        let mut proposal_rng = DeterministicStream::from_seed_u64(seed, 0);
        let mut simulator_rng = DeterministicStream::from_seed_u64(seed, 1);
        let mut previous = None::<ProposalMixture>;
        let mut populations = Vec::with_capacity(self.config.tolerances.len());

        for (generation, &tolerance) in self.config.tolerances.iter().enumerate() {
            let mut particles = Vec::new();
            let mut unit_points = Vec::new();
            let mut log_weights = Vec::new();
            let mut attempts = 0;
            while particles.len() < self.config.particles {
                if attempts == self.config.max_attempts {
                    return Err(AbcSmcError::ProposalBudget {
                        generation,
                        attempts,
                        accepted: particles.len(),
                    });
                }
                attempts += 1;
                let point: Vec<f64> = match &previous {
                    None => (0..self.prior.len())
                        .map(|_| proposal_rng.sample(StandardUniform))
                        .collect(),
                    Some(mixture) => mixture.draw(&mut proposal_rng, self.config.kernel_scale),
                };
                if point.iter().any(|x| !(0.0..=1.0).contains(x)) {
                    continue;
                }

                let parameters: Vec<_> = point
                    .iter()
                    .zip(&self.prior)
                    .map(|(x, (low, width))| low + x * width)
                    .collect();
                let distance = discrepancy(&parameters, &mut simulator_rng);
                if !distance.is_finite() || distance < 0.0 {
                    return Err(AbcSmcError::InvalidDiscrepancy {
                        generation,
                        attempt: attempts,
                    });
                }
                if distance > tolerance {
                    continue;
                }

                // The uniform prior density and constant Gaussian normalizer
                // cancel within each population. Rejection outside the box
                // truncates the whole mixture, so its normalizer also cancels.
                let log_weight = previous.as_ref().map_or(0.0, |mixture| {
                    -mixture.log_density(&point, self.config.kernel_scale)
                });
                particles.push(AbcParticle {
                    parameters,
                    discrepancy: distance,
                    weight: 0.0,
                });
                unit_points.push(point);
                log_weights.push(log_weight);
            }

            let weights =
                normalize(&log_weights).ok_or(AbcSmcError::InvalidWeights { generation })?;
            let count = self.config.particles as f64;
            let ess = if previous.is_none() {
                count
            } else {
                weights
                    .iter()
                    .map(|w| w * w)
                    .sum::<f64>()
                    .recip()
                    .min(count)
            };
            let minimum = self.config.min_ess_fraction * count;
            if ess < minimum {
                return Err(AbcSmcError::LowEffectiveSampleSize {
                    generation,
                    actual: ess,
                    minimum,
                });
            }
            for (particle, &weight) in particles.iter_mut().zip(&weights) {
                particle.weight = weight;
            }
            previous = Some(ProposalMixture::new(unit_points, &weights));
            populations.push(AbcPopulation {
                tolerance,
                particles,
                attempts,
                effective_sample_size: ess,
            });
        }
        Ok(populations)
    }
}

struct ProposalMixture {
    points: Vec<Vec<f64>>,
    log_weights: Vec<f64>,
    cumulative: Vec<f64>,
}

impl ProposalMixture {
    fn new(points: Vec<Vec<f64>>, weights: &[f64]) -> Self {
        let cumulative = weights
            .iter()
            .scan(0.0, |sum, weight| {
                *sum += weight;
                Some(*sum)
            })
            .collect();
        Self {
            points,
            cumulative,
            log_weights: weights.iter().map(|weight| weight.ln()).collect(),
        }
    }

    fn draw(&self, rng: &mut DeterministicStream, scale: f64) -> Vec<f64> {
        let u: f64 = rng.sample(StandardUniform);
        let draw = u * self.cumulative[self.cumulative.len() - 1];
        let index = self.cumulative.partition_point(|&weight| weight <= draw);
        self.points[index.min(self.points.len() - 1)]
            .iter()
            .map(|x| {
                let z: f64 = rng.sample(StandardNormal);
                x + scale * z
            })
            .collect()
    }

    fn log_density(&self, point: &[f64], scale: f64) -> f64 {
        // Shift terms by their running maximum to keep the exponential sum
        // representable, including mixtures with very small component weights.
        let mut highest = f64::NEG_INFINITY;
        let mut total = 0.0;
        for (ancestor, weight) in self.points.iter().zip(&self.log_weights) {
            let squared_distance = ancestor
                .iter()
                .zip(point)
                .map(|(a, p)| ((a - p) / scale).powi(2))
                .sum::<f64>();
            let term = weight - squared_distance / 2.0;
            if !term.is_finite() {
                continue;
            }
            if term > highest {
                total = total * (highest - term).exp() + 1.0;
                highest = term;
            } else {
                total += (term - highest).exp();
            }
        }
        highest + total.ln()
    }
}

fn normalize(log_weights: &[f64]) -> Option<Vec<f64>> {
    if log_weights.iter().any(|x| !x.is_finite()) {
        return None;
    }
    let max = log_weights
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let mut weights: Vec<_> = log_weights.iter().map(|w| (w - max).exp()).collect();
    let sum: f64 = weights.iter().sum();
    for weight in &mut weights {
        *weight /= sum;
    }
    Some(weights)
}
