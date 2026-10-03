use kairo_ecs_arrow::trace_time::{
    normalize_occurrence, relative_ticks, validate_location_interval,
    validate_movement_claim, ClockRole, EventSemantic, IntervalRule, Lineage, MovementEvidence,
    SourcePrecision, TemporalError, TimestampInput, TimeValue, TraceTimeResult,
};

fn occurrence(utc_ns: i128, precision: SourcePrecision) -> TimestampInput {
    TimestampInput {
        role: ClockRole::Occurrence,
        lineage: Some(Lineage::Observed),
        precision,
        value: TimeValue::ResolvedUtc(utc_ns),
    }
}

#[test]
fn relative_ticks_are_exact_and_reject_pre_origin_or_overflow() {
    assert_eq!(relative_ticks(10, 10), Ok(0));
    assert_eq!(relative_ticks(10, 70_000_000_010), Ok(70_000_000_000));
    assert_eq!(relative_ticks(10, 9), Err(TemporalError::PreOrigin));
    assert_eq!(
        relative_ticks(i128::MIN, i128::MAX),
        Err(TemporalError::Overflow)
    );
}

#[test]
fn occurrence_requires_the_right_clock_role_and_present_lineage() {
    assert_eq!(
        normalize_occurrence(0, None),
        TraceTimeResult::Excluded(TemporalError::MissingOccurrence)
    );

    let mut wrong_role = occurrence(1, SourcePrecision::Minute);
    wrong_role.role = ClockRole::MessageCreated;
    assert_eq!(
        normalize_occurrence(0, Some(wrong_role)),
        TraceTimeResult::Excluded(TemporalError::ClockRoleMismatch)
    );

    let mut missing_lineage = occurrence(1, SourcePrecision::Minute);
    missing_lineage.lineage = None;
    assert_eq!(
        normalize_occurrence(0, Some(missing_lineage)),
        TraceTimeResult::Excluded(TemporalError::MissingLineage)
    );
}

#[test]
fn unresolved_or_coarse_timestamps_are_exclusions_not_fabricated_ticks() {
    for value in [TimeValue::UnresolvedLocal, TimeValue::AmbiguousLocal, TimeValue::NonexistentLocal] {
        let mut input = occurrence(0, SourcePrecision::Nanosecond);
        input.value = value;
        assert_eq!(
            normalize_occurrence(0, Some(input)),
            TraceTimeResult::Excluded(TemporalError::UnresolvedLocal)
        );
    }

    for (value, expected) in [
        (TimeValue::DateOnly, TemporalError::CoarsePrecision),
        (TimeValue::SubNanosecond, TemporalError::SubNanosecond),
    ] {
        let precision = if value == TimeValue::DateOnly {
            SourcePrecision::Coarse
        } else {
            SourcePrecision::Nanosecond
        };
        let mut input = occurrence(0, precision);
        input.value = value;
        assert_eq!(
            normalize_occurrence(0, Some(input)),
            TraceTimeResult::Excluded(expected)
        );
    }
}

#[test]
fn minute_precision_is_retained_and_episode_end_does_not_imply_departure() {
    let normalized = normalize_occurrence(
        0,
        Some(occurrence(120_000_000_000, SourcePrecision::Minute)),
    );
    let TraceTimeResult::Accepted(normalized) = normalized else {
        panic!("a resolved minute-precision event should be retained");
    };
    assert_eq!(normalized.relative_ticks, 120_000_000_000);
    assert_eq!(normalized.precision, SourcePrecision::Minute);

    assert_eq!(
        validate_movement_claim(EventSemantic::EpisodeEnd, MovementEvidence::Unverified),
        TraceTimeResult::Excluded(TemporalError::UnsupportedMovement)
    );
    assert_eq!(
        validate_movement_claim(
            EventSemantic::PhysicalDeparture,
            MovementEvidence::VerifiedPhysicalBoundary
        ),
        TraceTimeResult::Accepted(())
    );
}

#[test]
fn valid_boarding_after_episode_end_is_not_rejected_as_reversed_location_time() {
    assert_eq!(
        validate_location_interval(20, Some(25), IntervalRule::OpenOrForward),
        TraceTimeResult::Accepted(())
    );
    assert_eq!(
        validate_location_interval(25, Some(20), IntervalRule::OpenOrForward),
        TraceTimeResult::Excluded(TemporalError::ReversedInterval)
    );
    assert_eq!(
        validate_location_interval(25, None, IntervalRule::OpenOrForward),
        TraceTimeResult::Accepted(())
    );
}
