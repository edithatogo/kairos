use kairo_ecs_arrow::trace_time::{
    normalize_occurrence, normalize_timestamp, ClockRole, Lineage, NormalizedOccurrence,
    NormalizedTimestamp, SourcePrecision, TemporalError, TimeValue, TimestampInput,
    TraceTimeResult,
};

fn roles() -> [ClockRole; 4] {
    [
        ClockRole::Occurrence,
        ClockRole::SourceRecorded,
        ClockRole::MessageCreated,
        ClockRole::KnowledgeAvailable,
    ]
}
fn input(role: ClockRole) -> TimestampInput {
    TimestampInput {
        role,
        lineage: Some(Lineage::Observed),
        precision: SourcePrecision::Nanosecond,
        value: TimeValue::ResolvedUtc(11),
    }
}
#[test]
fn every_role_precision_and_lineage_is_preserved() {
    for role in roles() {
        for precision in [
            SourcePrecision::Minute,
            SourcePrecision::Second,
            SourcePrecision::Millisecond,
            SourcePrecision::Microsecond,
            SourcePrecision::Nanosecond,
        ] {
            for lineage in [
                Lineage::Observed,
                Lineage::Derived,
                Lineage::Defaulted,
                Lineage::Unknown,
            ] {
                let mut v = input(role);
                v.precision = precision;
                v.lineage = Some(lineage);
                assert_eq!(
                    normalize_timestamp(1, v),
                    TraceTimeResult::Accepted(NormalizedTimestamp {
                        role,
                        utc_nanoseconds: 11,
                        relative_ticks: 10,
                        precision,
                        lineage
                    })
                );
            }
        }
    }
}
#[test]
fn each_role_retains_existing_exclusion_classification() {
    for role in roles() {
        for (value, reason) in [
            (TimeValue::UnresolvedLocal, TemporalError::UnresolvedLocal),
            (TimeValue::AmbiguousLocal, TemporalError::AmbiguousLocal),
            (TimeValue::NonexistentLocal, TemporalError::NonexistentLocal),
            (TimeValue::DateOnly, TemporalError::CoarsePrecision),
            (TimeValue::SubNanosecond, TemporalError::SubNanosecond),
            (TimeValue::ResolvedUtc(-1), TemporalError::PreOrigin),
        ] {
            let mut v = input(role);
            v.value = value;
            assert_eq!(normalize_timestamp(0, v), TraceTimeResult::Excluded(reason));
        }
        for precision in [SourcePrecision::Coarse, SourcePrecision::Unknown] {
            let mut v = input(role);
            v.precision = precision;
            assert_eq!(
                normalize_timestamp(0, v),
                TraceTimeResult::Excluded(TemporalError::CoarsePrecision)
            );
        }
    }
}
#[test]
fn shared_error_precedence_is_not_reordered() {
    for role in roles() {
        let mut v = input(role);
        v.lineage = None;
        v.precision = SourcePrecision::Coarse;
        v.value = TimeValue::AmbiguousLocal;
        assert_eq!(
            normalize_timestamp(0, v),
            TraceTimeResult::Excluded(TemporalError::MissingLineage)
        );
        v.lineage = Some(Lineage::Unknown);
        assert_eq!(
            normalize_timestamp(0, v),
            TraceTimeResult::Excluded(TemporalError::CoarsePrecision)
        );
    }
}
#[test]
fn occurrence_wrapper_preserves_missing_and_role_precedence() {
    assert_eq!(
        normalize_occurrence(0, None),
        TraceTimeResult::Excluded(TemporalError::MissingOccurrence)
    );
    for role in roles().into_iter().filter(|r| *r != ClockRole::Occurrence) {
        let mut v = input(role);
        v.lineage = None;
        v.precision = SourcePrecision::Coarse;
        v.value = TimeValue::AmbiguousLocal;
        assert_eq!(
            normalize_occurrence(0, Some(v)),
            TraceTimeResult::Excluded(TemporalError::ClockRoleMismatch)
        );
    }
}
#[test]
fn accepted_occurrence_projects_to_unchanged_wrapper_fields() {
    let v = input(ClockRole::Occurrence);
    assert_eq!(
        normalize_occurrence(1, Some(v)),
        TraceTimeResult::Accepted(NormalizedOccurrence {
            utc_nanoseconds: 11,
            relative_ticks: 10,
            precision: SourcePrecision::Nanosecond,
            lineage: Lineage::Observed
        })
    );
    let TraceTimeResult::Accepted(shared) = normalize_timestamp(1, v) else {
        panic!("expected accepted supplied occurrence")
    };
    let TraceTimeResult::Accepted(legacy) = normalize_occurrence(1, Some(v)) else {
        panic!("expected accepted occurrence")
    };
    assert_eq!(
        (
            shared.utc_nanoseconds,
            shared.relative_ticks,
            shared.precision,
            shared.lineage
        ),
        (
            legacy.utc_nanoseconds,
            legacy.relative_ticks,
            legacy.precision,
            legacy.lineage
        )
    );
}
#[test]
fn every_role_has_exact_full_signed_span_and_origin() {
    for role in roles() {
        let mut v = input(role);
        v.value = TimeValue::ResolvedUtc(i128::MAX);
        assert_eq!(
            normalize_timestamp(i128::MIN, v),
            TraceTimeResult::Accepted(NormalizedTimestamp {
                role,
                utc_nanoseconds: i128::MAX,
                relative_ticks: u128::MAX,
                precision: SourcePrecision::Nanosecond,
                lineage: Lineage::Observed
            })
        );
        v.value = TimeValue::ResolvedUtc(i128::MIN);
        assert_eq!(
            normalize_timestamp(i128::MIN, v),
            TraceTimeResult::Accepted(NormalizedTimestamp {
                role,
                utc_nanoseconds: i128::MIN,
                relative_ticks: 0,
                precision: SourcePrecision::Nanosecond,
                lineage: Lineage::Observed
            })
        );
        assert_eq!(
            normalize_timestamp(i128::MAX, v),
            TraceTimeResult::Excluded(TemporalError::PreOrigin)
        );
    }
}
