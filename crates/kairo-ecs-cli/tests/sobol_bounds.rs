use kairo_ecs_cli::ensemble::{SobolError, SobolSequence};

#[test]
fn dimensions_and_indices_are_checked_before_arithmetic() {
    for dimension in [0, 17, usize::MAX] {
        assert_eq!(
            SobolSequence::new(dimension).unwrap_err(),
            SobolError::Dimension(dimension)
        );
    }

    let mut sequence = SobolSequence::new(1).unwrap();
    let end = 1_u64 << 32;
    sequence.seek(end - 1).unwrap();
    for index in [end, u64::MAX] {
        assert_eq!(sequence.seek(index), Err(SobolError::Index(index)));
        assert_eq!(sequence.point_at(index), Err(SobolError::Index(index)));
    }
    assert_eq!(sequence.next_point().unwrap(), [1.0 / end as f64]);
    assert_eq!(sequence.next_point(), Err(SobolError::Index(end)));
    assert_eq!(sequence.next_point(), Err(SobolError::Index(end)));
    sequence.seek(0).unwrap();
    assert_eq!(sequence.next_point().unwrap(), [0.0]);
}
