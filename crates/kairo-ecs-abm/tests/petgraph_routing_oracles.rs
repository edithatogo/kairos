#![forbid(unsafe_code)]

use kairo_ecs_abm::routing::*;
use kairo_ecs_abm::{ABMContext, GraphSpace};
use kairo_ecs_rng::DeterministicStream;
use kairo_ecs_types::{EntityId, SimTime};

#[test]
fn test_small_graph_optimality() {
    let mut topo = SpatialTopology::new(CostUnit::Ticks);
    let n1 = EntityId::new(1, 0);
    let n2 = EntityId::new(2, 0);
    let n3 = EntityId::new(3, 0);
    let n4 = EntityId::new(4, 0);

    topo.add_node(n1, 0, SpatialNodeKind::Room).unwrap();
    topo.add_node(n2, 0, SpatialNodeKind::Corridor).unwrap();
    topo.add_node(n3, 0, SpatialNodeKind::Corridor).unwrap();
    topo.add_node(n4, 0, SpatialNodeKind::Room).unwrap();

    // Two routes:
    // Route 1: n1 -> n2 -> n4 (costs: 10 + 20 = 30)
    // Route 2: n1 -> n3 -> n4 (costs: 5 + 10 = 15)
    topo.add_edge(
        1,
        n1,
        n2,
        10,
        SpatialEdgeKind::Walkway,
        EdgeAccessRule::default(),
    )
    .unwrap();
    topo.add_edge(
        2,
        n2,
        n4,
        20,
        SpatialEdgeKind::Walkway,
        EdgeAccessRule::default(),
    )
    .unwrap();
    topo.add_edge(
        3,
        n1,
        n3,
        5,
        SpatialEdgeKind::Walkway,
        EdgeAccessRule::default(),
    )
    .unwrap();
    topo.add_edge(
        4,
        n3,
        n4,
        10,
        SpatialEdgeKind::Walkway,
        EdgeAccessRule::default(),
    )
    .unwrap();

    let route = topo.find_route(n1, n4, &AccessProfile::standard()).unwrap();
    assert_eq!(route.total_cost, 15);
    assert_eq!(route.hop_count, 2);
    assert_eq!(route.path, vec![n1, n3, n4]);
    assert_eq!(route.segments.len(), 2);
    assert_eq!(route.segments[0].edge_id, 3);
    assert_eq!(route.segments[1].edge_id, 4);
}

#[test]
fn test_cost_hops_edge_id_tie_breaking() {
    let mut topo = SpatialTopology::new(CostUnit::Ticks);
    let start = EntityId::new(1, 0);
    let target = EntityId::new(99, 0);

    topo.add_node(start, 0, SpatialNodeKind::Room).unwrap();
    topo.add_node(target, 0, SpatialNodeKind::Room).unwrap();

    // Intermediate nodes
    let a1 = EntityId::new(10, 0);
    let b1 = EntityId::new(20, 0);
    let b2 = EntityId::new(21, 0);
    let c1 = EntityId::new(30, 0);

    topo.add_node(a1, 0, SpatialNodeKind::Corridor).unwrap();
    topo.add_node(b1, 0, SpatialNodeKind::Corridor).unwrap();
    topo.add_node(b2, 0, SpatialNodeKind::Corridor).unwrap();
    topo.add_node(c1, 0, SpatialNodeKind::Corridor).unwrap();

    // Candidate Path A: start -> a1 -> target (cost 10 + 10 = 20, hops = 2, edge_ids = [100, 101])
    topo.add_edge(
        100,
        start,
        a1,
        10,
        SpatialEdgeKind::Walkway,
        EdgeAccessRule::default(),
    )
    .unwrap();
    topo.add_edge(
        101,
        a1,
        target,
        10,
        SpatialEdgeKind::Walkway,
        EdgeAccessRule::default(),
    )
    .unwrap();

    // Candidate Path B: start -> b1 -> b2 -> target (cost 5 + 5 + 10 = 20, hops = 3, edge_ids = [1, 2, 3])
    // Even though edge IDs [1, 2, 3] are smaller numbers, Path B has 3 hops, so Path A (2 hops) beats it on hop count!
    topo.add_edge(
        1,
        start,
        b1,
        5,
        SpatialEdgeKind::Walkway,
        EdgeAccessRule::default(),
    )
    .unwrap();
    topo.add_edge(
        2,
        b1,
        b2,
        5,
        SpatialEdgeKind::Walkway,
        EdgeAccessRule::default(),
    )
    .unwrap();
    topo.add_edge(
        3,
        b2,
        target,
        10,
        SpatialEdgeKind::Walkway,
        EdgeAccessRule::default(),
    )
    .unwrap();

    // Candidate Path C: start -> c1 -> target (cost 10 + 10 = 20, hops = 2, edge_ids = [200, 201])
    // Has same cost 20 and same hops 2 as Path A, but edge_ids [200, 201] > [100, 101].
    topo.add_edge(
        200,
        start,
        c1,
        10,
        SpatialEdgeKind::Walkway,
        EdgeAccessRule::default(),
    )
    .unwrap();
    topo.add_edge(
        201,
        c1,
        target,
        10,
        SpatialEdgeKind::Walkway,
        EdgeAccessRule::default(),
    )
    .unwrap();

    let route = topo
        .find_route(start, target, &AccessProfile::standard())
        .unwrap();

    // Verification: Path A must be chosen (cost=20, hops=2, edge_id sequence [100, 101])
    assert_eq!(route.total_cost, 20);
    assert_eq!(route.hop_count, 2);
    assert_eq!(route.path, vec![start, a1, target]);
    assert_eq!(route.segments[0].edge_id, 100);
    assert_eq!(route.segments[1].edge_id, 101);
}

#[test]
fn test_shuffled_insertion_order_invariance() {
    let mut rng = DeterministicStream::new(12345);

    let nodes = vec![
        EntityId::new(1, 0),
        EntityId::new(2, 0),
        EntityId::new(3, 0),
        EntityId::new(4, 0),
        EntityId::new(5, 0),
    ];

    let edges = vec![
        (10, nodes[0], nodes[1], 5),
        (11, nodes[1], nodes[4], 15),
        (20, nodes[0], nodes[2], 8),
        (21, nodes[2], nodes[3], 4),
        (22, nodes[3], nodes[4], 8),
        (30, nodes[0], nodes[3], 12),
    ];

    let mut baseline_route: Option<SpatialRoute> = None;

    // Test 10 different random insertion permutations
    for _ in 0..10 {
        let mut shuffled_nodes = nodes.clone();
        for i in (1..shuffled_nodes.len()).rev() {
            let j = (rng.next_u64() as usize) % (i + 1);
            shuffled_nodes.swap(i, j);
        }

        let mut shuffled_edges = edges.clone();
        for i in (1..shuffled_edges.len()).rev() {
            let j = (rng.next_u64() as usize) % (i + 1);
            shuffled_edges.swap(i, j);
        }

        let mut topo = SpatialTopology::new(CostUnit::Ticks);
        for &node in &shuffled_nodes {
            topo.add_node(node, 0, SpatialNodeKind::Corridor).unwrap();
        }
        for &(eid, src, dst, cost) in &shuffled_edges {
            topo.add_edge(
                eid,
                src,
                dst,
                cost,
                SpatialEdgeKind::Walkway,
                EdgeAccessRule::default(),
            )
            .unwrap();
        }

        let route = topo
            .find_route(nodes[0], nodes[4], &AccessProfile::standard())
            .unwrap();

        if let Some(ref base) = baseline_route {
            assert_eq!(
                route.path, base.path,
                "Path must be bit-identical across insertion permutations"
            );
            assert_eq!(route.total_cost, base.total_cost);
            assert_eq!(route.hop_count, base.hop_count);
            assert_eq!(route.segments, base.segments);
        } else {
            baseline_route = Some(route);
        }
    }
}

#[test]
fn test_zero_cost_cycle_resilience() {
    let mut topo = SpatialTopology::new(CostUnit::Ticks);
    let s = EntityId::new(1, 0);
    let a = EntityId::new(2, 0);
    let b = EntityId::new(3, 0);
    let t = EntityId::new(4, 0);

    topo.add_node(s, 0, SpatialNodeKind::Room).unwrap();
    topo.add_node(a, 0, SpatialNodeKind::Corridor).unwrap();
    topo.add_node(b, 0, SpatialNodeKind::Corridor).unwrap();
    topo.add_node(t, 0, SpatialNodeKind::Room).unwrap();

    // Zero-cost cycle between a and b: a -> b (cost 0), b -> a (cost 0)
    topo.add_edge(
        1,
        s,
        a,
        10,
        SpatialEdgeKind::Walkway,
        EdgeAccessRule::default(),
    )
    .unwrap();
    topo.add_edge(
        2,
        a,
        b,
        0,
        SpatialEdgeKind::Walkway,
        EdgeAccessRule::default(),
    )
    .unwrap();
    topo.add_edge(
        3,
        b,
        a,
        0,
        SpatialEdgeKind::Walkway,
        EdgeAccessRule::default(),
    )
    .unwrap();
    topo.add_edge(
        4,
        a,
        t,
        10,
        SpatialEdgeKind::Walkway,
        EdgeAccessRule::default(),
    )
    .unwrap();

    // Query must resolve cleanly without infinite loop or incorporating the cycle
    let route = topo.find_route(s, t, &AccessProfile::standard()).unwrap();
    assert_eq!(route.total_cost, 20);
    assert_eq!(route.hop_count, 2);
    assert_eq!(route.path, vec![s, a, t]);
}

#[test]
fn test_directionality_and_disconnected() {
    let mut topo = SpatialTopology::new(CostUnit::Ticks);
    let a = EntityId::new(1, 0);
    let b = EntityId::new(2, 0);
    let c = EntityId::new(3, 0);

    topo.add_node(a, 0, SpatialNodeKind::Room).unwrap();
    topo.add_node(b, 0, SpatialNodeKind::Room).unwrap();
    topo.add_node(c, 0, SpatialNodeKind::Room).unwrap();

    // Directed edge a -> b
    topo.add_edge(
        1,
        a,
        b,
        10,
        SpatialEdgeKind::Walkway,
        EdgeAccessRule::default(),
    )
    .unwrap();

    // Forward path exists
    assert!(topo.find_route(a, b, &AccessProfile::standard()).is_ok());

    // Reverse path must fail with NoRouteFound
    assert_eq!(
        topo.find_route(b, a, &AccessProfile::standard())
            .unwrap_err(),
        RoutingError::NoRouteFound { from: b, to: a }
    );

    // Disconnected node c
    assert_eq!(
        topo.find_route(a, c, &AccessProfile::standard())
            .unwrap_err(),
        RoutingError::NoRouteFound { from: a, to: c }
    );
}

#[test]
fn test_cost_overflow_and_exact_u64_max_handling() {
    let mut topo = SpatialTopology::new(CostUnit::Ticks);
    let a = EntityId::new(1, 0);
    let b = EntityId::new(2, 0);
    let c = EntityId::new(3, 0);
    let d = EntityId::new(4, 0);

    topo.add_node(a, 0, SpatialNodeKind::Room).unwrap();
    topo.add_node(b, 0, SpatialNodeKind::Room).unwrap();
    topo.add_node(c, 0, SpatialNodeKind::Room).unwrap();
    topo.add_node(d, 0, SpatialNodeKind::Room).unwrap();

    // Exactly u64::MAX cost path: a -> d (cost u64::MAX)
    topo.add_edge(
        10,
        a,
        d,
        u64::MAX,
        SpatialEdgeKind::Walkway,
        EdgeAccessRule::default(),
    )
    .unwrap();
    let exact_max_route = topo.find_route(a, d, &AccessProfile::standard()).unwrap();
    assert_eq!(exact_max_route.total_cost, u64::MAX);

    // Costs that together sum to (u64::MAX + 5), exceeding u64 capacity
    topo.add_edge(
        1,
        a,
        b,
        u64::MAX - 5,
        SpatialEdgeKind::Walkway,
        EdgeAccessRule::default(),
    )
    .unwrap();
    topo.add_edge(
        2,
        b,
        c,
        10,
        SpatialEdgeKind::Walkway,
        EdgeAccessRule::default(),
    )
    .unwrap();

    let err = topo
        .find_route(a, c, &AccessProfile::standard())
        .unwrap_err();
    assert_eq!(err, RoutingError::CostOverflow);
}

#[test]
fn test_duration_unit_validation() {
    let mut topo = SpatialTopology::new(CostUnit::Meters);
    let a = EntityId::new(1, 0);
    let b = EntityId::new(2, 0);

    topo.add_node(a, 0, SpatialNodeKind::Room).unwrap();
    topo.add_node(b, 0, SpatialNodeKind::Room).unwrap();
    topo.add_edge(
        1,
        a,
        b,
        50,
        SpatialEdgeKind::Walkway,
        EdgeAccessRule::default(),
    )
    .unwrap();

    let route = topo.find_route(a, b, &AccessProfile::standard()).unwrap();
    assert_eq!(route.unit, CostUnit::Meters);
    assert_eq!(
        route.duration().unwrap_err(),
        RoutingError::UnitMismatch {
            expected: CostUnit::Ticks,
            actual: CostUnit::Meters,
        }
    );
}

#[test]
fn test_multifloor_vertical_connectivity_and_step_free_filtering() {
    let mut topo = SpatialTopology::new(CostUnit::Meters);

    // Floor 0
    let ward_f0 = EntityId::new(101, 0);
    let lift_f0 = EntityId::new(102, 0);
    let stairs_f0 = EntityId::new(103, 0);

    // Floor 1
    let ward_f1 = EntityId::new(201, 0);
    let lift_f1 = EntityId::new(202, 0);
    let stairs_f1 = EntityId::new(203, 0);

    topo.add_node(ward_f0, 0, SpatialNodeKind::Room).unwrap();
    topo.add_node(lift_f0, 0, SpatialNodeKind::LiftLobby)
        .unwrap();
    topo.add_node(stairs_f0, 0, SpatialNodeKind::Stairwell)
        .unwrap();

    topo.add_node(ward_f1, 1, SpatialNodeKind::Room).unwrap();
    topo.add_node(lift_f1, 1, SpatialNodeKind::LiftLobby)
        .unwrap();
    topo.add_node(stairs_f1, 1, SpatialNodeKind::Stairwell)
        .unwrap();

    // Horizontal connections on Floor 0
    topo.add_edge(
        1,
        ward_f0,
        lift_f0,
        20,
        SpatialEdgeKind::Walkway,
        EdgeAccessRule::default(),
    )
    .unwrap();
    topo.add_edge(
        2,
        ward_f0,
        stairs_f0,
        5,
        SpatialEdgeKind::Walkway,
        EdgeAccessRule::default(),
    )
    .unwrap();

    // Horizontal connections on Floor 1
    topo.add_edge(
        3,
        lift_f1,
        ward_f1,
        20,
        SpatialEdgeKind::Walkway,
        EdgeAccessRule::default(),
    )
    .unwrap();
    topo.add_edge(
        4,
        stairs_f1,
        ward_f1,
        5,
        SpatialEdgeKind::Walkway,
        EdgeAccessRule::default(),
    )
    .unwrap();

    // Vertical connectors:
    // Stairs: cost 10, NOT step-free
    let stairs_rule = EdgeAccessRule {
        min_clearance: 0,
        step_free: false,
        is_open: true,
    };
    topo.add_edge(
        10,
        stairs_f0,
        stairs_f1,
        10,
        SpatialEdgeKind::Stairs,
        stairs_rule,
    )
    .unwrap();

    // Lift: cost 50 (slower traversal cost), step-free
    let lift_rule = EdgeAccessRule {
        min_clearance: 0,
        step_free: true,
        is_open: true,
    };
    topo.add_edge(20, lift_f0, lift_f1, 50, SpatialEdgeKind::Lift, lift_rule)
        .unwrap();

    // Standard profile (e.g. mobile staff) should take the faster stairs:
    // 5 (ward->stairs) + 10 (stairs) + 5 (stairs->ward) = 20 meters
    let standard_route = topo
        .find_route(ward_f0, ward_f1, &AccessProfile::standard())
        .unwrap();
    assert_eq!(standard_route.total_cost, 20);
    assert_eq!(
        standard_route.path,
        vec![ward_f0, stairs_f0, stairs_f1, ward_f1]
    );

    // Step-free profile (e.g. wheelchair patient or trolley) cannot use stairs; must take lift:
    // 20 (ward->lift) + 50 (lift) + 20 (lift->ward) = 90 meters
    let step_free_route = topo
        .find_route(ward_f0, ward_f1, &AccessProfile::step_free())
        .unwrap();
    assert_eq!(step_free_route.total_cost, 90);
    assert_eq!(
        step_free_route.path,
        vec![ward_f0, lift_f0, lift_f1, ward_f1]
    );
}

#[test]
fn test_clearance_and_door_state_filtering() {
    let mut topo = SpatialTopology::new(CostUnit::Ticks);
    let public_lobby = EntityId::new(1, 0);
    let corridor = EntityId::new(2, 0);
    let icu_room = EntityId::new(3, 0);

    topo.add_node(public_lobby, 0, SpatialNodeKind::Room)
        .unwrap();
    topo.add_node(corridor, 0, SpatialNodeKind::Corridor)
        .unwrap();
    topo.add_node(icu_room, 0, SpatialNodeKind::Room).unwrap();

    // Public walkway into corridor
    topo.add_edge(
        1,
        public_lobby,
        corridor,
        10,
        SpatialEdgeKind::Walkway,
        EdgeAccessRule::default(),
    )
    .unwrap();

    // Secure door into ICU (requires clearance level 3)
    let secure_door_rule = EdgeAccessRule {
        min_clearance: 3,
        step_free: true,
        is_open: true,
    };
    topo.add_edge(
        2,
        corridor,
        icu_room,
        5,
        SpatialEdgeKind::Doorway,
        secure_door_rule,
    )
    .unwrap();

    // Visitor profile (clearance 0) cannot access ICU
    assert_eq!(
        topo.find_route(public_lobby, icu_room, &AccessProfile::standard())
            .unwrap_err(),
        RoutingError::NoRouteFound {
            from: public_lobby,
            to: icu_room
        }
    );

    // Staff profile (clearance 3) can access ICU
    let staff_route = topo
        .find_route(public_lobby, icu_room, &AccessProfile::staff(3))
        .unwrap();
    assert_eq!(staff_route.total_cost, 15);
    assert_eq!(staff_route.path, vec![public_lobby, corridor, icu_room]);

    // Now close/lock the door (edge 2)
    topo.set_door_state(2, false).unwrap();

    // Even staff with clearance 3 is blocked by locked door
    assert_eq!(
        topo.find_route(public_lobby, icu_room, &AccessProfile::staff(3))
            .unwrap_err(),
        RoutingError::NoRouteFound {
            from: public_lobby,
            to: icu_room
        }
    );
}

#[test]
fn test_topology_revision_and_route_cache_invalidation() {
    let mut topo = SpatialTopology::new(CostUnit::Ticks);
    let n1 = EntityId::new(1, 0);
    let n2 = EntityId::new(2, 0);
    let n3 = EntityId::new(3, 0);

    topo.add_node(n1, 0, SpatialNodeKind::Room).unwrap();
    topo.add_node(n2, 0, SpatialNodeKind::Corridor).unwrap();
    topo.add_node(n3, 0, SpatialNodeKind::Room).unwrap();

    topo.add_edge(
        1,
        n1,
        n2,
        10,
        SpatialEdgeKind::Walkway,
        EdgeAccessRule::default(),
    )
    .unwrap();
    topo.add_edge(
        2,
        n2,
        n3,
        10,
        SpatialEdgeKind::Walkway,
        EdgeAccessRule::default(),
    )
    .unwrap();

    let initial_version = topo.version();

    // Query populates cache
    let r1 = topo.find_route(n1, n3, &AccessProfile::standard()).unwrap();
    assert_eq!(r1.total_cost, 20);

    // Mutating edge cost bumps topology version and invalidates cache
    topo.set_edge_cost(2, 50).unwrap();
    assert!(topo.version() > initial_version);

    let r2 = topo.find_route(n1, n3, &AccessProfile::standard()).unwrap();
    assert_eq!(r2.total_cost, 60, "Route must reflect updated edge cost");
}

#[test]
fn test_node_and_edge_validation_errors() {
    let mut topo = SpatialTopology::new(CostUnit::Ticks);
    let e1 = EntityId::new(1, 0);
    let e2 = EntityId::new(2, 0);

    topo.add_node(e1, 0, SpatialNodeKind::Room).unwrap();
    assert_eq!(
        topo.add_node(e1, 0, SpatialNodeKind::Room).unwrap_err(),
        RoutingError::DuplicateNode(e1)
    );

    // Edge connecting nonexistent node
    assert_eq!(
        topo.add_edge(
            1,
            e1,
            e2,
            10,
            SpatialEdgeKind::Walkway,
            EdgeAccessRule::default()
        )
        .unwrap_err(),
        RoutingError::NodeNotFound(e2)
    );

    topo.add_node(e2, 0, SpatialNodeKind::Room).unwrap();
    topo.add_edge(
        1,
        e1,
        e2,
        10,
        SpatialEdgeKind::Walkway,
        EdgeAccessRule::default(),
    )
    .unwrap();

    // Duplicate edge ID
    assert_eq!(
        topo.add_edge(
            1,
            e1,
            e2,
            20,
            SpatialEdgeKind::Walkway,
            EdgeAccessRule::default()
        )
        .unwrap_err(),
        RoutingError::DuplicateEdgeId(1)
    );

    // Node self-route
    let self_route = topo.find_route(e1, e1, &AccessProfile::standard()).unwrap();
    assert_eq!(self_route.total_cost, 0);
    assert_eq!(self_route.hop_count, 0);
    assert_eq!(self_route.path, vec![e1]);
}

#[test]
fn test_des_boundary_route_rendering_does_not_schedule_events() {
    let mut sim_ctx = ABMContext::new(42);
    let mut topo = SpatialTopology::new(CostUnit::Ticks);

    let a = sim_ctx.spawn_agent();
    let b = sim_ctx.spawn_agent();

    topo.add_node(a, 0, SpatialNodeKind::Room).unwrap();
    topo.add_node(b, 0, SpatialNodeKind::Room).unwrap();
    topo.add_edge(
        1,
        a,
        b,
        25,
        SpatialEdgeKind::Walkway,
        EdgeAccessRule::default(),
    )
    .unwrap();

    // Before routing: scheduler queue is empty
    assert!(matches!(
        sim_ctx.step(),
        kairo_ecs_types::StepOutcome::Empty
    ));

    // Execute 100 routing queries
    for _ in 0..100 {
        let route = topo.find_route(a, b, &AccessProfile::standard()).unwrap();
        assert_eq!(route.total_cost, 25);
    }

    // Verify DES boundary: route calculation does NOT schedule events or alter DES kernel state
    assert!(
        matches!(sim_ctx.step(), kairo_ecs_types::StepOutcome::Empty),
        "Route calculation must remain purely spatial and never schedule events in the DES scheduler"
    );

    // Traversal cost is expressed in SimDuration for the agent behavior to schedule explicitly
    let route = topo.find_route(a, b, &AccessProfile::standard()).unwrap();
    let duration = route.duration().expect("ticks unit converts to duration");
    sim_ctx.schedule_behaviour(a, 999, SimTime::from_ticks(duration.ticks()));
    assert!(
        matches!(sim_ctx.step(), kairo_ecs_types::StepOutcome::Dispatched(ev) if ev.at == SimTime::from_ticks(25)),
        "Agent behavior explicitly schedules traversal event using route duration"
    );
}

#[test]
fn test_graphspace_csr_interoperability() {
    let mut topo = SpatialTopology::new(CostUnit::Ticks);
    let n1 = EntityId::new(1, 0);
    let n2 = EntityId::new(2, 0);
    let n3 = EntityId::new(3, 0);

    topo.add_node(n1, 0, SpatialNodeKind::Room).unwrap();
    topo.add_node(n2, 0, SpatialNodeKind::Room).unwrap();
    topo.add_node(n3, 0, SpatialNodeKind::Room).unwrap();

    topo.add_edge(
        1,
        n1,
        n2,
        10,
        SpatialEdgeKind::Walkway,
        EdgeAccessRule::default(),
    )
    .unwrap();
    topo.add_edge(
        2,
        n1,
        n3,
        15,
        SpatialEdgeKind::Walkway,
        EdgeAccessRule::default(),
    )
    .unwrap();
    topo.add_edge(
        3,
        n2,
        n3,
        5,
        SpatialEdgeKind::Walkway,
        EdgeAccessRule::default(),
    )
    .unwrap();

    // Export to GraphSpace CSR view
    let gspace = GraphSpace::from_spatial_topology(&topo);
    let n1_neighbors = gspace.neighbors(n1);
    assert_eq!(n1_neighbors.len(), 2);
    assert!(n1_neighbors.contains(&n2));
    assert!(n1_neighbors.contains(&n3));

    let n2_neighbors = gspace.neighbors(n2);
    assert_eq!(n2_neighbors, &[n3]);
}
