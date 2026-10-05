use kairo_ecs_abm::spatial::{
    EdgeId, MovementModeId, MovementProfile, NodeId, TransitEdge, TransitGraphV1,
};

fn graph() -> TransitGraphV1 {
    TransitGraphV1::new(
        1,
        vec![NodeId::new(2), NodeId::new(1)],
        vec![TransitEdge {
            id: EdgeId::new(7),
            from: NodeId::new(1),
            to: NodeId::new(2),
            length_mm: 5,
            allowed_modes: vec![MovementModeId::new("walk").unwrap()],
        }],
    )
    .unwrap()
}

#[test]
fn exact_v1_topology_byte_golden() {
    let hex = "4b4149524f532d5452414e5349542d475241504800010000006d6d00020000000000000001000000000000000200000000000000010000000000000007000000000000000100000000000000020000000000000005000000000000000100000000000000040000000000000077616c6b";
    let expected: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    assert_eq!(graph().canonical_bytes(), expected);
}

#[test]
fn actual_public_getters_bind_geometry_profile_and_time() {
    let graph = graph();
    let profile = MovementProfile::new("walk", 2).unwrap();
    let route = graph
        .route(NodeId::new(1), NodeId::new(2), &profile, 3)
        .unwrap();
    assert_eq!(route.origin().value(), 1);
    assert_eq!(route.destination().value(), 2);
    assert_eq!(route.graph_version(), 1);
    assert_eq!(route.graph_canonical_bytes(), graph.canonical_bytes());
    assert_eq!(route.profile().mode().as_str(), "walk");
    assert_eq!(route.profile().speed_mm_per_second().get(), 2);
    assert_eq!(route.ticks_per_second(), 3);
    assert_eq!(route.distance_mm(), 5);
    assert_eq!(route.duration().ticks(), 8);
    let segment = &route.segments()[0];
    assert_eq!(segment.edge_id().value(), 7);
    assert_eq!(segment.from().value(), 1);
    assert_eq!(segment.to().value(), 2);
    assert_eq!(segment.length_mm(), 5);
    assert_eq!(segment.start_offset().ticks(), 0);
    assert_eq!(segment.end_offset().ticks(), 8);
}

#[test]
fn exact_mode_identity_boundaries_and_unicode_are_checked() {
    for invalid in [
        "",
        " walk",
        "walk ",
        "\u{2003}walk",
        "walk\u{2003}",
        "wa\0lk",
        "wa\nlk",
        "wa\tlk",
    ] {
        assert!(MovementModeId::new(invalid).is_err(), "{invalid:?}");
    }
    assert!(MovementModeId::new(&"x".repeat(1024)).is_ok());
    assert!(MovementModeId::new(&"x".repeat(1025)).is_err());
    assert!(MovementModeId::new(&"é".repeat(512)).is_ok());
    assert!(MovementModeId::new(&"é".repeat(513)).is_err());
    for valid in ["walk", "Walk", "patient walk", "é", "e\u{301}"] {
        assert_eq!(MovementModeId::new(valid).unwrap().as_str(), valid);
    }
}
