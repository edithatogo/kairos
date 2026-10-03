use kairo_ecs_arrow::trace_time::relative_ticks;

#[test]
fn relative_ticks_accepts_full_ordered_signed_domain() {
    assert_eq!(relative_ticks(i128::MIN, i128::MAX), Ok(u128::MAX));
    assert_eq!(relative_ticks(i128::MIN, i128::MIN), Ok(0));
    assert_eq!(relative_ticks(i128::MAX, i128::MAX), Ok(0));
    assert_eq!(
        relative_ticks(i128::MAX, i128::MIN),
        Err(kairo_ecs_arrow::trace_time::TemporalError::PreOrigin)
    );
    assert_eq!(relative_ticks(-1, 0), Ok(1));
    assert_eq!(relative_ticks(i128::MIN, 0), Ok(1u128 << 127));
    assert_eq!(relative_ticks(0, i128::MAX), Ok(i128::MAX as u128));
}
