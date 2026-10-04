//! Pure temporal checks for the opt-in calibration trace boundary.
//!
//! This module accepts already-resolved UTC nanoseconds. It does not parse
//! calendar strings, infer timezone rules, or decide clinical source mappings.
//! Callers must retain raw values and turn every [`TraceTimeResult::Excluded`]
//! into a counted trace exclusion; an ambiguous value is never replaced with an
//! invented instant. This is separate from the legacy `event_log.v1` smoke API.

/// Lossless normalized UTC storage companions; source parsing stays separate.
pub mod utc_codec;

/// Stable reason codes for rejecting temporal or source-semantic input.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum TemporalError {
    #[error("occurrence precedes origin")]
    PreOrigin,
    #[error("temporal arithmetic overflow")]
    Overflow,
    #[error("missing occurrence time")]
    MissingOccurrence,
    #[error("unresolved local time")]
    UnresolvedLocal,
    #[error("ambiguous local time")]
    AmbiguousLocal,
    #[error("nonexistent local time")]
    NonexistentLocal,
    #[error("unsupported coarse precision")]
    CoarsePrecision,
    #[error("sub-nanosecond precision")]
    SubNanosecond,
    #[error("clock role mismatch")]
    ClockRoleMismatch,
    #[error("unverified physical movement")]
    UnsupportedMovement,
    #[error("missing temporal lineage")]
    MissingLineage,
    #[error("reversed location interval")]
    ReversedInterval,
}

/// Explicit accepted/excluded result so invalid source data can be counted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceTimeResult<T> {
    Accepted(T),
    Excluded(TemporalError),
}

/// Clock represented by a timestamp field.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClockRole {
    Occurrence,
    SourceRecorded,
    MessageCreated,
    /// Explicit time at which source information became available.
    KnowledgeAvailable,
}

/// Declared source lineage. `Unknown` is an explicit value; absent lineage is
/// rejected separately.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Lineage {
    Observed,
    Derived,
    Defaulted,
    Unknown,
}

/// Precision carried from the source. No precision is synthesized here.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourcePrecision {
    Nanosecond,
    Microsecond,
    Millisecond,
    Second,
    Minute,
    Coarse,
    Unknown,
}

/// Timestamp resolution supplied by an upstream parser or source adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimeValue {
    ResolvedUtc(i128),
    UnresolvedLocal,
    AmbiguousLocal,
    NonexistentLocal,
    DateOnly,
    SubNanosecond,
}

/// A timestamp plus its source clock role, precision, and explicit lineage.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimestampInput {
    pub role: ClockRole,
    pub lineage: Option<Lineage>,
    pub precision: SourcePrecision,
    pub value: TimeValue,
}

/// Validated occurrence time. `relative_ticks` are nanoseconds from the
/// dataset origin; `precision` retains the source's declared precision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NormalizedOccurrence {
    pub utc_nanoseconds: i128,
    pub relative_ticks: u128,
    pub precision: SourcePrecision,
    pub lineage: Lineage,
}

/// Validated supplied timestamp, retaining its declared clock role.
/// Optional absence remains the source adapter's responsibility.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NormalizedTimestamp {
    pub role: ClockRole,
    pub utc_nanoseconds: i128,
    pub relative_ticks: u128,
    pub precision: SourcePrecision,
    pub lineage: Lineage,
}

/// Convert resolved UTC nanoseconds to nonnegative ticks relative to origin.
///
/// No rounding or saturation is permitted. Calendar/timezone resolution must
/// happen before this function is called.
pub fn relative_ticks(origin_utc_ns: i128, occurrence_utc_ns: i128) -> Result<u128, TemporalError> {
    if occurrence_utc_ns < origin_utc_ns {
        return Err(TemporalError::PreOrigin);
    }

    let sign_bit = 1u128 << 127;
    let origin_ordered = (origin_utc_ns as u128) ^ sign_bit;
    let occurrence_ordered = (occurrence_utc_ns as u128) ^ sign_bit;
    occurrence_ordered
        .checked_sub(origin_ordered)
        .ok_or(TemporalError::Overflow)
}

/// Validate a required occurrence timestamp, preserving source precision and
/// mapping invalid or ambiguous source values to an explicit exclusion reason.
pub fn normalize_occurrence(
    origin_utc_ns: i128,
    input: Option<TimestampInput>,
) -> TraceTimeResult<NormalizedOccurrence> {
    let Some(input) = input else {
        return TraceTimeResult::Excluded(TemporalError::MissingOccurrence);
    };
    if input.role != ClockRole::Occurrence {
        return TraceTimeResult::Excluded(TemporalError::ClockRoleMismatch);
    }
    match normalize_timestamp(origin_utc_ns, input) {
        TraceTimeResult::Accepted(timestamp) => TraceTimeResult::Accepted(NormalizedOccurrence {
            utc_nanoseconds: timestamp.utc_nanoseconds,
            relative_ticks: timestamp.relative_ticks,
            precision: timestamp.precision,
            lineage: timestamp.lineage,
        }),
        TraceTimeResult::Excluded(reason) => TraceTimeResult::Excluded(reason),
    }
}

/// Validate a supplied timestamp without substituting or rejecting its role.
///
/// Preserves explicit source lineage and precision, including minute precision.
/// Already-resolved UTC values use the same exact relative arithmetic as the
/// required occurrence wrapper. Classified local/date/subnanosecond values
/// remain exclusions; this function does not parse or resolve them.
pub fn normalize_timestamp(
    origin_utc_ns: i128,
    input: TimestampInput,
) -> TraceTimeResult<NormalizedTimestamp> {
    let Some(lineage) = input.lineage else {
        return TraceTimeResult::Excluded(TemporalError::MissingLineage);
    };
    if matches!(
        input.precision,
        SourcePrecision::Coarse | SourcePrecision::Unknown
    ) {
        return TraceTimeResult::Excluded(TemporalError::CoarsePrecision);
    }

    let utc_nanoseconds = match input.value {
        TimeValue::ResolvedUtc(value) => value,
        TimeValue::UnresolvedLocal => {
            return TraceTimeResult::Excluded(TemporalError::UnresolvedLocal)
        }
        TimeValue::AmbiguousLocal => {
            return TraceTimeResult::Excluded(TemporalError::AmbiguousLocal)
        }
        TimeValue::NonexistentLocal => {
            return TraceTimeResult::Excluded(TemporalError::NonexistentLocal)
        }
        TimeValue::DateOnly => {
            return TraceTimeResult::Excluded(TemporalError::CoarsePrecision);
        }
        TimeValue::SubNanosecond => {
            return TraceTimeResult::Excluded(TemporalError::SubNanosecond);
        }
    };
    let relative_ticks = match relative_ticks(origin_utc_ns, utc_nanoseconds) {
        Ok(ticks) => ticks,
        Err(reason) => return TraceTimeResult::Excluded(reason),
    };

    TraceTimeResult::Accepted(NormalizedTimestamp {
        role: input.role,
        utc_nanoseconds,
        relative_ticks,
        precision: input.precision,
        lineage,
    })
}

/// Event kinds relevant to physical-movement claims. Episode end and physical
/// departure remain separate; one never implies the other.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventSemantic {
    EpisodeEnd,
    PhysicalDeparture,
    Boarding,
    LocationChange,
    Other,
}

/// Evidence that a mapped event has a verified physical boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MovementEvidence {
    Unverified,
    VerifiedPhysicalBoundary,
}

/// Accept only source-defined physical departures or location changes whose
/// local boundary semantics have been verified. Administrative events and
/// episode-end events alone do not establish physical movement.
pub fn validate_movement_claim(
    semantic: EventSemantic,
    evidence: MovementEvidence,
) -> TraceTimeResult<()> {
    match (semantic, evidence) {
        (
            EventSemantic::PhysicalDeparture | EventSemantic::LocationChange,
            MovementEvidence::VerifiedPhysicalBoundary,
        ) => TraceTimeResult::Accepted(()),
        _ => TraceTimeResult::Excluded(TemporalError::UnsupportedMovement),
    }
}

/// Location interval rule for schemas where open ends are retained and equal
/// endpoints are permitted by the already-reviewed source mapping. This does
/// not infer an endpoint or compare unrelated episode-end and boarding events.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntervalRule {
    OpenOrForward,
}

/// Reject reversed location intervals. `None` remains open/unknown, never zero.
pub fn validate_location_interval(
    start_ticks: u128,
    end_ticks: Option<u128>,
    _rule: IntervalRule,
) -> TraceTimeResult<()> {
    if end_ticks.is_some_and(|end| end < start_ticks) {
        TraceTimeResult::Excluded(TemporalError::ReversedInterval)
    } else {
        TraceTimeResult::Accepted(())
    }
}
