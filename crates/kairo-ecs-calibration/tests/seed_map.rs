use kairo_ecs_calibration::seed_map::{CalibrationSeedError, CalibrationSeedMap, SeedPurpose};

fn map() -> CalibrationSeedMap {
    CalibrationSeedMap::new(1, "study-α", 1234).expect("valid map")
}

fn fixture_stream(
    seed_map: &mut CalibrationSeedMap,
    schedule: &str,
    purpose: SeedPurpose,
) -> kairo_ecs_calibration::seed_map::CalibrationStream {
    seed_map
        .stream_for(schedule, 7, "case-0001", "triage:1", purpose)
        .expect("valid seed identity")
}

#[test]
fn independent_python_golden_seed_and_draws_match() {
    let mut seed_map = map();
    let mut stream = fixture_stream(&mut seed_map, "crn-v1", SeedPurpose::Service);
    assert_eq!(stream.derived_seed(), 5_190_915_868_605_194_747);
    assert_eq!(stream.next_u64().unwrap(), 0xcabf_5867_c66c_f8ef);
    assert_eq!(stream.next_u64().unwrap(), 0xa20d_8e83_7c7b_ea4e);
    assert_eq!(stream.next_u64().unwrap(), 0xee85_ff56_f99b_d5b4);
    assert_eq!(stream.draw_position(), 3);
}

#[test]
fn reordered_registration_and_interleaved_draws_preserve_purpose_sequences() {
    let mut first_map = map();
    let mut service_a = fixture_stream(&mut first_map, "crn-v1", SeedPurpose::Service);
    let mut transit_a = fixture_stream(&mut first_map, "crn-v1", SeedPurpose::Transit);

    let mut second_map = map();
    let mut transit_b = fixture_stream(&mut second_map, "crn-v1", SeedPurpose::Transit);
    let mut service_b = fixture_stream(&mut second_map, "crn-v1", SeedPurpose::Service);

    let service_first_a = service_a.next_u64().unwrap();
    let transit_first_a = transit_a.next_u64().unwrap();
    let service_second_a = service_a.next_u64().unwrap();

    let transit_first_b = transit_b.next_u64().unwrap();
    let service_first_b = service_b.next_u64().unwrap();
    let service_second_b = service_b.next_u64().unwrap();
    assert_eq!(service_first_a, service_first_b);
    assert_eq!(service_second_a, service_second_b);
    assert_eq!(transit_first_a, transit_first_b);
    assert_eq!(service_a.draw_position(), 2);
    assert_eq!(transit_a.draw_position(), 1);
}

#[test]
fn paired_candidates_share_schedule_streams_and_independent_schedules_diverge() {
    let mut candidates = map();
    let mut a = fixture_stream(&mut candidates, "paired-schedule-v1", SeedPurpose::Service);
    let mut b = fixture_stream(&mut candidates, "paired-schedule-v1", SeedPurpose::Service);
    assert_eq!(a.derived_seed(), b.derived_seed());
    assert_eq!(a.next_u64().unwrap(), b.next_u64().unwrap());

    let c = fixture_stream(
        &mut candidates,
        "candidate-b-independent-v1",
        SeedPurpose::Service,
    );
    assert_ne!(a.derived_seed(), c.derived_seed());
}

#[test]
fn owned_snapshots_resume_at_positions_zero_and_one() {
    for position in [0, 1] {
        let mut seed_map = map();
        let mut stream = fixture_stream(&mut seed_map, "crn-v1", SeedPurpose::Behavior);
        if position == 1 {
            stream.next_u64().unwrap();
        }
        assert_eq!(stream.draw_position(), position);
        let snapshot = stream.snapshot();
        let expected = [stream.next_u64().unwrap(), stream.next_u64().unwrap()];

        let mut restored = snapshot.restore().expect("valid owned snapshot");
        assert_eq!(restored.draw_position(), position);
        assert_eq!(
            [restored.next_u64().unwrap(), restored.next_u64().unwrap()],
            expected
        );
    }
}

#[test]
fn unknown_seed_map_version_fails_explicitly() {
    assert!(matches!(
        CalibrationSeedMap::new(2, "study", 0),
        Err(CalibrationSeedError::UnsupportedSeedMapVersion(2))
    ));
}
