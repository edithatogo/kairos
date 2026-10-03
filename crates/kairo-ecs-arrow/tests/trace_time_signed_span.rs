use kairo_ecs_arrow::trace_time::relative_ticks;

#[test]
fn relative_ticks_accepts_full_ordered_signed_domain() {
    assert_eq!(relative_ticks(i128::MIN, i128::MAX), Ok(u128::MAX));
}
