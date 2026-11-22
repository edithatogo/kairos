use super::*;
use kairo_ecs_arrow::arrow_c_data::{Array, ArrowDataType};
use kairo_ecs_arrow::cohort::{EXPOSURE_SCHEMA, TRANSITION_SCHEMA};
use kairo_ecs_rng::DeterministicStream;

#[test]
fn paired_arms_replay_and_reduce_fixture_delta_variance() -> Result<()> {
    let baseline = arm(SEED, BASELINE, ENTITIES)?;
    let paired = arm(SEED, INTERVENTION, ENTITIES)?;
    let independent = arm(SEED + 1, INTERVENTION, ENTITIES)?;
    let prefix = baseline.iter().filter(|row| row.event_at < ticks(START));
    assert!(prefix.clone().count() > 0);
    assert!(prefix.eq(paired.iter().filter(|row| row.event_at < ticks(START))));
    assert!(paired.len() > baseline.len());
    assert_eq!(paired, arm(SEED, INTERVENTION, ENTITIES)?);
    let variance = |other: &[Transition]| {
        let mut differences = vec![0.0_f64; ENTITIES as usize];
        for row in baseline.iter().filter(|row| row.to_state == 1) {
            differences[row.trial.entity.index as usize] -= 1.0;
        }
        for row in other.iter().filter(|row| row.to_state == 1) {
            differences[row.trial.entity.index as usize] += 1.0;
        }
        let mean = differences.iter().sum::<f64>() / ENTITIES as f64;
        differences.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / ENTITIES as f64
    };
    assert!(variance(&paired) < variance(&independent));

    let address = paired[0].trial;
    let expected = address.stream(SEED).next_u64();
    let mut unrelated = TrialAddress {
        slot: address.slot + 1,
        ..address
    }
    .stream(SEED);
    for _ in 0..37 {
        unrelated.next_u64();
    }
    assert_eq!(address.stream(SEED).next_u64(), expected);
    Ok(())
}

#[test]
fn extra_intermediate_visit_preserves_a_reconverged_trial() -> Result<()> {
    let model = hazards(BASELINE)?;
    let trace = |scheduled: &[(u32, u32, u128)]| -> Result<Vec<Transition>> {
        let entity = EntityId::new(5, 9);
        let mut entered_at = SimTime::ZERO;
        let mut rows = Vec::new();
        for &(from_state, to_state, exit) in scheduled {
            let trial = TrialAddress {
                entity,
                from_state,
                visit: 0,
                slot: 0,
            };
            let event_at = SimTime::from_ticks(exit);
            rows.push(Transition {
                trial,
                stratum_id: 0,
                to_state,
                entered_at,
                event_at,
                rng_start: trial.stream(SEED).checkpoint(),
            });
            entered_at = event_at;
        }
        let trial = TrialAddress {
            entity,
            from_state: 9,
            visit: 0,
            slot: 0,
        };
        let mut rng = trial.stream(SEED);
        let rng_start = rng.checkpoint();
        let outcome = model
            .sample_next(0.0, None, &mut rng)
            .ok_or("missing event")?;
        rows.push(Transition {
            trial,
            stratum_id: 0,
            to_state: outcome.winning_cause.0,
            entered_at,
            rng_start,
            event_at: SimTime::from_ticks(entered_at.ticks() + ticks(outcome.delta_time).ticks()),
        });
        transition_frame(&rows, TickUnit::Millisecond, SEED)?;
        Ok(rows)
    };
    let baseline = trace(&[(0, 4, 1), (4, 9, 4)])?;
    let policy = trace(&[(0, 4, 1), (4, 8, 2), (8, 9, 4)])?;
    assert_eq!(baseline.first(), policy.first());
    assert_eq!(baseline.len() + 1, policy.len());
    assert_eq!(baseline.last(), policy.last());
    Ok(())
}

#[test]
fn piecewise_competing_risks_match_independent_incidence_and_survival() -> Result<()> {
    const N: u64 = 100_000;
    let rows = arm(SEED, BASELINE, N)?;
    let (frame, schema) = transition_frame(&rows, TickUnit::Millisecond, SEED)?;
    let batch = domain::export_batch(&frame, &schema, Some(&schema))?;
    drop(frame);
    drop(rows);
    let (actual, data) = import_batch(batch)?;
    assert_eq!(actual, schema);
    let causes = data.values()[4]
        .as_any()
        .downcast_ref::<PrimitiveArray<u32>>()
        .unwrap();
    let times = data.values()[6]
        .as_any()
        .downcast_ref::<PrimitiveArray<u64>>()
        .unwrap();
    let mut counts = [[N, 0, 0]; 2];
    for row in 0..data.len() {
        let cause = causes.value(row) as usize;
        assert!(matches!(cause, 1 | 2));
        let at = u128::from(times.value(row));
        assert!(at <= ticks(END).ticks());
        counts[1][0] -= 1;
        counts[1][cause] += 1;
        if at < ticks(START).ticks() {
            counts[0][0] -= 1;
            counts[0][cause] += 1;
        }
    }
    let s2 = (-0.22_f64 * 2.0).exp();
    let conditional_s6 = (-0.30_f64 * 4.0).exp();
    let early = [s2, 0.14 / 0.22 * (1.0 - s2), 0.08 / 0.22 * (1.0 - s2)];
    let late = [
        s2 * conditional_s6,
        early[1] + s2 * 0.05 / 0.30 * (1.0 - conditional_s6),
        early[2] + s2 * 0.25 / 0.30 * (1.0 - conditional_s6),
    ];
    for (observed, expected) in counts.into_iter().zip([early, late]) {
        assert_eq!(observed.iter().sum::<u64>(), N);
        for (count, probability) in observed.into_iter().zip(expected) {
            let standard_error = (probability * (1.0 - probability) / N as f64).sqrt();
            assert!((count as f64 / N as f64 - probability).abs() <= 4.0 * standard_error);
        }
    }
    assert!(hazards([-0.1, 0.2]).is_err());
    assert!(hazards([f64::NAN, 0.2]).is_err());
    Ok(())
}

#[test]
fn trajectory_schema_preserves_limbs_and_rejects_invalid_rows() -> Result<()> {
    let trial = TrialAddress {
        entity: EntityId::new(u64::MAX, u32::MAX),
        from_state: 41,
        visit: 0,
        slot: 0,
    };
    let row = Transition {
        trial,
        stratum_id: 9,
        to_state: 63,
        entered_at: SimTime::from_ticks(2),
        event_at: SimTime::from_ticks(u128::MAX),
        rng_start: trial.stream(SEED).checkpoint(),
    };
    let encode = |rows: &[Transition]| transition_frame(rows, TickUnit::Second, SEED);
    let (frame, schema) = encode(std::slice::from_ref(&row))?;
    assert_eq!(schema.name.as_str(), TRANSITION_SCHEMA);
    assert_eq!(
        schema.metadata.as_ref().unwrap()["kairo_ecs.tick_unit"].as_str(),
        "second"
    );
    let (_, wrong_unit) = transition_frame(&[], TickUnit::Day, SEED)?;
    assert!(domain::export_batch(&frame, &schema, Some(&wrong_unit)).is_err());
    let (actual, array) = import_batch(domain::export_batch(&frame, &schema, Some(&schema))?)?;
    assert_eq!(actual, schema);
    let ArrowDataType::Struct(columns) = &actual.dtype else {
        panic!("expected a struct");
    };
    assert_eq!(
        columns
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>(),
        [
            "entity_id_index",
            "entity_id_gen",
            "stratum_id",
            "from_state",
            "to_state",
            "event_time_hi",
            "event_time_lo",
            "dwell_time_hi",
            "dwell_time_lo",
            "visit_index",
            "draw_slot",
            "rng_substream_id"
        ]
    );
    assert!(columns.iter().all(|item| !item.is_nullable));
    for (index, expected) in [
        (0, u64::MAX),
        (5, u64::MAX),
        (6, u64::MAX),
        (7, u64::MAX),
        (8, u64::MAX - 2),
    ] {
        let values = array.values()[index]
            .as_any()
            .downcast_ref::<PrimitiveArray<u64>>()
            .unwrap();
        assert_eq!(values.value(0), expected);
    }
    let (empty, empty_schema) = encode(&[])?;
    assert_eq!(empty_schema, schema);
    let (_, empty_array) = import_batch(domain::export_batch(&empty, &schema, None)?)?;
    assert_eq!(empty_array.len(), 0);

    let next_trial = TrialAddress {
        from_state: row.to_state,
        visit: 0,
        ..trial
    };
    let next = Transition {
        trial: next_trial,
        to_state: 8,
        entered_at: row.event_at,
        rng_start: next_trial.stream(SEED).checkpoint(),
        ..row.clone()
    };
    assert!(encode(&[row.clone(), next.clone()]).is_ok());
    assert!(encode(&[next.clone(), row.clone()]).is_err());
    assert!(encode(&[row.clone(), row.clone()]).is_err());
    let returning = Transition {
        to_state: trial.from_state,
        ..next
    };
    let reentered = Transition {
        entered_at: row.event_at,
        ..row.clone()
    };
    assert!(encode(&[row.clone(), returning.clone(), reentered.clone()]).is_err());
    let fresh_trial = TrialAddress { visit: 1, ..trial };
    let reentered = Transition {
        trial: fresh_trial,
        rng_start: fresh_trial.stream(SEED).checkpoint(),
        ..reentered
    };
    assert!(encode(&[row.clone(), returning, reentered]).is_ok());
    for invalid in [
        Transition {
            event_at: SimTime::ZERO,
            ..row.clone()
        },
        Transition {
            to_state: trial.from_state,
            ..row.clone()
        },
        Transition {
            rng_start: DeterministicStream::from_seed_u64(SEED, 0).checkpoint(),
            ..row.clone()
        },
        Transition {
            rng_start: trial.stream(SEED + 1).checkpoint(),
            ..row.clone()
        },
        Transition {
            rng_start: TrialAddress { slot: 1, ..trial }.stream(SEED).checkpoint(),
            ..row.clone()
        },
    ] {
        assert!(encode(&[invalid]).is_err());
    }
    let mut advanced = trial.stream(SEED);
    advanced.next_u64();
    assert!(encode(&[Transition {
        rng_start: advanced.checkpoint(),
        ..row
    }])
    .is_err());
    Ok(())
}

#[test]
fn proximity_queries_export_distance_and_no_neighbor_as_null() -> Result<()> {
    let rows = spatial_rows()?;
    let encode = |rows: &[SpatialExposure]| spatial_exposure_frame(rows, TickUnit::Millisecond);
    let (frame, schema) = encode(&rows)?;
    assert_eq!(schema.name.as_str(), EXPOSURE_SCHEMA);
    assert_eq!(
        schema.metadata.as_ref().unwrap()["kairo_ecs.length_unit"].as_str(),
        "metre"
    );
    let count = frame
        .column("features_within_radius")?
        .u32()?
        .cont_slice()?;
    assert_eq!(count, [1, 0]);
    let batch = domain::export_batch(&frame, &schema, None)?;
    drop(frame);
    let (actual, exposure) = import_batch(batch)?;
    assert_eq!(actual, schema);
    let distance = exposure.values()[8]
        .as_any()
        .downcast_ref::<PrimitiveArray<f64>>()
        .unwrap();
    assert_eq!(distance.value(0), 5.0);
    assert!(distance.is_null(1));
    assert!(encode(&[rows[1], rows[0]]).is_err());
    for invalid in [
        SpatialExposure {
            nearest_feature_metres: None,
            ..rows[0]
        },
        SpatialExposure {
            nearest_feature_metres: Some(f64::INFINITY),
            ..rows[0]
        },
        SpatialExposure {
            nearest_feature_metres: Some(10.0),
            ..rows[0]
        },
        SpatialExposure {
            radius_metres: -1.0,
            ..rows[0]
        },
    ] {
        assert!(encode(&[invalid]).is_err());
    }
    Ok(())
}
