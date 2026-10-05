//! Private empirical intrinsic-work sampling primitives.
//!
//! This module samples service work only. Queue and transit intervals belong
//! to the execution model and are deliberately not represented here.

use crate::seed_map::{
    validate_id, CalibrationSeedError, CalibrationStream, CalibrationStreamKey, SeedPurpose,
};
use kairo_ecs_types::SimDuration;
use std::collections::{HashMap, HashSet};
use thiserror::Error;

pub(crate) const INTRINSIC_WORK_PROVIDER_VERSION_V1: u32 = 1;

#[derive(Clone)]
enum Distribution {
    Fixed(u128),
    Weighted {
        support: Vec<(u128, u64)>,
        total: u64,
    },
}

pub(crate) struct IntrinsicDurationDistribution {
    distribution: Distribution,
}

pub(crate) struct IntrinsicWorkProvider {
    strata: HashMap<String, IntrinsicDurationDistribution>,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct SampledWorkDuration {
    duration: SimDuration,
    draw_before: u64,
    draw_after: u64,
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub(crate) enum WorkDurationError {
    #[error("unsupported intrinsic work provider version {0}")]
    UnsupportedProviderVersion(u32),
    #[error("intrinsic work support is empty")]
    EmptySupport,
    #[error("intrinsic work duration must be positive")]
    ZeroDuration,
    #[error("intrinsic work weight must be positive")]
    ZeroWeight,
    #[error("intrinsic work support contains a duplicate duration")]
    DuplicateDuration,
    #[error("intrinsic work weight sum overflow")]
    WeightOverflow,
    #[error("intrinsic work stratum identifier is invalid")]
    InvalidStratum,
    #[error("intrinsic work strata contain a duplicate identifier")]
    DuplicateStratum,
    #[error("intrinsic work stratum is missing")]
    MissingStratum,
    #[error("intrinsic work requires a Service stream")]
    WrongPurpose,
    #[error("intrinsic work stream identity does not match the expected owner")]
    IdentityMismatch,
    #[error("calibration seed stream operation failed")]
    Seed(CalibrationSeedError),
}

impl IntrinsicDurationDistribution {
    pub(crate) fn fixed(ticks: u128) -> Result<Self, WorkDurationError> {
        if ticks == 0 {
            return Err(WorkDurationError::ZeroDuration);
        }
        Ok(Self {
            distribution: Distribution::Fixed(ticks),
        })
    }

    pub(crate) fn weighted_ticks(support: Vec<(u128, u64)>) -> Result<Self, WorkDurationError> {
        if support.is_empty() {
            return Err(WorkDurationError::EmptySupport);
        }

        let mut seen_durations = HashSet::with_capacity(support.len());
        let mut total = 0_u64;
        for &(duration, weight) in &support {
            if duration == 0 {
                return Err(WorkDurationError::ZeroDuration);
            }
            if weight == 0 {
                return Err(WorkDurationError::ZeroWeight);
            }
            if !seen_durations.insert(duration) {
                return Err(WorkDurationError::DuplicateDuration);
            }
            total = total
                .checked_add(weight)
                .ok_or(WorkDurationError::WeightOverflow)?;
        }

        Ok(Self {
            distribution: Distribution::Weighted { support, total },
        })
    }

    pub(crate) fn sample(
        &self,
        stream: &mut CalibrationStream,
        expected: &CalibrationStreamKey,
    ) -> Result<SampledWorkDuration, WorkDurationError> {
        if stream.purpose() != SeedPurpose::Service {
            return Err(WorkDurationError::WrongPurpose);
        }
        if stream.key() != expected.clone() {
            return Err(WorkDurationError::IdentityMismatch);
        }

        let mut candidate = stream
            .snapshot()
            .restore_for(expected)
            .map_err(WorkDurationError::Seed)?;
        let draw_before = candidate.draw_position();
        let duration = match &self.distribution {
            Distribution::Fixed(ticks) => *ticks,
            Distribution::Weighted { support, total } => {
                let threshold = total.wrapping_neg() % total;
                let residue = loop {
                    let value = candidate.next_u64().map_err(WorkDurationError::Seed)?;
                    if value >= threshold {
                        break value % total;
                    }
                };
                let mut cumulative = 0_u64;
                let mut selected = None;
                for &(ticks, weight) in support {
                    cumulative += weight;
                    if residue < cumulative {
                        selected = Some(ticks);
                        break;
                    }
                }
                selected.expect("positive validated weights cover every residue")
            }
        };
        let sample = SampledWorkDuration {
            duration: SimDuration::from_ticks(duration),
            draw_before,
            draw_after: candidate.draw_position(),
        };
        *stream = candidate;
        Ok(sample)
    }
}

impl IntrinsicWorkProvider {
    pub(crate) fn new(
        version: u32,
        strata: Vec<(String, IntrinsicDurationDistribution)>,
    ) -> Result<Self, WorkDurationError> {
        if version != INTRINSIC_WORK_PROVIDER_VERSION_V1 {
            return Err(WorkDurationError::UnsupportedProviderVersion(version));
        }
        if strata.is_empty() {
            return Err(WorkDurationError::EmptySupport);
        }

        for (id, _) in &strata {
            validate_id("stratum_id", id).map_err(|_| WorkDurationError::InvalidStratum)?;
        }

        let mut by_id = HashMap::with_capacity(strata.len());
        for (id, distribution) in strata {
            if by_id.insert(id, distribution).is_some() {
                return Err(WorkDurationError::DuplicateStratum);
            }
        }
        Ok(Self { strata: by_id })
    }

    pub(crate) fn sample(
        &self,
        stratum: &str,
        stream: &mut CalibrationStream,
        expected: &CalibrationStreamKey,
    ) -> Result<SampledWorkDuration, WorkDurationError> {
        validate_id("stratum_id", stratum).map_err(|_| WorkDurationError::InvalidStratum)?;
        let distribution = self
            .strata
            .get(stratum)
            .ok_or(WorkDurationError::MissingStratum)?;
        distribution.sample(stream, expected)
    }
}

impl SampledWorkDuration {
    pub(crate) fn duration(&self) -> SimDuration {
        self.duration
    }

    pub(crate) fn draw_before(&self) -> u64 {
        self.draw_before
    }

    pub(crate) fn draw_after(&self) -> u64 {
        self.draw_after
    }
}
