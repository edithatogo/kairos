#![forbid(unsafe_code)]

use crate::DeterministicStream;

/// Identifier for a cause in a competing risks setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CauseId(pub u32);

/// Explicit tie-breaking rule when competing risks sample identical realization times.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TieBreaker {
    /// Lowest CauseId wins.
    LowestCauseId,
    /// Highest CauseId wins.
    HighestCauseId,
    /// Use an explicit user-defined priority ordering.
    ExplicitPriority,
}

/// A non-Markovian or parametric hazard specification.
/// All hazard rates h(t) >= 0.
#[derive(Clone, Debug, PartialEq)]
pub enum HazardRate {
    /// Constant hazard rate: h(t) = lambda (Exponential survival, Markovian).
    Constant { rate: f64 },
    /// Weibull hazard rate: h(t) = (k / lambda) * (t / lambda)^(k - 1).
    /// Shape k > 0, Scale lambda > 0.
    Weibull { shape: f64, scale: f64 },
    /// Gompertz hazard rate: h(t) = eta * exp(b * t).
    /// eta > 0, b >= 0.
    Gompertz { eta: f64, b: f64 },
    /// Piecewise constant hazard across specified ascending intervals.
    /// intervals: [(t_end, rate)], where final rate applies for t >= last t_end.
    PiecewiseConstant {
        intervals: Vec<(f64, f64)>,
        trailing_rate: f64,
    },
    /// Degenerate zero hazard: event never occurs spontaneously (h(t) = 0).
    Zero,
}

impl HazardRate {
    /// Evaluate instantaneous hazard rate h(t) at age or elapsed duration t >= 0.
    pub fn hazard_at(&self, t: f64) -> f64 {
        if t < 0.0 {
            return 0.0;
        }
        match self {
            Self::Constant { rate } => rate.max(0.0),
            Self::Weibull { shape, scale } => {
                if *scale <= 0.0 || *shape <= 0.0 {
                    0.0
                } else if t == 0.0 && *shape < 1.0 {
                    f64::INFINITY
                } else {
                    let k = *shape;
                    let lambda = *scale;
                    (k / lambda) * (t / lambda).powf(k - 1.0)
                }
            }
            Self::Gompertz { eta, b } => {
                if *eta <= 0.0 {
                    0.0
                } else {
                    *eta * (b * t).exp()
                }
            }
            Self::PiecewiseConstant {
                intervals,
                trailing_rate,
            } => {
                for (end_t, rate) in intervals {
                    if t < *end_t {
                        return rate.max(0.0);
                    }
                }
                trailing_rate.max(0.0)
            }
            Self::Zero => 0.0,
        }
    }

    /// Evaluate cumulative hazard H(t) = \int_0^t h(u) du for t >= 0.
    pub fn cumulative_hazard(&self, t: f64) -> f64 {
        if t <= 0.0 {
            return 0.0;
        }
        match self {
            Self::Constant { rate } => rate.max(0.0) * t,
            Self::Weibull { shape, scale } => {
                if *scale <= 0.0 || *shape <= 0.0 {
                    0.0
                } else {
                    (t / scale).powf(*shape)
                }
            }
            Self::Gompertz { eta, b } => {
                if *eta <= 0.0 {
                    0.0
                } else if *b == 0.0 {
                    *eta * t
                } else {
                    (*eta / *b) * ((b * t).exp() - 1.0)
                }
            }
            Self::PiecewiseConstant {
                intervals,
                trailing_rate,
            } => {
                let mut cum = 0.0;
                let mut cur_t = 0.0;
                for (end_t, rate) in intervals {
                    if t <= *end_t {
                        cum += rate.max(0.0) * (t - cur_t);
                        return cum;
                    } else {
                        cum += rate.max(0.0) * (end_t - cur_t);
                        cur_t = *end_t;
                    }
                }
                cum += trailing_rate.max(0.0) * (t - cur_t);
                cum
            }
            Self::Zero => 0.0,
        }
    }

    /// Invert cumulative hazard: solve for t given integrated hazard threshold W = -ln(U) >= 0,
    /// starting from existing accumulated hazard H(t_start).
    /// That is, find \Delta t such that H(t_start + \Delta t) - H(t_start) = target_w.
    pub fn invert_cumulative_hazard(&self, t_start: f64, target_w: f64) -> Option<f64> {
        if target_w <= 0.0 {
            return Some(0.0);
        }
        let t_start = t_start.max(0.0);
        let h_start = self.cumulative_hazard(t_start);
        let total_target = h_start + target_w;

        match self {
            Self::Constant { rate } => {
                let r = rate.max(0.0);
                if r <= 0.0 {
                    None
                } else {
                    Some(target_w / r)
                }
            }
            Self::Weibull { shape, scale } => {
                if *scale <= 0.0 || *shape <= 0.0 {
                    None
                } else {
                    // total_target = (t_end / scale)^k => t_end = scale * (total_target)^(1/k)
                    let t_end = scale * total_target.powf(1.0 / shape);
                    Some((t_end - t_start).max(0.0))
                }
            }
            Self::Gompertz { eta, b } => {
                if *eta <= 0.0 {
                    None
                } else if *b == 0.0 {
                    Some(target_w / eta)
                } else {
                    // total_target = (eta / b) * (exp(b * t_end) - 1)
                    // exp(b * t_end) = 1 + total_target * b / eta
                    let inner = 1.0 + (total_target * b / eta);
                    if inner <= 0.0 {
                        None
                    } else {
                        let t_end = inner.ln() / b;
                        Some((t_end - t_start).max(0.0))
                    }
                }
            }
            Self::PiecewiseConstant {
                intervals,
                trailing_rate,
            } => {
                // Advance interval by interval
                let mut cur_t = t_start;
                let mut remaining_w = target_w;

                // Find which interval t_start is in
                let mut interval_idx = 0;
                while interval_idx < intervals.len() && cur_t >= intervals[interval_idx].0 {
                    interval_idx += 1;
                }

                while interval_idx < intervals.len() {
                    let (end_t, rate) = intervals[interval_idx];
                    let r = rate.max(0.0);
                    let delta_t_max = end_t - cur_t;
                    let capacity_w = r * delta_t_max;

                    if r > 0.0 && remaining_w <= capacity_w {
                        let dt = remaining_w / r;
                        return Some(cur_t + dt - t_start);
                    }
                    remaining_w -= capacity_w;
                    cur_t = end_t;
                    interval_idx += 1;
                }

                // Trailing rate applies
                let r = trailing_rate.max(0.0);
                if r <= 0.0 {
                    None
                } else {
                    let dt = remaining_w / r;
                    Some(cur_t + dt - t_start)
                }
            }
            Self::Zero => None,
        }
    }
}

/// Specification of a single competing risk hazard channel.
#[derive(Clone, Debug)]
pub struct CompetingCause {
    pub cause_id: CauseId,
    pub priority: u32,
    pub hazard: HazardRate,
}

/// Result of sampling competing risks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RealizedEvent {
    pub winning_cause: CauseId,
    pub delta_time: f64,
    pub is_censored: bool,
}

/// Draw a uniform (0, 1) float from a deterministic stream using `rand_distr::Open01`.
pub fn draw_uniform_open01(stream: &mut DeterministicStream) -> f64 {
    stream.sample(rand_distr::Open01)
}

/// Draw exponential standard threshold W = -ln(U) where U ~ Uniform(0, 1) using `rand_distr::Exp1`.
pub fn draw_standard_threshold(stream: &mut DeterministicStream) -> f64 {
    stream.sample(rand_distr::Exp1)
}

/// Competing risks sampler across multiple cause-specific hazards.
/// Samples the minimum time to event across all causes under explicit tie-breaking and right censoring.
#[derive(Clone, Debug)]
pub struct CompetingHazards {
    causes: Vec<CompetingCause>,
    tie_breaker: TieBreaker,
}

impl CompetingHazards {
    pub fn new(causes: Vec<CompetingCause>, tie_breaker: TieBreaker) -> Self {
        Self {
            causes,
            tie_breaker,
        }
    }

    pub fn causes(&self) -> &[CompetingCause] {
        &self.causes
    }

    /// Sample the next competing event starting from `current_age`, subject to right censoring at `censor_age`.
    /// Each cause consumes its threshold draw from the stream in strict CauseId ordering to guarantee
    /// sampling invariance regardless of internal memory layout.
    pub fn sample_next(
        &self,
        current_age: f64,
        censor_age: Option<f64>,
        stream: &mut DeterministicStream,
    ) -> Option<RealizedEvent> {
        if self.causes.is_empty() {
            return None;
        }

        // Sort causes deterministically by CauseId to enforce draw invariance
        let mut sorted_causes = self.causes.clone();
        sorted_causes.sort_by_key(|c| c.cause_id);

        let mut candidate_events: Vec<(CauseId, u32, f64)> = Vec::new();

        for cause in &sorted_causes {
            // Draw threshold W ~ Exp(1)
            let w = draw_standard_threshold(stream);
            if let Some(dt) = cause.hazard.invert_cumulative_hazard(current_age, w) {
                if dt >= 0.0 && !dt.is_infinite() {
                    candidate_events.push((cause.cause_id, cause.priority, dt));
                }
            }
        }

        if candidate_events.is_empty() {
            if let Some(c_age) = censor_age {
                if c_age > current_age {
                    return Some(RealizedEvent {
                        winning_cause: CauseId(u32::MAX),
                        delta_time: c_age - current_age,
                        is_censored: true,
                    });
                }
            }
            return None;
        }

        // Sort candidates by time, breaking ties per policy
        candidate_events.sort_by(|a, b| {
            let time_cmp = a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal);
            if time_cmp != std::cmp::Ordering::Equal {
                return time_cmp;
            }
            match self.tie_breaker {
                TieBreaker::LowestCauseId => a.0.cmp(&b.0),
                TieBreaker::HighestCauseId => b.0.cmp(&a.0),
                TieBreaker::ExplicitPriority => {
                    let prio_cmp = b.1.cmp(&a.1); // higher priority first
                    if prio_cmp != std::cmp::Ordering::Equal {
                        prio_cmp
                    } else {
                        a.0.cmp(&b.0)
                    }
                }
            }
        });

        let (winner_cause, _, min_dt) = candidate_events[0];

        // Check right censoring
        if let Some(c_age) = censor_age {
            let max_dt = (c_age - current_age).max(0.0);
            if min_dt > max_dt {
                return Some(RealizedEvent {
                    winning_cause: CauseId(u32::MAX),
                    delta_time: max_dt,
                    is_censored: true,
                });
            }
        }

        Some(RealizedEvent {
            winning_cause: winner_cause,
            delta_time: min_dt,
            is_censored: false,
        })
    }
}
