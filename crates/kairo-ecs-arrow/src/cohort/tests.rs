use super::*;

const SEED: u64 = 81;

fn transition(entity: EntityId, states: (u32, u32), visit: u64, ticks: (u128, u128)) -> Transition {
    let trial = TrialAddress {
        entity,
        from_state: states.0,
        visit,
        slot: 0,
    };
    Transition {
        trial,
        stratum_id: 10,
        to_state: states.1,
        entered_at: SimTime::from_ticks(ticks.0),
        event_at: SimTime::from_ticks(ticks.1),
        rng_start: trial.stream(SEED).checkpoint(),
    }
}

#[test]
fn interleaved_generations_and_same_tick_reentry_preserve_state_visits() {
    let generation_1 = EntityId::new(7, 1);
    let generation_2 = EntityId::new(7, 2);
    let wide = u128::from(u64::MAX) + 4;
    let rows = [
        transition(generation_1, (4, 9), 0, (0, 2)),
        transition(generation_2, (4, 7), 0, (0, wide)),
        transition(generation_1, (9, 4), 0, (2, 2)),
        transition(generation_1, (4, 8), 1, (2, 2)),
    ];
    for (unit, expected) in [
        (TickUnit::Millisecond, "millisecond"),
        (TickUnit::Second, "second"),
        (TickUnit::Minute, "minute"),
        (TickUnit::Hour, "hour"),
        (TickUnit::Day, "day"),
    ] {
        let (frame, schema) = transition_frame(&rows, unit, SEED).unwrap();
        assert_eq!(schema.metadata.unwrap()["kairo_ecs.tick_unit"], expected);
        assert_eq!(
            frame
                .column("entity_id_gen")
                .unwrap()
                .u32()
                .unwrap()
                .cont_slice()
                .unwrap(),
            [1, 2, 1, 1]
        );
        let time_hi = frame.column("event_time_hi").unwrap().u64().unwrap();
        let dwell_lo = frame.column("dwell_time_lo").unwrap().u64().unwrap();
        let substreams = frame.column("rng_substream_id").unwrap().u64().unwrap();
        assert_eq!(time_hi.cont_slice().unwrap(), [0, 1, 0, 0]);
        assert_eq!(dwell_lo.cont_slice().unwrap(), [2, 3, 0, 0]);
        assert_ne!(substreams.get(0), substreams.get(1));
        assert_ne!(substreams.get(0), substreams.get(3));
    }

    let mut skipped = rows.clone();
    skipped[3] = transition(generation_1, (4, 8), 2, (2, 2));
    assert_eq!(
        transition_frame(&skipped, TickUnit::Second, SEED).unwrap_err(),
        ArrowError::InvalidArgument("cohort visits are discontinuous or out of order")
    );
    let mut mismatched_seed = rows.clone();
    mismatched_seed[1].rng_start = mismatched_seed[1].trial.stream(SEED + 1).checkpoint();
    assert_eq!(
        transition_frame(&mismatched_seed, TickUnit::Second, SEED).unwrap_err(),
        ArrowError::InvalidArgument("cohort RNG checkpoint does not match the trial address")
    );
    for (row, reason) in [
        (
            transition(generation_1, (4, 8), 0, (3, 2)),
            "cohort dwell time is negative",
        ),
        (
            transition(generation_1, (4, 4), 0, (2, 3)),
            "cohort state did not change",
        ),
    ] {
        assert_eq!(
            transition_frame(&[row], TickUnit::Second, SEED).unwrap_err(),
            ArrowError::InvalidArgument(reason)
        );
    }
}

#[test]
fn zero_radius_keeps_exact_matches_and_distinguishes_missing_features() {
    let at_origin = SpatialExposure {
        entity: EntityId::new(3, 8),
        stratum_id: 9,
        observed_at: SimTime::from_ticks(10),
        spatial_cell_id: 4,
        radius_metres: 0.0,
        features_within_radius: 1,
        nearest_feature_metres: Some(0.0),
    };
    let absent = SpatialExposure {
        observed_at: SimTime::from_ticks(11),
        spatial_cell_id: 5,
        features_within_radius: 0,
        nearest_feature_metres: None,
        ..at_origin
    };
    let encode = |rows: &[SpatialExposure]| spatial_exposure_frame(rows, TickUnit::Hour);
    let (frame, schema) = encode(&[at_origin, absent]).unwrap();
    let nearest = frame
        .column("nearest_feature_metres")
        .unwrap()
        .f64()
        .unwrap();
    assert_eq!(nearest.get(0), Some(0.0));
    assert_eq!(nearest.get(1), None);
    assert_eq!(encode(&[]).unwrap().1, schema);
    assert_eq!(
        encode(&[absent, at_origin]).unwrap_err(),
        ArrowError::InvalidArgument("spatial observations are out of order")
    );
    for invalid in [
        SpatialExposure {
            nearest_feature_metres: Some(f64::NAN),
            ..at_origin
        },
        SpatialExposure {
            nearest_feature_metres: Some(-1.0),
            ..at_origin
        },
        SpatialExposure {
            nearest_feature_metres: Some(1.0),
            ..at_origin
        },
        SpatialExposure {
            nearest_feature_metres: Some(0.0),
            ..absent
        },
        SpatialExposure {
            radius_metres: f64::INFINITY,
            ..at_origin
        },
    ] {
        assert_eq!(
            encode(&[invalid]).unwrap_err(),
            ArrowError::InvalidArgument("invalid spatial proximity")
        );
    }
}
