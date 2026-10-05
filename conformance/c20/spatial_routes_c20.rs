use kairo_ecs_abm::spatial::{
    EdgeId, MovementModeId, MovementProfile, NodeId, TransitEdge, TransitError, TransitGraphV1,
};

fn n(value: u64) -> NodeId {
    NodeId::new(value)
}

fn e(value: u64, from: u64, to: u64, length_mm: u64, modes: &[&str]) -> TransitEdge {
    TransitEdge {
        id: EdgeId::new(value),
        from: n(from),
        to: n(to),
        length_mm,
        allowed_modes: modes
            .iter()
            .map(|mode| MovementModeId::new(mode).unwrap())
            .collect(),
    }
}

fn profile(mode: &str, speed: u64) -> MovementProfile {
    MovementProfile::new(mode, speed).unwrap()
}

#[test]
fn rejects_invalid_version_graph_mode_speed_and_tick_rate() {
    assert!(matches!(
        TransitGraphV1::new(0, vec![n(1)], vec![]),
        Err(TransitError::UnsupportedVersion)
    ));
    assert!(matches!(
        TransitGraphV1::new(1, vec![n(1), n(1)], vec![]),
        Err(TransitError::InvalidGraph)
    ));
    assert!(matches!(
        TransitGraphV1::new(1, vec![n(1), n(2)], vec![e(1, 1, 3, 1, &["walk"])]),
        Err(TransitError::InvalidGraph)
    ));
    assert!(matches!(
        TransitGraphV1::new(1, vec![n(1), n(2)], vec![e(1, 1, 2, 1, &[])]),
        Err(TransitError::InvalidGraph)
    ));
    assert!(matches!(
        MovementModeId::new(""),
        Err(TransitError::InvalidMovementMode)
    ));
    assert!(matches!(
        MovementProfile::new("walk", 0),
        Err(TransitError::InvalidSpeed)
    ));

    let graph = TransitGraphV1::new(1, vec![n(1), n(2)], vec![e(1, 1, 2, 1, &["walk"])]).unwrap();
    assert!(matches!(
        graph.route(n(1), n(2), &profile("bike", 1), 1),
        Err(TransitError::Unreachable)
    ));
    assert!(matches!(
        graph.route(n(0), n(2), &profile("walk", 1), 1),
        Err(TransitError::UnknownNode)
    ));
    assert!(matches!(
        graph.route(n(1), n(2), &profile("walk", 1), 0),
        Err(TransitError::InvalidTickRate)
    ));
}

#[test]
fn rejects_duplicate_edges_and_duplicate_allowed_modes() {
    assert!(matches!(
        TransitGraphV1::new(
            1,
            vec![n(1), n(2)],
            vec![e(7, 1, 2, 1, &["walk"]), e(7, 1, 2, 2, &["walk"])]
        ),
        Err(TransitError::InvalidGraph)
    ));
    assert!(matches!(
        TransitGraphV1::new(1, vec![n(1), n(2)], vec![e(7, 1, 2, 1, &["walk", "walk"])]),
        Err(TransitError::InvalidGraph)
    ));
}

#[test]
fn chooses_distance_then_hops_then_full_edge_id_sequence_and_ignores_zero_cycles() {
    let graph = TransitGraphV1::new(
        1,
        vec![n(1), n(2), n(3), n(4)],
        vec![
            e(9, 1, 4, 5, &["walk"]),
            e(4, 1, 2, 2, &["walk"]),
            e(5, 2, 4, 3, &["walk"]),
            e(1, 1, 3, 2, &["walk"]),
            e(8, 3, 4, 3, &["walk"]),
            e(2, 2, 3, 0, &["walk"]),
            e(3, 3, 2, 0, &["walk"]),
        ],
    )
    .unwrap();
    let route = graph.route(n(1), n(4), &profile("walk", 1), 1).unwrap();
    assert_eq!(route.distance_mm(), 5);
    assert_eq!(
        route
            .segments()
            .iter()
            .map(|segment| segment.edge_id().value())
            .collect::<Vec<_>>(),
        vec![9]
    );
}

#[test]
fn equal_distance_and_hop_count_use_lexicographic_full_edge_id_sequence() {
    let graph = TransitGraphV1::new(
        1,
        vec![n(1), n(2), n(3), n(4)],
        vec![
            e(90, 1, 4, 7, &["walk"]),
            e(8, 1, 2, 3, &["walk"]),
            e(30, 2, 4, 4, &["walk"]),
            e(3, 1, 3, 3, &["walk"]),
            e(99, 3, 4, 4, &["walk"]),
        ],
    )
    .unwrap();
    let route = graph.route(n(1), n(4), &profile("walk", 1), 1).unwrap();
    assert_eq!(
        route
            .segments()
            .iter()
            .map(|segment| segment.edge_id().value())
            .collect::<Vec<_>>(),
        vec![3, 99]
    );
}

#[test]
fn origin_equals_destination_is_empty_and_distinct_zero_route_is_valid() {
    let graph = TransitGraphV1::new(
        1,
        vec![n(1), n(2), n(3)],
        vec![e(1, 1, 2, 0, &["walk"]), e(2, 2, 3, 0, &["walk"])],
    )
    .unwrap();
    let same = graph.route(n(1), n(1), &profile("walk", 1), 7).unwrap();
    assert!(same.segments().is_empty());
    assert_eq!(same.distance_mm(), 0);
    assert_eq!(same.duration().ticks(), 0);

    let distinct = graph.route(n(1), n(3), &profile("walk", 1), 7).unwrap();
    assert_eq!(distinct.distance_mm(), 0);
    assert_eq!(distinct.duration().ticks(), 0);
    assert_eq!(distinct.segments().len(), 2);
}

#[test]
fn canonical_bytes_ignore_permutation_and_bind_sorted_modes() {
    let a = TransitGraphV1::new(
        1,
        vec![n(1), n(2), n(3)],
        vec![e(2, 2, 3, 4, &["walk", "wheel"]), e(1, 1, 2, 3, &["walk"])],
    )
    .unwrap();
    let b = TransitGraphV1::new(
        1,
        vec![n(3), n(1), n(2)],
        vec![e(1, 1, 2, 3, &["walk"]), e(2, 2, 3, 4, &["wheel", "walk"])],
    )
    .unwrap();
    assert_eq!(a.canonical_bytes(), b.canonical_bytes());
    let route_a = a.route(n(1), n(3), &profile("walk", 3), 5).unwrap();
    let route_b = b.route(n(1), n(3), &profile("walk", 3), 5).unwrap();
    assert_eq!(route_a.duration(), route_b.duration());
    assert_eq!(route_a.distance_mm(), route_b.distance_mm());
    assert_eq!(
        route_a
            .segments()
            .iter()
            .map(|segment| segment.edge_id().value())
            .collect::<Vec<_>>(),
        route_b
            .segments()
            .iter()
            .map(|segment| segment.edge_id().value())
            .collect::<Vec<_>>()
    );
}

#[test]
fn cumulative_ceiling_segment_increments_sum_to_total_duration() {
    let graph = TransitGraphV1::new(
        1,
        vec![n(1), n(2), n(3)],
        vec![e(1, 1, 2, 1, &["walk"]), e(2, 2, 3, 1, &["walk"])],
    )
    .unwrap();
    let route = graph.route(n(1), n(3), &profile("walk", 3), 2).unwrap();
    let increments = route
        .segments()
        .iter()
        .map(|segment| segment.end_offset().ticks() - segment.start_offset().ticks())
        .collect::<Vec<_>>();
    assert_eq!(increments, vec![1, 1]);
    assert_eq!(increments.iter().sum::<u128>(), route.duration().ticks());
    assert_eq!(route.duration().ticks(), 2);
}

#[test]
fn accumulates_distance_in_u128_beyond_u64() {
    let graph = TransitGraphV1::new(
        1,
        vec![n(1), n(2), n(3)],
        vec![
            e(1, 1, 2, u64::MAX, &["walk"]),
            e(2, 2, 3, u64::MAX, &["walk"]),
        ],
    )
    .unwrap();
    let route = graph
        .route(n(1), n(3), &profile("walk", u64::MAX), 1)
        .unwrap();
    assert_eq!(route.distance_mm(), u128::from(u64::MAX) * 2);
    assert_eq!(route.duration().ticks(), 2);
}

#[test]
fn checked_tick_multiplication_overflow_is_reported() {
    let graph =
        TransitGraphV1::new(1, vec![n(1), n(2)], vec![e(1, 1, 2, u64::MAX, &["walk"])]).unwrap();
    assert!(matches!(
        graph.route(n(1), n(2), &profile("walk", 1), u64::MAX),
        Err(TransitError::Overflow)
    ));
}
