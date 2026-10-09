//! Private empirical intrinsic-work sampling primitives.
//!
//! This module samples service work only. Queue and transit intervals belong
//! to the execution model and are deliberately not represented here.

use crate::seed_map::{
    CalibrationSeedError, CalibrationStream, CalibrationStreamKey, SeedPurpose, validate_id,
};
use kairo_ecs_types::SimDuration;
use std::collections::{HashMap, HashSet};
use std::fmt;
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

/// Owned, crate-private representation used only by the experimental flow
/// checkpoint assembly. Stratum order is canonical; weighted support order is
/// intentionally retained because it participates in seeded selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct IntrinsicWorkProviderCheckpointV1 {
    version: u32,
    strata: Vec<ProviderStratumCheckpointV1>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ProviderStratumCheckpointV1 {
    id: String,
    distribution: ProviderDistributionCheckpointV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ProviderDistributionCheckpointV1 {
    FixedTicks(u128),
    WeightedTicks {
        support: Vec<(u128, u64)>,
        cached_total: u64,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct IntrinsicWorkProviderCheckpointLimits {
    pub(crate) max_strata: usize,
    pub(crate) max_total_support: usize,
    pub(crate) max_identifier_bytes: usize,
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub(crate) enum IntrinsicWorkProviderCheckpointError {
    #[error("unsupported provider checkpoint version")]
    UnsupportedVersion,
    #[error("provider checkpoint is empty or exceeds a configured limit")]
    LimitExceeded,
    #[error("provider checkpoint has invalid identifiers or ordering")]
    InvalidIdentifierOrOrder,
    #[error("provider checkpoint contains invalid distribution data")]
    InvalidDistribution,
    #[error("provider checkpoint allocation failed")]
    Allocation,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct SampledWorkDuration {
    duration: SimDuration,
    key: CalibrationStreamKey,
    draw_before: u64,
    draw_after: u64,
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub(crate) enum SampledWorkDurationCheckpointError {
    #[error("sample checkpoint requires a Service stream")]
    WrongPurpose,
    #[error("sample checkpoint identity does not match the expected stream")]
    IdentityMismatch,
    #[error("sample checkpoint draw bounds or duration are invalid")]
    InvalidSample,
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
            key: expected.clone(),
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

    pub(crate) fn checkpoint_v1(
        &self,
        limits: IntrinsicWorkProviderCheckpointLimits,
    ) -> Result<IntrinsicWorkProviderCheckpointV1, IntrinsicWorkProviderCheckpointError> {
        // Complete aggregate preflight before sorting, hashing, or cloning any
        // identifier/support data into the checkpoint.
        let mut support_count = 0usize;
        let mut identifier_bytes = 0usize;
        for (id, distribution) in &self.strata {
            identifier_bytes = identifier_bytes
                .checked_add(id.len())
                .ok_or(IntrinsicWorkProviderCheckpointError::LimitExceeded)?;
            if let Distribution::Weighted { support, .. } = &distribution.distribution {
                support_count = support_count
                    .checked_add(support.len())
                    .ok_or(IntrinsicWorkProviderCheckpointError::LimitExceeded)?;
            }
        }
        if self.strata.is_empty()
            || self.strata.len() > limits.max_strata
            || support_count > limits.max_total_support
            || identifier_bytes > limits.max_identifier_bytes
        {
            return Err(IntrinsicWorkProviderCheckpointError::LimitExceeded);
        }

        let mut keys = Vec::new();
        keys.try_reserve_exact(self.strata.len())
            .map_err(|_| IntrinsicWorkProviderCheckpointError::Allocation)?;
        keys.extend(self.strata.keys());
        keys.sort_unstable();
        let mut strata = Vec::new();
        strata
            .try_reserve_exact(keys.len())
            .map_err(|_| IntrinsicWorkProviderCheckpointError::Allocation)?;
        for id in keys {
            validate_id("stratum_id", id)
                .map_err(|_| IntrinsicWorkProviderCheckpointError::InvalidIdentifierOrOrder)?;
            let distribution = &self.strata[id].distribution;
            let checkpoint_distribution = match distribution {
                Distribution::Fixed(ticks) => {
                    if *ticks == 0 {
                        return Err(IntrinsicWorkProviderCheckpointError::InvalidDistribution);
                    }
                    ProviderDistributionCheckpointV1::FixedTicks(*ticks)
                }
                Distribution::Weighted { support, total } => {
                    validate_support(support, Some(*total))?;
                    let mut copied = Vec::new();
                    copied
                        .try_reserve_exact(support.len())
                        .map_err(|_| IntrinsicWorkProviderCheckpointError::Allocation)?;
                    copied.extend_from_slice(support);
                    ProviderDistributionCheckpointV1::WeightedTicks {
                        support: copied,
                        cached_total: *total,
                    }
                }
            };
            strata.push(ProviderStratumCheckpointV1 {
                id: id.clone(),
                distribution: checkpoint_distribution,
            });
        }
        Ok(IntrinsicWorkProviderCheckpointV1 {
            version: INTRINSIC_WORK_PROVIDER_VERSION_V1,
            strata,
        })
    }
}

impl IntrinsicWorkProviderCheckpointV1 {
    pub(crate) fn restore(
        self,
        limits: IntrinsicWorkProviderCheckpointLimits,
    ) -> Result<IntrinsicWorkProvider, IntrinsicWorkProviderCheckpointError> {
        if self.version != INTRINSIC_WORK_PROVIDER_VERSION_V1 {
            return Err(IntrinsicWorkProviderCheckpointError::UnsupportedVersion);
        }
        let mut support_count = 0usize;
        let mut identifier_bytes = 0usize;
        for stratum in &self.strata {
            identifier_bytes = identifier_bytes
                .checked_add(stratum.id.len())
                .ok_or(IntrinsicWorkProviderCheckpointError::LimitExceeded)?;
            if let ProviderDistributionCheckpointV1::WeightedTicks { support, .. } =
                &stratum.distribution
            {
                support_count = support_count
                    .checked_add(support.len())
                    .ok_or(IntrinsicWorkProviderCheckpointError::LimitExceeded)?;
            }
        }
        if self.strata.is_empty()
            || self.strata.len() > limits.max_strata
            || support_count > limits.max_total_support
            || identifier_bytes > limits.max_identifier_bytes
        {
            return Err(IntrinsicWorkProviderCheckpointError::LimitExceeded);
        }
        // After aggregate caps, linear uniqueness and canonical-order checks
        // avoid attacker-sized hash table construction and reject aliases.
        for (index, stratum) in self.strata.iter().enumerate() {
            validate_id("stratum_id", &stratum.id)
                .map_err(|_| IntrinsicWorkProviderCheckpointError::InvalidIdentifierOrOrder)?;
            if index > 0 && self.strata[index - 1].id >= stratum.id {
                return Err(IntrinsicWorkProviderCheckpointError::InvalidIdentifierOrOrder);
            }
            if let ProviderDistributionCheckpointV1::WeightedTicks {
                support,
                cached_total,
            } = &stratum.distribution
            {
                validate_support(support, Some(*cached_total))?;
            }
        }
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(self.strata.len())
            .map_err(|_| IntrinsicWorkProviderCheckpointError::Allocation)?;
        for stratum in self.strata {
            let distribution = match stratum.distribution {
                ProviderDistributionCheckpointV1::FixedTicks(ticks) if ticks > 0 => {
                    IntrinsicDurationDistribution {
                        distribution: Distribution::Fixed(ticks),
                    }
                }
                ProviderDistributionCheckpointV1::FixedTicks(_) => {
                    return Err(IntrinsicWorkProviderCheckpointError::InvalidDistribution);
                }
                ProviderDistributionCheckpointV1::WeightedTicks {
                    support,
                    cached_total,
                } => IntrinsicDurationDistribution {
                    distribution: Distribution::Weighted {
                        support,
                        total: cached_total,
                    },
                },
            };
            entries.push((stratum.id, distribution));
        }
        IntrinsicWorkProvider::new(INTRINSIC_WORK_PROVIDER_VERSION_V1, entries)
            .map_err(|_| IntrinsicWorkProviderCheckpointError::InvalidDistribution)
    }
}

fn validate_support(
    support: &[(u128, u64)],
    cached_total: Option<u64>,
) -> Result<(), IntrinsicWorkProviderCheckpointError> {
    if support.is_empty() {
        return Err(IntrinsicWorkProviderCheckpointError::InvalidDistribution);
    }
    let mut seen = HashSet::new();
    seen.try_reserve(support.len())
        .map_err(|_| IntrinsicWorkProviderCheckpointError::Allocation)?;
    let mut total = 0u64;
    for &(ticks, weight) in support {
        if ticks == 0 || weight == 0 {
            return Err(IntrinsicWorkProviderCheckpointError::InvalidDistribution);
        }
        if !seen.insert(ticks) {
            return Err(IntrinsicWorkProviderCheckpointError::InvalidDistribution);
        }
        total = total
            .checked_add(weight)
            .ok_or(IntrinsicWorkProviderCheckpointError::InvalidDistribution)?;
    }
    if total == 0 || cached_total.is_some_and(|cached| cached != total) {
        return Err(IntrinsicWorkProviderCheckpointError::InvalidDistribution);
    }
    Ok(())
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

    /// Borrow the complete owner identity and draw bounds for a private
    /// checkpoint assembler. The opaque key itself remains module-owned.
    pub(crate) fn checkpoint_parts(&self) -> (SimDuration, &CalibrationStreamKey, u64, u64) {
        (self.duration, &self.key, self.draw_before, self.draw_after)
    }

    /// Rebuild a sample only after validating it against the restored exact
    /// Service stream state. This does not sample, advance, or replay draws.
    pub(crate) fn from_checkpoint_parts(
        duration: SimDuration,
        key: CalibrationStreamKey,
        draw_before: u64,
        draw_after: u64,
        stream: &CalibrationStream,
        expected: &CalibrationStreamKey,
    ) -> Result<Self, SampledWorkDurationCheckpointError> {
        if stream.purpose() != SeedPurpose::Service {
            return Err(SampledWorkDurationCheckpointError::WrongPurpose);
        }
        if &key != expected || stream.key() != *expected {
            return Err(SampledWorkDurationCheckpointError::IdentityMismatch);
        }
        if duration.ticks() == 0 || draw_before > draw_after || draw_after > stream.draw_position()
        {
            return Err(SampledWorkDurationCheckpointError::InvalidSample);
        }
        Ok(Self {
            duration,
            key,
            draw_before,
            draw_after,
        })
    }
}

impl fmt::Debug for CalibrationStreamKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CalibrationStreamKey([redacted])")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seed_map::CalibrationSeedMap;

    fn canonical_service_stream() -> (CalibrationStreamKey, CalibrationStream) {
        let mut map = CalibrationSeedMap::new(1, "study-α", 1234).unwrap();
        let key = map
            .key_for("crn-v1", 7, "case-0001", "triage:1", SeedPurpose::Service)
            .unwrap();
        let stream = map
            .stream_for("crn-v1", 7, "case-0001", "triage:1", SeedPurpose::Service)
            .unwrap();
        (key, stream)
    }

    #[test]
    fn sampled_duration_retains_opaque_identity_without_debug_leakage() {
        fn sample_for(case: &str, task: &str) -> (SampledWorkDuration, CalibrationStreamKey) {
            let mut map = CalibrationSeedMap::new(1, "private-study-sentinel", 1234).unwrap();
            let key = map
                .key_for("same-schedule", 7, case, task, SeedPurpose::Service)
                .unwrap();
            let mut stream = map
                .stream_for("same-schedule", 7, case, task, SeedPurpose::Service)
                .unwrap();
            let sample = IntrinsicDurationDistribution::fixed(30)
                .unwrap()
                .sample(&mut stream, &key)
                .unwrap();
            (sample, key)
        }

        let (first, first_key) = sample_for("private-case-one", "private-task-one");
        let (second, second_key) = sample_for("private-case-two", "private-task-two");
        assert_eq!(first.duration(), second.duration());
        assert_eq!(first.draw_before(), second.draw_before());
        assert_eq!(first.draw_after(), second.draw_after());
        assert_eq!(first.key, first_key);
        assert_eq!(second.key, second_key);
        assert_ne!(first.key, second.key);

        let debug = format!("{first:?}");
        for private_id in [
            "private-study-sentinel",
            "private-case-one",
            "private-task-one",
        ] {
            assert!(!debug.contains(private_id));
        }
        assert!(debug.contains("[redacted]"));
    }

    #[test]
    fn sample_checkpoint_parts_rebuild_without_consuming_rng() {
        let (key, mut stream) = canonical_service_stream();
        let fixed = IntrinsicDurationDistribution::fixed(12)
            .unwrap()
            .sample(&mut stream, &key)
            .unwrap();
        let (duration, stored_key, before, after) = fixed.checkpoint_parts();
        assert_eq!((before, after), (0, 0));
        // The sample interval is historical metadata. The owning stream can
        // continue after sampling; restoring the sample must not rewind it.
        stream.next_u64().unwrap();
        let restored_stream = stream.snapshot().restore().unwrap();
        let rebuilt = SampledWorkDuration::from_checkpoint_parts(
            duration,
            stored_key.clone(),
            before,
            after,
            &restored_stream,
            &key,
        )
        .unwrap();
        assert_eq!(rebuilt.duration(), fixed.duration());
        assert_eq!(
            rebuilt.checkpoint_parts().2..=rebuilt.checkpoint_parts().3,
            0..=0
        );
        assert_eq!(restored_stream.draw_position(), 1);
        assert_eq!(stream.draw_position(), 1);
    }

    #[test]
    fn sample_checkpoint_rejects_invalid_owner_purpose_duration_and_draw_bounds() {
        let (key, mut stream) = canonical_service_stream();
        let weighted = IntrinsicDurationDistribution::weighted_ticks(vec![(4, 2), (9, 3)])
            .unwrap()
            .sample(&mut stream, &key)
            .unwrap();
        let (duration, stored_key, before, after) = weighted.checkpoint_parts();
        let restored = stream.snapshot().restore().unwrap();
        assert!(
            SampledWorkDuration::from_checkpoint_parts(
                duration,
                stored_key.clone(),
                before,
                after,
                &restored,
                &key,
            )
            .is_ok()
        );

        assert_eq!(
            SampledWorkDuration::from_checkpoint_parts(
                SimDuration::from_ticks(0),
                stored_key.clone(),
                before,
                after,
                &restored,
                &key,
            ),
            Err(SampledWorkDurationCheckpointError::InvalidSample)
        );
        assert_eq!(
            SampledWorkDuration::from_checkpoint_parts(
                duration,
                stored_key.clone(),
                after,
                before,
                &restored,
                &key,
            ),
            Err(SampledWorkDurationCheckpointError::InvalidSample)
        );
        assert_eq!(
            SampledWorkDuration::from_checkpoint_parts(
                duration,
                stored_key.clone(),
                before,
                restored.draw_position() + 1,
                &restored,
                &key,
            ),
            Err(SampledWorkDurationCheckpointError::InvalidSample)
        );

        let mut map = CalibrationSeedMap::new(1, "study-α", 1234).unwrap();
        let other_key = map
            .key_for("crn-v1", 7, "other-case", "triage:1", SeedPurpose::Service)
            .unwrap();
        assert_eq!(
            SampledWorkDuration::from_checkpoint_parts(
                duration, other_key, before, after, &restored, &key,
            ),
            Err(SampledWorkDurationCheckpointError::IdentityMismatch)
        );

        let behavior = map
            .stream_for("crn-v1", 7, "case-0001", "triage:1", SeedPurpose::Behavior)
            .unwrap();
        assert_eq!(
            SampledWorkDuration::from_checkpoint_parts(
                duration,
                stored_key.clone(),
                before,
                after,
                &behavior,
                &key,
            ),
            Err(SampledWorkDurationCheckpointError::WrongPurpose)
        );
    }

    #[test]
    fn weighted_sampling_matches_canonical_golden_sequence_and_draw_positions() {
        let (key, mut stream) = canonical_service_stream();
        let distribution =
            IntrinsicDurationDistribution::weighted_ticks(vec![(10, 3), (20, 2), (30, 5)]).unwrap();

        let samples: Vec<_> = (0..3)
            .map(|_| distribution.sample(&mut stream, &key).unwrap())
            .collect();

        assert_eq!(
            samples
                .iter()
                .map(|sample| sample.duration().ticks())
                .collect::<Vec<_>>(),
            vec![30, 10, 20]
        );
        assert_eq!(
            samples
                .iter()
                .map(|sample| (sample.draw_before(), sample.draw_after()))
                .collect::<Vec<_>>(),
            vec![(0, 1), (1, 2), (2, 3)]
        );
        assert_eq!(stream.draw_position(), 3);
    }

    #[test]
    fn weighted_sampling_preserves_support_order_and_assigns_boundary_to_next_bucket() {
        let (key, mut reversed_stream) = canonical_service_stream();
        let reversed =
            IntrinsicDurationDistribution::weighted_ticks(vec![(30, 5), (20, 2), (10, 3)]).unwrap();
        let reversed_samples: Vec<_> = (0..3)
            .map(|_| {
                reversed
                    .sample(&mut reversed_stream, &key)
                    .unwrap()
                    .duration()
                    .ticks()
            })
            .collect();
        assert_eq!(reversed_samples, vec![10, 30, 30]);

        // The canonical stream's first accepted value has residue 7 modulo 10.
        // Since residue 7 is the first index after the weight-7 bucket, it must
        // select the second support item.
        let (key, mut boundary_stream) = canonical_service_stream();
        let boundary =
            IntrinsicDurationDistribution::weighted_ticks(vec![(10, 7), (20, 3)]).unwrap();
        let sample = boundary.sample(&mut boundary_stream, &key).unwrap();
        assert_eq!(sample.duration().ticks(), 20);
        assert_eq!((sample.draw_before(), sample.draw_after()), (0, 1));
    }

    #[test]
    fn fixed_u128_max_preserves_full_width_and_consumes_no_draws() {
        let (key, mut stream) = canonical_service_stream();
        let sample = IntrinsicDurationDistribution::fixed(u128::MAX)
            .unwrap()
            .sample(&mut stream, &key)
            .unwrap();

        assert_eq!(sample.duration().ticks(), u128::MAX);
        assert_eq!((sample.draw_before(), sample.draw_after()), (0, 0));
        assert_eq!(stream.draw_position(), 0);
    }

    #[test]
    fn provider_rejects_typed_constructor_and_lookup_errors() {
        assert_eq!(
            IntrinsicDurationDistribution::fixed(0).err().unwrap(),
            WorkDurationError::ZeroDuration
        );
        assert_eq!(
            IntrinsicDurationDistribution::weighted_ticks(vec![])
                .err()
                .unwrap(),
            WorkDurationError::EmptySupport
        );
        assert_eq!(
            IntrinsicDurationDistribution::weighted_ticks(vec![(0, 1)])
                .err()
                .unwrap(),
            WorkDurationError::ZeroDuration
        );
        assert_eq!(
            IntrinsicDurationDistribution::weighted_ticks(vec![(1, 0)])
                .err()
                .unwrap(),
            WorkDurationError::ZeroWeight
        );
        assert_eq!(
            IntrinsicDurationDistribution::weighted_ticks(vec![(1, 1), (1, 2)])
                .err()
                .unwrap(),
            WorkDurationError::DuplicateDuration
        );
        assert_eq!(
            IntrinsicDurationDistribution::weighted_ticks(vec![(1, u64::MAX), (2, 1)])
                .err()
                .unwrap(),
            WorkDurationError::WeightOverflow
        );
        assert_eq!(
            IntrinsicWorkProvider::new(99, vec![]).err().unwrap(),
            WorkDurationError::UnsupportedProviderVersion(99)
        );
        assert_eq!(
            IntrinsicWorkProvider::new(INTRINSIC_WORK_PROVIDER_VERSION_V1, vec![])
                .err()
                .unwrap(),
            WorkDurationError::EmptySupport
        );
        assert_eq!(
            IntrinsicWorkProvider::new(
                INTRINSIC_WORK_PROVIDER_VERSION_V1,
                vec![(
                    "bad\nstratum".into(),
                    IntrinsicDurationDistribution::fixed(1).unwrap()
                )]
            )
            .err()
            .unwrap(),
            WorkDurationError::InvalidStratum
        );
        assert_eq!(
            IntrinsicWorkProvider::new(
                INTRINSIC_WORK_PROVIDER_VERSION_V1,
                vec![
                    (
                        "same".into(),
                        IntrinsicDurationDistribution::fixed(1).unwrap()
                    ),
                    (
                        "same".into(),
                        IntrinsicDurationDistribution::fixed(2).unwrap()
                    ),
                ]
            )
            .err()
            .unwrap(),
            WorkDurationError::DuplicateStratum
        );

        let provider = IntrinsicWorkProvider::new(
            INTRINSIC_WORK_PROVIDER_VERSION_V1,
            vec![(
                "triage".into(),
                IntrinsicDurationDistribution::fixed(1).unwrap(),
            )],
        )
        .unwrap();
        let (key, mut stream) = canonical_service_stream();
        let mut control = stream.snapshot().restore_for(&key).unwrap();
        assert_eq!(
            provider.sample("missing", &mut stream, &key).err().unwrap(),
            WorkDurationError::MissingStratum
        );
        assert_eq!(
            provider
                .sample("bad\nstratum", &mut stream, &key)
                .err()
                .unwrap(),
            WorkDurationError::InvalidStratum
        );
        assert_eq!(stream.next_u64().unwrap(), control.next_u64().unwrap());
    }

    #[test]
    fn provider_validation_precedence_is_deterministic() {
        let fixed = || IntrinsicDurationDistribution::fixed(1).unwrap();

        assert_eq!(
            IntrinsicWorkProvider::new(99, vec![("bad\nstratum".into(), fixed())],)
                .err()
                .unwrap(),
            WorkDurationError::UnsupportedProviderVersion(99)
        );

        assert_eq!(
            IntrinsicWorkProvider::new(
                INTRINSIC_WORK_PROVIDER_VERSION_V1,
                vec![
                    ("same".into(), fixed()),
                    ("same".into(), fixed()),
                    ("bad\nstratum".into(), fixed()),
                ],
            )
            .err()
            .unwrap(),
            WorkDurationError::InvalidStratum
        );

        assert_eq!(
            IntrinsicDurationDistribution::weighted_ticks(vec![(0, 0)])
                .err()
                .unwrap(),
            WorkDurationError::ZeroDuration
        );
        assert_eq!(
            IntrinsicDurationDistribution::weighted_ticks(vec![(1, 1), (1, 0)])
                .err()
                .unwrap(),
            WorkDurationError::ZeroWeight
        );
        assert_eq!(
            IntrinsicDurationDistribution::weighted_ticks(vec![(1, u64::MAX), (1, 1)])
                .err()
                .unwrap(),
            WorkDurationError::DuplicateDuration
        );
        assert_eq!(
            IntrinsicDurationDistribution::weighted_ticks(vec![(1, u64::MAX), (2, 1)])
                .err()
                .unwrap(),
            WorkDurationError::WeightOverflow
        );
    }

    #[test]
    fn rejected_real_stream_draws_are_included_in_sample_positions() {
        let mut map = CalibrationSeedMap::new(1, "study-α", 1234).unwrap();
        let key = map
            .key_for("reject-4", 7, "case-0001", "triage:1", SeedPurpose::Service)
            .unwrap();
        let mut stream = map
            .stream_for("reject-4", 7, "case-0001", "triage:1", SeedPurpose::Service)
            .unwrap();
        let mut oracle = stream.snapshot().restore_for(&key).unwrap();
        let weight = 0x8000_0000_0000_0001_u64;
        let threshold = weight.wrapping_neg() % weight;
        let mut rejected = 0;
        loop {
            if oracle.next_u64().unwrap() >= threshold {
                break;
            }
            rejected += 1;
        }
        assert!(
            rejected > 0,
            "the fixed real-stream fixture must reject a draw"
        );

        let sample = IntrinsicDurationDistribution::weighted_ticks(vec![(7, weight)])
            .unwrap()
            .sample(&mut stream, &key)
            .unwrap();

        assert_eq!(sample.duration().ticks(), 7);
        assert_eq!(sample.draw_before(), 0);
        assert_eq!(sample.draw_after(), rejected + 1);
        assert_eq!(stream.draw_position(), oracle.draw_position());
        assert_eq!(stream.next_u64().unwrap(), oracle.next_u64().unwrap());
    }

    #[test]
    fn invalid_purpose_and_identity_errors_preserve_stream_continuation() {
        let (service_key, service) = canonical_service_stream();
        let fixed = IntrinsicDurationDistribution::fixed(u128::MAX).unwrap();

        let mut map = CalibrationSeedMap::new(1, "study-α", 1234).unwrap();
        let transit_key = map
            .key_for("crn-v1", 7, "case-0001", "triage:1", SeedPurpose::Transit)
            .unwrap();
        let mut transit = map
            .stream_for("crn-v1", 7, "case-0001", "triage:1", SeedPurpose::Transit)
            .unwrap();
        let mut transit_control = transit.snapshot().restore_for(&transit_key).unwrap();
        assert_eq!(
            fixed.sample(&mut transit, &service_key).err().unwrap(),
            WorkDurationError::WrongPurpose
        );
        assert_eq!(
            transit.next_u64().unwrap(),
            transit_control.next_u64().unwrap()
        );

        let other_key = map
            .key_for("crn-v1", 7, "case-0001", "other-task", SeedPurpose::Service)
            .unwrap();
        let mut other = map
            .stream_for("crn-v1", 7, "case-0001", "other-task", SeedPurpose::Service)
            .unwrap();
        let mut other_control = other.snapshot().restore_for(&other_key).unwrap();
        assert_eq!(
            fixed.sample(&mut other, &service_key).err().unwrap(),
            WorkDurationError::IdentityMismatch
        );
        assert_eq!(other.next_u64().unwrap(), other_control.next_u64().unwrap());
        assert_eq!(service.draw_position(), 0);
    }

    fn checkpoint_limits() -> IntrinsicWorkProviderCheckpointLimits {
        IntrinsicWorkProviderCheckpointLimits {
            max_strata: 8,
            max_total_support: 32,
            max_identifier_bytes: 128,
        }
    }

    fn mixed_provider() -> IntrinsicWorkProvider {
        IntrinsicWorkProvider::new(
            INTRINSIC_WORK_PROVIDER_VERSION_V1,
            vec![
                (
                    "z-fixed".into(),
                    IntrinsicDurationDistribution::fixed(11).unwrap(),
                ),
                (
                    "a-weighted".into(),
                    IntrinsicDurationDistribution::weighted_ticks(vec![(31, 2), (17, 5), (23, 1)])
                        .unwrap(),
                ),
            ],
        )
        .unwrap()
    }

    #[test]
    fn checkpoint_restores_canonical_strata_and_exact_draw_behavior() {
        let provider = mixed_provider();
        let image = provider.checkpoint_v1(checkpoint_limits()).unwrap();
        assert_eq!(image.strata[0].id, "a-weighted");
        assert_eq!(image.strata[1].id, "z-fixed");
        let restored = image.restore(checkpoint_limits()).unwrap();
        for stratum in ["a-weighted", "z-fixed", "a-weighted", "z-fixed"] {
            let (key1, mut stream1) = canonical_service_stream();
            let (key2, mut stream2) = canonical_service_stream();
            let original = provider.sample(stratum, &mut stream1, &key1).unwrap();
            let replay = restored.sample(stratum, &mut stream2, &key2).unwrap();
            assert_eq!(original.duration(), replay.duration());
            assert_eq!(original.draw_before(), replay.draw_before());
            assert_eq!(original.draw_after(), replay.draw_after());
            assert_eq!(stream1.draw_position(), stream2.draw_position());
            assert_eq!(stream1.next_u64().unwrap(), stream2.next_u64().unwrap());
        }
    }

    #[test]
    fn checkpoint_rejects_version_order_duplicates_and_invalid_distributions() {
        let image = mixed_provider().checkpoint_v1(checkpoint_limits()).unwrap();
        let mut bad = image.clone();
        bad.version = 99;
        assert!(matches!(
            bad.restore(checkpoint_limits()),
            Err(IntrinsicWorkProviderCheckpointError::UnsupportedVersion)
        ));

        let mut bad = image.clone();
        bad.strata.swap(0, 1);
        assert!(matches!(
            bad.restore(checkpoint_limits()),
            Err(IntrinsicWorkProviderCheckpointError::InvalidIdentifierOrOrder)
        ));

        let mut bad = image.clone();
        bad.strata[1].id = bad.strata[0].id.clone();
        assert!(matches!(
            bad.restore(checkpoint_limits()),
            Err(IntrinsicWorkProviderCheckpointError::InvalidIdentifierOrOrder)
        ));

        let mut bad = image.clone();
        if let ProviderDistributionCheckpointV1::WeightedTicks { cached_total, .. } =
            &mut bad.strata[0].distribution
        {
            *cached_total += 1;
        }
        assert!(matches!(
            bad.restore(checkpoint_limits()),
            Err(IntrinsicWorkProviderCheckpointError::InvalidDistribution)
        ));

        let mut bad = image.clone();
        if let ProviderDistributionCheckpointV1::WeightedTicks { support, .. } =
            &mut bad.strata[0].distribution
        {
            support.push(support[0]);
        }
        assert!(matches!(
            bad.restore(checkpoint_limits()),
            Err(IntrinsicWorkProviderCheckpointError::InvalidDistribution)
        ));

        let mut bad = image.clone();
        bad.strata[1].id.clear();
        assert!(matches!(
            bad.restore(checkpoint_limits()),
            Err(IntrinsicWorkProviderCheckpointError::InvalidIdentifierOrOrder)
        ));

        let mut bad = image.clone();
        if let ProviderDistributionCheckpointV1::WeightedTicks {
            support,
            cached_total,
        } = &mut bad.strata[0].distribution
        {
            support[0].0 = 0;
            *cached_total -= support[0].1;
        }
        assert!(matches!(
            bad.restore(checkpoint_limits()),
            Err(IntrinsicWorkProviderCheckpointError::InvalidDistribution)
        ));

        let mut bad = image.clone();
        if let ProviderDistributionCheckpointV1::WeightedTicks {
            support,
            cached_total,
        } = &mut bad.strata[0].distribution
        {
            support.clear();
            *cached_total = 0;
        }
        assert!(matches!(
            bad.restore(checkpoint_limits()),
            Err(IntrinsicWorkProviderCheckpointError::InvalidDistribution)
        ));

        let mut bad = image;
        if let ProviderDistributionCheckpointV1::WeightedTicks {
            support,
            cached_total,
        } = &mut bad.strata[0].distribution
        {
            support.clear();
            support.extend([(1, u64::MAX), (2, 1)]);
            *cached_total = u64::MAX;
        }
        assert!(matches!(
            bad.restore(checkpoint_limits()),
            Err(IntrinsicWorkProviderCheckpointError::InvalidDistribution)
        ));
    }

    #[test]
    fn checkpoint_aggregate_limits_apply_before_restore_and_capture() {
        let provider = mixed_provider();
        let too_few_strata = IntrinsicWorkProviderCheckpointLimits {
            max_strata: 1,
            ..checkpoint_limits()
        };
        assert_eq!(
            provider.checkpoint_v1(too_few_strata),
            Err(IntrinsicWorkProviderCheckpointError::LimitExceeded)
        );
        let image = provider.checkpoint_v1(checkpoint_limits()).unwrap();
        assert!(matches!(
            image.clone().restore(too_few_strata),
            Err(IntrinsicWorkProviderCheckpointError::LimitExceeded)
        ));
        let too_little_support = IntrinsicWorkProviderCheckpointLimits {
            max_total_support: 2,
            ..checkpoint_limits()
        };
        assert_eq!(
            provider.checkpoint_v1(too_little_support),
            Err(IntrinsicWorkProviderCheckpointError::LimitExceeded)
        );
        let too_few_identifier_bytes = IntrinsicWorkProviderCheckpointLimits {
            max_identifier_bytes: 2,
            ..checkpoint_limits()
        };
        assert_eq!(
            provider.checkpoint_v1(too_few_identifier_bytes),
            Err(IntrinsicWorkProviderCheckpointError::LimitExceeded)
        );
    }
}
