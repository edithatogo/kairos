//! Private empirical intrinsic-work sampling primitives.
//!
//! This module samples service work only. Queue and transit intervals belong
//! to the execution model and are deliberately not represented here.

use crate::seed_map::{
    validate_id, CalibrationSeedError, CalibrationStream, CalibrationStreamKey, SeedPurpose,
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

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct SampledWorkDuration {
    duration: SimDuration,
    key: CalibrationStreamKey,
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
}
