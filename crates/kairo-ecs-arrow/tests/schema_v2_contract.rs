use kairo_ecs_arrow::*;

#[test]
fn schema_v2_field_contract_and_fingerprint() {
    let fp = event_log_v2_schema_fingerprint();
    assert!(fp.starts_with("kairo_ecs.event_log.v2;major=2;fields="));

    let fields = EVENT_LOG_FIELDS_V2;
    assert_eq!(fields[0].name, "schema_version");
    assert_eq!(fields[1].name, "run_id");
    assert_eq!(fields[2].name, "event_id_index");
    assert_eq!(fields[3].name, "event_id_gen");
    assert_eq!(fields[4].name, "entity_id_index");
    assert_eq!(fields[5].name, "entity_id_gen");
    assert_eq!(fields[6].name, "time_ticks_hi");
    assert_eq!(fields[7].name, "time_ticks_lo");

    assert!(fields[4].nullable);
    assert!(fields[5].nullable);
    assert!(!fields[6].nullable);
    assert!(!fields[7].nullable);
}

#[test]
fn schema_v2_limb_subtraction_preserves_numerical_ordering() {
    let tick1_hi: u64 = 1;
    let tick1_lo: u64 = 50; // (1 << 64) + 50
    let tick2_hi: u64 = 0;
    let tick2_lo: u64 = 100; // 100

    // Borrow-preserving limb subtraction:
    let (diff_lo, borrow) = tick1_lo.overflowing_sub(tick2_lo);
    let diff_hi = tick1_hi - tick2_hi - (borrow as u64);

    let actual_diff = ((tick1_hi as u128) << 64 | tick1_lo as u128)
        - ((tick2_hi as u128) << 64 | tick2_lo as u128);

    assert_eq!(((diff_hi as u128) << 64) | (diff_lo as u128), actual_diff);
}
