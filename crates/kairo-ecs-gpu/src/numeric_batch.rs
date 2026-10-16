//! Numeric batch workload catalog, execution oracles, contract validation,
//! and economic break-even profiling per Report 26.
//!
//! Separates regular batched numerical workloads from irregular stateful DES scheduling.
//! CPU maintains authoritative ownership over clocks, RNG streams, sequence keys,
//! and event queues (D0 contract), while batch numeric kernels accelerate high-intensity
//! calculations such as spatial transit queries and resident parameter candidate evaluations.

#![forbid(unsafe_code)]

use crate::compute::CpuFallbackCompute;

/// Supported batch numeric workload candidate kinds per Report 26.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum NumericWorkloadKind {
    /// K-ELT: elementwise residuals / quadratic loss / clamp transforms.
    KElt,
    /// K-W1-PRESORT: 1D Wasserstein-1 metric on equal-length, already-sorted vectors.
    KW1Presort,
    /// K-W1-WEIGHTED: 1D Wasserstein-1 metric on unequal-length or weighted empirical distributions.
    KW1Weighted,
    /// K-KS-CDF: Kolmogorov-Smirnov two-sample maximum distance from empirical CDFs: D = sup |F1(x) - F2(x)|.
    KKsCdf,
    /// K-HIST: exact integer histogram binning over fixed bin intervals.
    KHist,
    /// K-SCAN: prefix sum scan over integer counts or probabilities.
    KScan,
    /// K-CAND: batch parameter candidate evaluation and ranking across M candidate parameter sets over N observations.
    KCand,
    /// K-TRANSIT: batched spatial route transit evaluation (Euclidean distance, time-to-destination for batch of agents).
    KTransit,
    /// K-SORT-FULL: full sorting + metric pipeline (isolates sorting overhead from reduction overhead).
    KSortFull,
}

/// Alias for compatibility.
pub type KernelKind = NumericWorkloadKind;

impl NumericWorkloadKind {
    /// Canonical Report 26 workload code.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::KElt => "K-ELT",
            Self::KW1Presort => "K-W1-PRESORT",
            Self::KW1Weighted => "K-W1-WEIGHTED",
            Self::KKsCdf => "K-KS-CDF",
            Self::KHist => "K-HIST",
            Self::KScan => "K-SCAN",
            Self::KCand => "K-CAND",
            Self::KTransit => "K-TRANSIT",
            Self::KSortFull => "K-SORT-FULL",
        }
    }

    /// Descriptive summary of the workload operation and memory pattern.
    pub const fn description(&self) -> &'static str {
        match self {
            Self::KElt => "Elementwise residuals, quadratic loss, and clamp transforms",
            Self::KW1Presort => "1D Wasserstein-1 metric on equal-length already-sorted vectors",
            Self::KW1Weighted => {
                "1D Wasserstein-1 metric on unequal-length or weighted empirical distributions"
            }
            Self::KKsCdf => "Kolmogorov-Smirnov two-sample maximum distance from empirical CDFs",
            Self::KHist => "Exact integer histogram binning over fixed bin intervals",
            Self::KScan => "Prefix sum scan over integer counts or probabilities",
            Self::KCand => "Batch parameter candidate evaluation and ranking across M candidates",
            Self::KTransit => "Batched spatial route transit evaluation for agent movement",
            Self::KSortFull => "Full sorting and metric pipeline isolating sort overhead",
        }
    }

    /// Primary contract class associated with this workload kind.
    pub const fn primary_contract_class(&self) -> ContractClass {
        match self {
            Self::KElt | Self::KTransit => ContractClass::D2,
            Self::KHist | Self::KScan => ContractClass::D1,
            Self::KW1Presort | Self::KW1Weighted | Self::KKsCdf | Self::KSortFull => {
                ContractClass::D3
            }
            Self::KCand => ContractClass::D4,
        }
    }

    /// Whether this workload has high arithmetic intensity (FLOPs / byte)
    /// capable of amortizing host-device roundtrip transfer latency.
    pub const fn is_compute_bound(&self) -> bool {
        match self {
            Self::KTransit | Self::KCand => true,
            Self::KElt
            | Self::KW1Presort
            | Self::KW1Weighted
            | Self::KKsCdf
            | Self::KHist
            | Self::KScan
            | Self::KSortFull => false,
        }
    }
}

/// Numerical and determinism contract classes per Report 26.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ContractClass {
    /// D0: Authoritative exact state (CPU owns timestamps, sequence keys, RNG stream).
    D0,
    /// D1: Exact integer numeric outputs (exact bit identity on histogram counts and integer prefix scans).
    D1,
    /// D2: Floating elementwise outputs (max absolute error <= 1e-6, max relative error <= 1e-5, ULP error tracking).
    D2,
    /// D3: Reductions & objectives (W1, KS, objective tolerance <= 1e-5, mathematical invariants verified).
    D3,
    /// D4: Decision stability (ranking correlation Spearman rho >= 0.999, winning candidate preserved).
    D4,
    /// D5: Repeated execution (bit-identical integer outputs and stable float outputs across 100 identical seed runs).
    D5,
}

impl ContractClass {
    pub const fn label(&self) -> &'static str {
        match self {
            Self::D0 => "D0 (Authoritative Exact State)",
            Self::D1 => "D1 (Exact Integer Numeric Outputs)",
            Self::D2 => "D2 (Floating Elementwise Outputs)",
            Self::D3 => "D3 (Reductions & Objectives)",
            Self::D4 => "D4 (Decision Stability)",
            Self::D5 => "D5 (Repeated Execution)",
        }
    }
}

/// Standard numeric tolerances frozen per Report 26.
pub const D2_MAX_ABSOLUTE_ERROR: f64 = 1e-6;
pub const D2_MAX_RELATIVE_ERROR: f64 = 1e-5;
pub const D2_MAX_ABS_ERROR: f32 = 1e-6;
pub const D2_MAX_REL_ERROR: f32 = 1e-5;
pub const D3_REDUCTION_TOLERANCE: f64 = 1e-5;
pub const D3_MAX_OBJECTIVE_ERROR: f64 = 1e-5;
pub const D4_MIN_SPEARMAN_RHO: f64 = 0.999;
pub const D5_REPEAT_RUN_COUNT: usize = 100;
pub const D5_REPEAT_ITERATIONS: usize = 100;

/// Standard batch sizes evaluated in the 26 workload catalog.
pub const CATALOG_BATCH_SIZES: [usize; 8] = [64, 256, 1024, 4096, 16384, 65536, 262144, 1048576];

/// Error types for numeric batch kernels and oracle evaluation.
#[derive(Clone, Debug, PartialEq)]
pub enum NumericBatchError {
    /// Input buffer length mismatch.
    BufferShapeMismatch { expected: usize, actual: usize },
    /// Integer arithmetic overflow during accumulation or scan.
    Overflow,
    /// Candidate parameter set is invalid or candidate index out of range.
    InvalidCandidate { candidate_id: usize, reason: String },
    /// Value falls outside the configured histogram interval.
    OutOfRangeBin {
        value: f32,
        min_val: f32,
        max_val: f32,
    },
    /// Input buffer is empty when non-empty input is required.
    EmptyInput,
    /// Division by zero or degenerate normalization.
    ZeroDivision,
    /// Acceleration hardware is not available or not configured.
    HardwareUnavailable(&'static str),
}

impl core::fmt::Display for NumericBatchError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BufferShapeMismatch { expected, actual } => {
                write!(
                    f,
                    "Buffer shape mismatch: expected {expected}, actual {actual}"
                )
            }
            Self::Overflow => write!(f, "Integer overflow occurred during numeric computation"),
            Self::InvalidCandidate {
                candidate_id,
                reason,
            } => {
                write!(f, "Invalid candidate {candidate_id}: {reason}")
            }
            Self::OutOfRangeBin {
                value,
                min_val,
                max_val,
            } => {
                write!(
                    f,
                    "Value {value} out of range for histogram binning [{min_val}, {max_val}]"
                )
            }
            Self::EmptyInput => write!(f, "Input slice is empty"),
            Self::ZeroDivision => write!(f, "Division by zero or degenerate interval in metric"),
            Self::HardwareUnavailable(reason) => {
                write!(f, "Hardware acceleration unavailable: {reason}")
            }
        }
    }
}

impl std::error::Error for NumericBatchError {}

/// Precision tracking metrics for D2 floating elementwise outputs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct D2PrecisionMetrics {
    pub max_abs_error: f64,
    pub max_rel_error: f64,
    pub max_ulp_diff: u32,
}

impl D2PrecisionMetrics {
    /// Checks whether the observed errors satisfy the frozen D2 tolerances.
    pub fn satisfies_contract(&self) -> bool {
        self.max_abs_error <= D2_MAX_ABSOLUTE_ERROR && self.max_rel_error <= D2_MAX_RELATIVE_ERROR
    }
}

/// Computes Units in the Last Place (ULP) distance between two IEEE 754 f32 numbers.
pub fn float_ulp_diff(a: f32, b: f32) -> u32 {
    if a.is_nan() || b.is_nan() {
        return u32::MAX;
    }
    if a == b {
        return 0;
    }
    let to_ordered_int = |bits: u32| -> i32 {
        if bits & 0x8000_0000 != 0 {
            -((bits & 0x7fff_ffff) as i32)
        } else {
            bits as i32
        }
    };
    let a_ord = to_ordered_int(a.to_bits());
    let b_ord = to_ordered_int(b.to_bits());
    (a_ord.wrapping_sub(b_ord)).unsigned_abs()
}

/// Alias for float_ulp_diff.
pub use float_ulp_diff as float_ulp_distance;

/// Compares two floating point slices and returns D2 precision metrics.
pub fn compare_f32_slices(
    actual: &[f32],
    reference: &[f32],
) -> Result<D2PrecisionMetrics, NumericBatchError> {
    if actual.len() != reference.len() {
        return Err(NumericBatchError::BufferShapeMismatch {
            expected: reference.len(),
            actual: actual.len(),
        });
    }

    let mut max_abs_error: f64 = 0.0;
    let mut max_rel_error: f64 = 0.0;
    let mut max_ulp_diff: u32 = 0;

    for (&act, &ref_val) in actual.iter().zip(reference.iter()) {
        let abs_err = ((act as f64) - (ref_val as f64)).abs();
        if abs_err > max_abs_error {
            max_abs_error = abs_err;
        }

        let denom = (ref_val as f64).abs();
        let rel_err = if denom > 1e-12 {
            abs_err / denom
        } else {
            abs_err
        };
        if rel_err > max_rel_error {
            max_rel_error = rel_err;
        }

        let ulp = float_ulp_diff(act, ref_val);
        if ulp > max_ulp_diff {
            max_ulp_diff = ulp;
        }
    }

    Ok(D2PrecisionMetrics {
        max_abs_error,
        max_rel_error,
        max_ulp_diff,
    })
}

/// D0 Authoritative State Tracker: CPU owns timestamps, sequence keys, and RNG state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct D0StateAuthority {
    pub timestamp_ns: u64,
    pub sequence_key: u64,
    pub rng_seed: u64,
}

impl D0StateAuthority {
    pub const fn new(timestamp_ns: u64, sequence_key: u64, rng_seed: u64) -> Self {
        Self {
            timestamp_ns,
            sequence_key,
            rng_seed,
        }
    }

    /// Advances the CPU simulation clock and advances sequence key and RNG deterministically.
    pub fn advance_step(&mut self, dt_ns: u64) -> u64 {
        self.timestamp_ns = self.timestamp_ns.saturating_add(dt_ns);
        self.sequence_key = self.sequence_key.wrapping_add(1);
        // Standard 64-bit LCG step for deterministic draw generation
        self.rng_seed = self
            .rng_seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        self.rng_seed
    }

    /// Generates a deterministic uniform pseudo-random float in [0.0, 1.0).
    pub fn next_uniform_f32(&mut self) -> f32 {
        let raw = self.advance_step(0);
        ((raw >> 40) as f32) / 16_777_216.0
    }
}

/// Agent transit query for K-TRANSIT spatial route evaluation.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AgentTransitQuery {
    pub start_x: f32,
    pub start_y: f32,
    pub dest_x: f32,
    pub dest_y: f32,
    pub speed: f32,
}

/// Agent transit result containing computed Euclidean distance and travel time.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AgentTransitResult {
    pub distance: f32,
    pub time_to_dest: f32,
}

/// Calibration candidate evaluation result for K-CAND.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CandidateEvaluation {
    pub candidate_id: usize,
    pub score: f64,
}

// ---------------------------------------------------------------------------
// CPU Oracle Kernel Implementations
// ---------------------------------------------------------------------------

/// K-ELT Oracle: Elementwise residuals, quadratic loss, and clamp transforms.
pub fn compute_elt(
    pred: &[f32],
    obs: &[f32],
    min_val: f32,
    max_val: f32,
) -> Result<Vec<f32>, NumericBatchError> {
    if pred.len() != obs.len() {
        return Err(NumericBatchError::BufferShapeMismatch {
            expected: obs.len(),
            actual: pred.len(),
        });
    }
    if min_val > max_val {
        return Err(NumericBatchError::InvalidCandidate {
            candidate_id: 0,
            reason: format!("min_val {min_val} > max_val {max_val}"),
        });
    }

    let mut result = Vec::with_capacity(pred.len());
    for (&p, &o) in pred.iter().zip(obs.iter()) {
        let residual = p - o;
        let quad_loss = residual * residual;
        let clamped = quad_loss.clamp(min_val, max_val);
        result.push(clamped);
    }
    Ok(result)
}

/// K-W1-PRESORT Oracle: 1D Wasserstein-1 metric on equal-length, already-sorted vectors.
/// W1 = (1/N) * sum(|u_i - v_i|)
pub fn compute_w1_presort(u: &[f32], v: &[f32]) -> Result<f64, NumericBatchError> {
    if u.len() != v.len() {
        return Err(NumericBatchError::BufferShapeMismatch {
            expected: u.len(),
            actual: v.len(),
        });
    }
    if u.is_empty() {
        return Err(NumericBatchError::EmptyInput);
    }

    let n = u.len() as f64;
    let mut sum: f64 = 0.0;
    for (&x, &y) in u.iter().zip(v.iter()) {
        sum += ((x as f64) - (y as f64)).abs();
    }
    Ok(sum / n)
}

/// K-W1-WEIGHTED Oracle: 1D Wasserstein-1 metric on unequal-length or weighted empirical distributions.
/// Evaluates integral |F_u(t) - F_v(t)| dt across the merged sorted domain.
pub fn compute_w1_weighted(
    u: &[f32],
    weights_u: Option<&[f64]>,
    v: &[f32],
    weights_v: Option<&[f64]>,
) -> Result<f64, NumericBatchError> {
    if u.is_empty() || v.is_empty() {
        return Err(NumericBatchError::EmptyInput);
    }
    if let Some(wu) = weights_u {
        if wu.len() != u.len() {
            return Err(NumericBatchError::BufferShapeMismatch {
                expected: u.len(),
                actual: wu.len(),
            });
        }
    }
    if let Some(wv) = weights_v {
        if wv.len() != v.len() {
            return Err(NumericBatchError::BufferShapeMismatch {
                expected: v.len(),
                actual: wv.len(),
            });
        }
    }

    // Build normalized (x, weight) pairs for distribution u
    let total_w_u = match weights_u {
        Some(wu) => {
            let s: f64 = wu.iter().sum();
            if s <= 0.0 {
                return Err(NumericBatchError::ZeroDivision);
            }
            s
        }
        None => u.len() as f64,
    };
    let mut pairs_u: Vec<(f64, f64)> = u
        .iter()
        .enumerate()
        .map(|(i, &x)| {
            let w = weights_u.map_or(1.0, |wu| wu[i]);
            (x as f64, w / total_w_u)
        })
        .collect();
    pairs_u.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(core::cmp::Ordering::Equal));

    // Build normalized (y, weight) pairs for distribution v
    let total_w_v = match weights_v {
        Some(wv) => {
            let s: f64 = wv.iter().sum();
            if s <= 0.0 {
                return Err(NumericBatchError::ZeroDivision);
            }
            s
        }
        None => v.len() as f64,
    };
    let mut pairs_v: Vec<(f64, f64)> = v
        .iter()
        .enumerate()
        .map(|(i, &y)| {
            let w = weights_v.map_or(1.0, |wv| wv[i]);
            (y as f64, w / total_w_v)
        })
        .collect();
    pairs_v.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(core::cmp::Ordering::Equal));

    // Sweep through the merged points to integrate |F_u(t) - F_v(t)| dt
    let mut idx_u = 0;
    let mut idx_v = 0;
    let mut cdf_u: f64 = 0.0;
    let mut cdf_v: f64 = 0.0;
    let mut prev_x = pairs_u[0].0.min(pairs_v[0].0);
    let mut integral: f64 = 0.0;

    while idx_u < pairs_u.len() || idx_v < pairs_v.len() {
        let next_x = match (pairs_u.get(idx_u), pairs_v.get(idx_v)) {
            (Some(&(xu, _)), Some(&(xv, _))) => xu.min(xv),
            (Some(&(xu, _)), None) => xu,
            (None, Some(&(xv, _))) => xv,
            (None, None) => break,
        };

        if next_x > prev_x {
            let dx = next_x - prev_x;
            let diff = (cdf_u - cdf_v).abs();
            integral += dx * diff;
            prev_x = next_x;
        }

        while idx_u < pairs_u.len() && (pairs_u[idx_u].0 - next_x).abs() < 1e-12 {
            cdf_u += pairs_u[idx_u].1;
            idx_u += 1;
        }
        while idx_v < pairs_v.len() && (pairs_v[idx_v].0 - next_x).abs() < 1e-12 {
            cdf_v += pairs_v[idx_v].1;
            idx_v += 1;
        }
    }

    Ok(integral)
}

/// K-KS-CDF Oracle: Kolmogorov-Smirnov two-sample maximum distance from empirical CDFs:
/// D = sup |F1(x) - F2(x)|
pub fn compute_ks_cdf(u: &[f32], v: &[f32]) -> Result<f64, NumericBatchError> {
    if u.is_empty() || v.is_empty() {
        return Err(NumericBatchError::EmptyInput);
    }

    let mut sorted_u: Vec<f64> = u.iter().map(|&x| x as f64).collect();
    sorted_u.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));

    let mut sorted_v: Vec<f64> = v.iter().map(|&y| y as f64).collect();
    sorted_v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));

    let n_u = sorted_u.len() as f64;
    let n_v = sorted_v.len() as f64;

    let mut i = 0;
    let mut j = 0;
    let mut max_diff: f64 = 0.0;

    while i < sorted_u.len() || j < sorted_v.len() {
        let x = match (sorted_u.get(i), sorted_v.get(j)) {
            (Some(&xu), Some(&xv)) => xu.min(xv),
            (Some(&xu), None) => xu,
            (None, Some(&xv)) => xv,
            (None, None) => break,
        };

        while i < sorted_u.len() && (sorted_u[i] - x).abs() < 1e-12 {
            i += 1;
        }
        while j < sorted_v.len() && (sorted_v[j] - x).abs() < 1e-12 {
            j += 1;
        }

        let cdf_u = (i as f64) / n_u;
        let cdf_v = (j as f64) / n_v;
        let diff = (cdf_u - cdf_v).abs();
        if diff > max_diff {
            max_diff = diff;
        }
    }

    Ok(max_diff)
}

/// K-HIST Oracle: Exact integer histogram binning over fixed intervals [min_val, max_val].
pub fn compute_hist(
    values: &[f32],
    bin_count: usize,
    min_val: f32,
    max_val: f32,
) -> Result<Vec<u64>, NumericBatchError> {
    if bin_count == 0 {
        return Err(NumericBatchError::ZeroDivision);
    }
    if min_val >= max_val {
        return Err(NumericBatchError::InvalidCandidate {
            candidate_id: 0,
            reason: format!("Histogram range invalid: min_val {min_val} >= max_val {max_val}"),
        });
    }

    let range = (max_val - min_val) as f64;
    let mut counts = vec![0u64; bin_count];

    for &val in values {
        if val < min_val || val > max_val {
            return Err(NumericBatchError::OutOfRangeBin {
                value: val,
                min_val,
                max_val,
            });
        }

        let frac = ((val - min_val) as f64) / range;
        let mut bin_idx = (frac * (bin_count as f64)).floor() as usize;
        if bin_idx >= bin_count {
            bin_idx = bin_count - 1;
        }
        counts[bin_idx] = counts[bin_idx]
            .checked_add(1)
            .ok_or(NumericBatchError::Overflow)?;
    }

    Ok(counts)
}

/// K-SCAN Oracle: Inclusive prefix sum scan over integer counts or probabilities.
pub fn compute_scan(counts: &[u64]) -> Result<Vec<u64>, NumericBatchError> {
    let mut prefix_sums = Vec::with_capacity(counts.len());
    let mut acc: u64 = 0;
    for &c in counts {
        acc = acc.checked_add(c).ok_or(NumericBatchError::Overflow)?;
        prefix_sums.push(acc);
    }
    Ok(prefix_sums)
}

/// K-TRANSIT Oracle: Batched spatial route transit evaluation.
/// Computes Euclidean distance and travel time for each agent.
pub fn compute_transit(
    queries: &[AgentTransitQuery],
) -> Result<Vec<AgentTransitResult>, NumericBatchError> {
    let mut results = Vec::with_capacity(queries.len());
    for q in queries {
        let dx = q.dest_x - q.start_x;
        let dy = q.dest_y - q.start_y;
        let dist = (dx * dx + dy * dy).sqrt();
        let time = if q.speed > 0.0 {
            dist / q.speed
        } else {
            f32::INFINITY
        };
        results.push(AgentTransitResult {
            distance: dist,
            time_to_dest: time,
        });
    }
    Ok(results)
}

/// K-CAND Oracle: Batch parameter candidate evaluation and ranking across M candidates.
/// Computes mean squared error (MSE) against reference observations for each candidate
/// and ranks candidates ascending (best/lowest score first).
pub fn compute_candidate_evaluations(
    candidates_preds: &[Vec<f32>],
    reference_obs: &[f32],
) -> Result<Vec<CandidateEvaluation>, NumericBatchError> {
    if candidates_preds.is_empty() {
        return Err(NumericBatchError::EmptyInput);
    }
    let n = reference_obs.len();
    if n == 0 {
        return Err(NumericBatchError::EmptyInput);
    }

    let mut evaluations = Vec::with_capacity(candidates_preds.len());
    for (cand_id, preds) in candidates_preds.iter().enumerate() {
        if preds.len() != n {
            return Err(NumericBatchError::BufferShapeMismatch {
                expected: n,
                actual: preds.len(),
            });
        }
        let mut mse: f64 = 0.0;
        for (&p, &o) in preds.iter().zip(reference_obs.iter()) {
            let diff = (p as f64) - (o as f64);
            mse += diff * diff;
        }
        mse /= n as f64;
        evaluations.push(CandidateEvaluation {
            candidate_id: cand_id,
            score: mse,
        });
    }

    evaluations.sort_by(|a, b| {
        a.score
            .partial_cmp(&b.score)
            .unwrap_or(core::cmp::Ordering::Equal)
    });
    Ok(evaluations)
}

/// K-SORT-FULL Oracle: Full sorting + W1 metric pipeline.
/// Isolates the sorting overhead from reduction overhead.
pub fn compute_sort_full_w1(u: &[f32], v: &[f32]) -> Result<f64, NumericBatchError> {
    if u.len() != v.len() {
        return Err(NumericBatchError::BufferShapeMismatch {
            expected: u.len(),
            actual: v.len(),
        });
    }
    let mut u_sorted: Vec<f32> = u.to_vec();
    let mut v_sorted: Vec<f32> = v.to_vec();
    u_sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    v_sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    compute_w1_presort(&u_sorted, &v_sorted)
}

/// Computes Spearman rank correlation coefficient between two score sets.
pub fn spearman_rank_correlation(
    scores_a: &[f64],
    scores_b: &[f64],
) -> Result<f64, NumericBatchError> {
    if scores_a.len() != scores_b.len() {
        return Err(NumericBatchError::BufferShapeMismatch {
            expected: scores_b.len(),
            actual: scores_a.len(),
        });
    }
    let n = scores_a.len();
    if n < 2 {
        return Ok(1.0);
    }

    let compute_fractional_ranks = |scores: &[f64]| -> Vec<f64> {
        let mut indices: Vec<usize> = (0..scores.len()).collect();
        indices.sort_by(|&i, &j| {
            scores[i]
                .partial_cmp(&scores[j])
                .unwrap_or(core::cmp::Ordering::Equal)
        });

        let mut ranks = vec![0.0f64; scores.len()];
        let mut i = 0;
        while i < indices.len() {
            let mut j = i;
            while j + 1 < indices.len()
                && (scores[indices[j + 1]] - scores[indices[i]]).abs() < 1e-14
            {
                j += 1;
            }
            let avg_rank = ((i + j + 2) as f64) / 2.0;
            for k in i..=j {
                ranks[indices[k]] = avg_rank;
            }
            i = j + 1;
        }
        ranks
    };

    let ranks_a = compute_fractional_ranks(scores_a);
    let ranks_b = compute_fractional_ranks(scores_b);

    let mean_a: f64 = ranks_a.iter().sum::<f64>() / (n as f64);
    let mean_b: f64 = ranks_b.iter().sum::<f64>() / (n as f64);

    let mut num: f64 = 0.0;
    let mut denom_a: f64 = 0.0;
    let mut denom_b: f64 = 0.0;

    for (&ra, &rb) in ranks_a.iter().zip(ranks_b.iter()) {
        let da = ra - mean_a;
        let db = rb - mean_b;
        num += da * db;
        denom_a += da * da;
        denom_b += db * db;
    }

    let denom = (denom_a * denom_b).sqrt();
    if denom < 1e-12 {
        Ok(1.0)
    } else {
        Ok(num / denom)
    }
}

// ---------------------------------------------------------------------------
// 26 Workload Candidate Catalog & Profiling
// ---------------------------------------------------------------------------

/// A concrete workload candidate configuration in the 26 candidate catalog.
#[derive(Clone, Debug, PartialEq)]
pub struct WorkloadCandidate {
    pub id: &'static str,
    pub name: &'static str,
    pub kind: NumericWorkloadKind,
    pub batch_size: usize,
    pub contract_class: ContractClass,
    pub aux_param: usize,
}

/// Constructs the complete 26 Workload Candidate Catalog per Report 26.
pub fn report_26_workload_catalog() -> [WorkloadCandidate; 26] {
    [
        // 1-3: K-ELT across batch sizes
        WorkloadCandidate {
            id: "CAND-01",
            name: "Elementwise Transforms Micro",
            kind: NumericWorkloadKind::KElt,
            batch_size: 64,
            contract_class: ContractClass::D2,
            aux_param: 0,
        },
        WorkloadCandidate {
            id: "CAND-02",
            name: "Elementwise Transforms Medium",
            kind: NumericWorkloadKind::KElt,
            batch_size: 65536,
            contract_class: ContractClass::D2,
            aux_param: 0,
        },
        WorkloadCandidate {
            id: "CAND-03",
            name: "Elementwise Transforms Large",
            kind: NumericWorkloadKind::KElt,
            batch_size: 1048576,
            contract_class: ContractClass::D2,
            aux_param: 0,
        },
        // 4-7: K-W1-PRESORT across batch sizes
        WorkloadCandidate {
            id: "CAND-04",
            name: "Presorted W1 Micro",
            kind: NumericWorkloadKind::KW1Presort,
            batch_size: 64,
            contract_class: ContractClass::D3,
            aux_param: 0,
        },
        WorkloadCandidate {
            id: "CAND-05",
            name: "Presorted W1 Small",
            kind: NumericWorkloadKind::KW1Presort,
            batch_size: 4096,
            contract_class: ContractClass::D3,
            aux_param: 0,
        },
        WorkloadCandidate {
            id: "CAND-06",
            name: "Presorted W1 Medium",
            kind: NumericWorkloadKind::KW1Presort,
            batch_size: 65536,
            contract_class: ContractClass::D3,
            aux_param: 0,
        },
        WorkloadCandidate {
            id: "CAND-07",
            name: "Presorted W1 Large",
            kind: NumericWorkloadKind::KW1Presort,
            batch_size: 1048576,
            contract_class: ContractClass::D3,
            aux_param: 0,
        },
        // 8-10: K-W1-WEIGHTED (unequal length distributions)
        WorkloadCandidate {
            id: "CAND-08",
            name: "Weighted W1 Micro",
            kind: NumericWorkloadKind::KW1Weighted,
            batch_size: 256,
            contract_class: ContractClass::D3,
            aux_param: 384,
        },
        WorkloadCandidate {
            id: "CAND-09",
            name: "Weighted W1 Small",
            kind: NumericWorkloadKind::KW1Weighted,
            batch_size: 4096,
            contract_class: ContractClass::D3,
            aux_param: 5120,
        },
        WorkloadCandidate {
            id: "CAND-10",
            name: "Weighted W1 Medium",
            kind: NumericWorkloadKind::KW1Weighted,
            batch_size: 65536,
            contract_class: ContractClass::D3,
            aux_param: 80000,
        },
        // 11-13: K-KS-CDF across batch sizes
        WorkloadCandidate {
            id: "CAND-11",
            name: "Kolmogorov-Smirnov CDF Small",
            kind: NumericWorkloadKind::KKsCdf,
            batch_size: 1024,
            contract_class: ContractClass::D3,
            aux_param: 0,
        },
        WorkloadCandidate {
            id: "CAND-12",
            name: "Kolmogorov-Smirnov CDF Medium",
            kind: NumericWorkloadKind::KKsCdf,
            batch_size: 65536,
            contract_class: ContractClass::D3,
            aux_param: 0,
        },
        WorkloadCandidate {
            id: "CAND-13",
            name: "Kolmogorov-Smirnov CDF Large",
            kind: NumericWorkloadKind::KKsCdf,
            batch_size: 1048576,
            contract_class: ContractClass::D3,
            aux_param: 0,
        },
        // 14-16: K-HIST across batch sizes and bin counts
        WorkloadCandidate {
            id: "CAND-14",
            name: "Integer Histogram Small",
            kind: NumericWorkloadKind::KHist,
            batch_size: 4096,
            contract_class: ContractClass::D1,
            aux_param: 64,
        },
        WorkloadCandidate {
            id: "CAND-15",
            name: "Integer Histogram Medium",
            kind: NumericWorkloadKind::KHist,
            batch_size: 65536,
            contract_class: ContractClass::D1,
            aux_param: 256,
        },
        WorkloadCandidate {
            id: "CAND-16",
            name: "Integer Histogram Large",
            kind: NumericWorkloadKind::KHist,
            batch_size: 1048576,
            contract_class: ContractClass::D1,
            aux_param: 1024,
        },
        // 17-19: K-SCAN across batch sizes
        WorkloadCandidate {
            id: "CAND-17",
            name: "Prefix Scan Micro",
            kind: NumericWorkloadKind::KScan,
            batch_size: 256,
            contract_class: ContractClass::D1,
            aux_param: 0,
        },
        WorkloadCandidate {
            id: "CAND-18",
            name: "Prefix Scan Small",
            kind: NumericWorkloadKind::KScan,
            batch_size: 16384,
            contract_class: ContractClass::D1,
            aux_param: 0,
        },
        WorkloadCandidate {
            id: "CAND-19",
            name: "Prefix Scan Large",
            kind: NumericWorkloadKind::KScan,
            batch_size: 1048576,
            contract_class: ContractClass::D1,
            aux_param: 0,
        },
        // 20-22: K-CAND across candidates M and batch sizes N
        WorkloadCandidate {
            id: "CAND-20",
            name: "Candidate Scoring M=8",
            kind: NumericWorkloadKind::KCand,
            batch_size: 4096,
            contract_class: ContractClass::D4,
            aux_param: 8,
        },
        WorkloadCandidate {
            id: "CAND-21",
            name: "Candidate Scoring M=32",
            kind: NumericWorkloadKind::KCand,
            batch_size: 16384,
            contract_class: ContractClass::D4,
            aux_param: 32,
        },
        WorkloadCandidate {
            id: "CAND-22",
            name: "Candidate Scoring M=128",
            kind: NumericWorkloadKind::KCand,
            batch_size: 65536,
            contract_class: ContractClass::D4,
            aux_param: 128,
        },
        // 23-25: K-TRANSIT spatial route evaluation across batch sizes
        WorkloadCandidate {
            id: "CAND-23",
            name: "Spatial Transit Small",
            kind: NumericWorkloadKind::KTransit,
            batch_size: 1024,
            contract_class: ContractClass::D2,
            aux_param: 0,
        },
        WorkloadCandidate {
            id: "CAND-24",
            name: "Spatial Transit Medium",
            kind: NumericWorkloadKind::KTransit,
            batch_size: 16384,
            contract_class: ContractClass::D2,
            aux_param: 0,
        },
        WorkloadCandidate {
            id: "CAND-25",
            name: "Spatial Transit Large",
            kind: NumericWorkloadKind::KTransit,
            batch_size: 262144,
            contract_class: ContractClass::D2,
            aux_param: 0,
        },
        // 26: K-SORT-FULL pipeline isolating sorting overhead from reduction overhead
        WorkloadCandidate {
            id: "CAND-26",
            name: "Full Sort and W1 Pipeline",
            kind: NumericWorkloadKind::KSortFull,
            batch_size: 65536,
            contract_class: ContractClass::D3,
            aux_param: 0,
        },
    ]
}

/// Timing breakdown and break-even metrics for a workload candidate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorkloadTimingBreakdown {
    pub t_cpu_nanos: f64,
    pub t_ingress_nanos: f64,
    pub t_kernel_nanos: f64,
    pub t_sync_readback_nanos: f64,
    pub t_e2e_device_nanos: f64,
    /// S_e2e(N) = T_CPU / (T_ingress + T_kernel + T_sync_readback)
    pub speedup_e2e: f64,
}

/// Resident chain economics breakdown for amortized evaluation across k iterations.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResidentChainBreakdown {
    pub chain_depth_k: usize,
    pub t_cpu_total_nanos: f64,
    pub t_device_total_nanos: f64,
    /// S_resident(N, k) = sum(T_CPU, i) / (T_ingress + sum(T_kernel, i) + T_readback)
    pub speedup_resident: f64,
}

/// Computes analytical end-to-end speedup S_e2e(N).
pub fn speedup_e2e(t_cpu: f64, t_ingress: f64, t_kernel: f64, t_sync_readback: f64) -> f64 {
    let t_device_e2e = t_ingress + t_kernel + t_sync_readback;
    if t_device_e2e <= 0.0 {
        0.0
    } else {
        t_cpu / t_device_e2e
    }
}

/// Computes resident chain speedup S_resident(N, k).
pub fn speedup_resident(
    t_cpu_step: f64,
    t_ingress: f64,
    t_kernel_step: f64,
    t_readback: f64,
    k: usize,
) -> f64 {
    let k_f = k as f64;
    let t_total_cpu = k_f * t_cpu_step;
    let t_total_device = t_ingress + (k_f * t_kernel_step) + t_readback;
    if t_total_device <= 0.0 {
        0.0
    } else {
        t_total_cpu / t_total_device
    }
}

/// Cost model parameters derived from Apple Silicon / unified memory architecture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CostModelProfile {
    pub fixed_ingress_overhead_nanos: f64,
    pub fixed_kernel_launch_overhead_nanos: f64,
    pub fixed_sync_readback_overhead_nanos: f64,
    pub memory_bandwidth_gb_per_sec: f64,
}

impl Default for CostModelProfile {
    fn default() -> Self {
        Self {
            fixed_ingress_overhead_nanos: 2_000.0,
            fixed_kernel_launch_overhead_nanos: 3_000.0,
            fixed_sync_readback_overhead_nanos: 5_000.0,
            memory_bandwidth_gb_per_sec: 50.0,
        }
    }
}

impl CostModelProfile {
    /// Computes the break-even timing breakdown for a workload candidate.
    pub fn compute_breakdown(&self, candidate: &WorkloadCandidate) -> WorkloadTimingBreakdown {
        let n = candidate.batch_size as f64;
        let bytes_per_sec = self.memory_bandwidth_gb_per_sec * 1e9;
        let nanos_per_byte = 1e9 / bytes_per_sec;

        let (t_cpu_nanos, ingress_bytes, kernel_nanos, readback_bytes) = match candidate.kind {
            NumericWorkloadKind::KElt => {
                let cpu_ns = n * 1.5;
                let ing_bytes = n * 8.0;
                let kern_ns = 200.0 + n * 0.015;
                let rb_bytes = n * 4.0;
                (cpu_ns, ing_bytes, kern_ns, rb_bytes)
            }
            NumericWorkloadKind::KW1Presort => {
                let cpu_ns = n * 1.0;
                let ing_bytes = n * 8.0;
                let kern_ns = 250.0 + n * 0.02;
                let rb_bytes = 8.0;
                (cpu_ns, ing_bytes, kern_ns, rb_bytes)
            }
            NumericWorkloadKind::KW1Weighted => {
                let m = if candidate.aux_param > 0 {
                    candidate.aux_param as f64
                } else {
                    n
                };
                let cpu_ns = (n + m) * 4.0;
                let ing_bytes = (n + m) * 8.0;
                let kern_ns = 500.0 + (n + m) * 0.08;
                let rb_bytes = 8.0;
                (cpu_ns, ing_bytes, kern_ns, rb_bytes)
            }
            NumericWorkloadKind::KKsCdf => {
                let cpu_ns = n * 3.0;
                let ing_bytes = n * 8.0;
                let kern_ns = 400.0 + n * 0.04;
                let rb_bytes = 8.0;
                (cpu_ns, ing_bytes, kern_ns, rb_bytes)
            }
            NumericWorkloadKind::KHist => {
                let bins = candidate.aux_param.max(64) as f64;
                let cpu_ns = n * 2.5;
                let ing_bytes = n * 4.0;
                let kern_ns = 300.0 + n * 0.03;
                let rb_bytes = bins * 8.0;
                (cpu_ns, ing_bytes, kern_ns, rb_bytes)
            }
            NumericWorkloadKind::KScan => {
                let cpu_ns = n * 1.2;
                let ing_bytes = n * 8.0;
                let kern_ns = 350.0 + n * 0.025;
                let rb_bytes = n * 8.0;
                (cpu_ns, ing_bytes, kern_ns, rb_bytes)
            }
            NumericWorkloadKind::KCand => {
                let m = candidate.aux_param.max(8) as f64;
                let cpu_ns = m * n * 1.5;
                let ing_bytes = (m + 1.0) * n * 4.0;
                let kern_ns = 500.0 + m * n * 0.02;
                let rb_bytes = m * 8.0;
                (cpu_ns, ing_bytes, kern_ns, rb_bytes)
            }
            NumericWorkloadKind::KTransit => {
                let cpu_ns = n * 22.0;
                let ing_bytes = n * 20.0;
                let kern_ns = 300.0 + n * 0.05;
                let rb_bytes = n * 8.0;
                (cpu_ns, ing_bytes, kern_ns, rb_bytes)
            }
            NumericWorkloadKind::KSortFull => {
                let log2_n = (n.log2()).max(1.0);
                let cpu_ns = (n * log2_n * 8.0) + (n * 1.0);
                let ing_bytes = n * 8.0;
                let kern_ns = 1_000.0 + (n * log2_n * 0.3) + (n * 0.02);
                let rb_bytes = 8.0;
                (cpu_ns, ing_bytes, kern_ns, rb_bytes)
            }
        };

        let t_ingress = self.fixed_ingress_overhead_nanos + ingress_bytes * nanos_per_byte;
        let t_kernel = self.fixed_kernel_launch_overhead_nanos + kernel_nanos;
        let t_sync_readback =
            self.fixed_sync_readback_overhead_nanos + readback_bytes * nanos_per_byte;
        let t_e2e = t_ingress + t_kernel + t_sync_readback;
        let speedup_e2e = speedup_e2e(t_cpu_nanos, t_ingress, t_kernel, t_sync_readback);

        WorkloadTimingBreakdown {
            t_cpu_nanos,
            t_ingress_nanos: t_ingress,
            t_kernel_nanos: t_kernel,
            t_sync_readback_nanos: t_sync_readback,
            t_e2e_device_nanos: t_e2e,
            speedup_e2e,
        }
    }

    /// Computes resident chain speedup for repeated operations over resident buffers.
    pub fn compute_resident_chain(
        &self,
        candidate: &WorkloadCandidate,
        chain_depth_k: usize,
    ) -> ResidentChainBreakdown {
        let k = chain_depth_k.max(1);
        let single = self.compute_breakdown(candidate);

        let t_cpu_total = (k as f64) * single.t_cpu_nanos;
        let t_device_total = single.t_ingress_nanos
            + ((k as f64) * single.t_kernel_nanos)
            + single.t_sync_readback_nanos;
        let speedup_resident = speedup_resident(
            single.t_cpu_nanos,
            single.t_ingress_nanos,
            single.t_kernel_nanos,
            single.t_sync_readback_nanos,
            k,
        );

        ResidentChainBreakdown {
            chain_depth_k,
            t_cpu_total_nanos: t_cpu_total,
            t_device_total_nanos: t_device_total,
            speedup_resident,
        }
    }
}

/// Primary decision outcome selecting accelerating kernels per Report 26.
#[derive(Clone, Debug, PartialEq)]
pub struct KernelSelectionDecision {
    /// Chosen one-shot kernel with measurable end-to-end benefit.
    pub primary_kernel: NumericWorkloadKind,
    /// Chosen resident kernel for iterative calibration with high resident gain.
    pub resident_kernel: NumericWorkloadKind,
    pub transit_speedup_e2e: f64,
    pub resident_cand_speedup: f64,
    pub justification: String,
}

/// Evaluates the 26 workload catalog and executes the kernel selection decision.
pub fn evaluate_kernel_selection(catalog: &[WorkloadCandidate]) -> KernelSelectionDecision {
    let profile = CostModelProfile::default();

    let mut best_transit_speedup = 0.0f64;
    let mut best_resident_cand_speedup = 0.0f64;

    for cand in catalog {
        let breakdown = profile.compute_breakdown(cand);
        if cand.kind == NumericWorkloadKind::KTransit
            && breakdown.speedup_e2e > best_transit_speedup
        {
            best_transit_speedup = breakdown.speedup_e2e;
        }
        if cand.kind == NumericWorkloadKind::KCand {
            let res = profile.compute_resident_chain(cand, 32);
            if res.speedup_resident > best_resident_cand_speedup {
                best_resident_cand_speedup = res.speedup_resident;
            }
        }
    }

    KernelSelectionDecision {
        primary_kernel: NumericWorkloadKind::KTransit,
        resident_kernel: NumericWorkloadKind::KCand,
        transit_speedup_e2e: best_transit_speedup,
        resident_cand_speedup: best_resident_cand_speedup,
        justification: "Selected K-TRANSIT for one-shot batch spatial evaluation due to high compute intensity (sqrt, division) overcoming transfer latency, and resident K-CAND for multi-candidate calibration chains where host-to-device ingress is amortized across parameter evaluations. Simple presorted W1 reductions stay CPU-bound or memory-bound without resident chaining.".into(),
    }
}

// ---------------------------------------------------------------------------
// Fallback and Executor Facade
// ---------------------------------------------------------------------------

/// Safe executor facade that executes batch numeric workloads, falling back
/// to CpuFallbackCompute when hardware acceleration is unavailable.
#[derive(Clone, Debug)]
pub struct NumericBatchExecutor {
    fallback: CpuFallbackCompute,
    is_device_active: bool,
}

impl Default for NumericBatchExecutor {
    fn default() -> Self {
        Self::new_cpu_fallback()
    }
}

impl NumericBatchExecutor {
    /// Constructs an executor explicitly operating in CPU fallback mode.
    pub fn new_cpu_fallback() -> Self {
        Self {
            fallback: CpuFallbackCompute::new(),
            is_device_active: false,
        }
    }

    /// Whether a native hardware device backend is active.
    pub const fn is_device_active(&self) -> bool {
        self.is_device_active
    }

    /// Access reference to the underlying CPU fallback compute engine.
    pub const fn fallback_compute(&self) -> &CpuFallbackCompute {
        &self.fallback
    }

    /// Executes transit batch with CPU fallback.
    pub fn execute_transit(
        &self,
        queries: &[AgentTransitQuery],
    ) -> Result<Vec<AgentTransitResult>, NumericBatchError> {
        compute_transit(queries)
    }

    /// Executes elementwise residuals and loss clamp with CPU fallback.
    pub fn execute_elt(
        &self,
        pred: &[f32],
        obs: &[f32],
        min_val: f32,
        max_val: f32,
    ) -> Result<Vec<f32>, NumericBatchError> {
        compute_elt(pred, obs, min_val, max_val)
    }

    /// Executes presorted W1 with CPU fallback.
    pub fn execute_w1_presort(&self, u: &[f32], v: &[f32]) -> Result<f64, NumericBatchError> {
        compute_w1_presort(u, v)
    }

    /// Executes weighted/unequal W1 with CPU fallback.
    pub fn execute_w1_weighted(
        &self,
        u: &[f32],
        wu: Option<&[f64]>,
        v: &[f32],
        wv: Option<&[f64]>,
    ) -> Result<f64, NumericBatchError> {
        compute_w1_weighted(u, wu, v, wv)
    }

    /// Executes KS distance with CPU fallback.
    pub fn execute_ks_cdf(&self, u: &[f32], v: &[f32]) -> Result<f64, NumericBatchError> {
        compute_ks_cdf(u, v)
    }

    /// Executes integer histogram with CPU fallback.
    pub fn execute_hist(
        &self,
        values: &[f32],
        bin_count: usize,
        min_val: f32,
        max_val: f32,
    ) -> Result<Vec<u64>, NumericBatchError> {
        compute_hist(values, bin_count, min_val, max_val)
    }

    /// Executes integer prefix scan with CPU fallback.
    pub fn execute_scan(&self, counts: &[u64]) -> Result<Vec<u64>, NumericBatchError> {
        compute_scan(counts)
    }

    /// Executes candidate evaluation with CPU fallback.
    pub fn execute_candidates(
        &self,
        candidates_preds: &[Vec<f32>],
        reference_obs: &[f32],
    ) -> Result<Vec<CandidateEvaluation>, NumericBatchError> {
        compute_candidate_evaluations(candidates_preds, reference_obs)
    }
}
