//! Comprehensive test oracles for geo geometry and route-position primitives (Track 87).
//!
//! Validates:
//! 1. Coordinate validation (finite bounds, invalid lat/lon rejection).
//! 2. Coordinate ordering (explicit (lat, lon) constructor and GIS Point (x=lon, y=lat) conversion).
//! 3. Distance and bearing oracles against known geodetic benchmarks.
//! 4. Local tangent plane forward projection and inverse unprojection millimetric roundtrip.
//! 5. Route distance calculation (single segment, zero-length segment, multi-segment).
//! 6. Route position interpolation at exact distance and normalized progress t in [0.0, 1.0].
//! 7. Endpoint clamping and negative/non-finite distance error handling.
//! 8. Monotonic route progress along multi-segment paths.
//! 9. Planar Cartesian route interpolation and error handling.
//! 10. SpatialTopology node coordinate integration.

use kairo_ecs_abm::geometry::{
    CoordinateSystem, GeoCoordinate, GeoRoute, GeometryError, LocalTangentPlaneProjection,
    ProjectedCoordinate, ProjectedRoute,
};
use kairo_ecs_abm::routing::{
    CostUnit, EdgeAccessRule, SpatialEdgeKind, SpatialNodeKind, SpatialTopology,
};

#[test]
fn test_coordinate_validation_and_bounds() {
    // Valid coordinates
    let london = GeoCoordinate::try_new(51.5074, -0.1278).unwrap();
    assert!((london.lat() - 51.5074).abs() < 1e-6);
    assert!((london.lon() - (-0.1278)).abs() < 1e-6);

    let equator_origin = GeoCoordinate::try_new(0.0, 0.0).unwrap();
    assert_eq!(equator_origin.lat(), 0.0);
    assert_eq!(equator_origin.lon(), 0.0);

    let south_pole = GeoCoordinate::try_new(-90.0, 0.0).unwrap();
    assert_eq!(south_pole.lat(), -90.0);

    let north_pole = GeoCoordinate::try_new(90.0, 180.0).unwrap();
    assert_eq!(north_pole.lat(), 90.0);
    assert_eq!(north_pole.lon(), 180.0);

    // Invalid latitudes
    assert!(matches!(
        GeoCoordinate::try_new(90.001, 0.0),
        Err(GeometryError::InvalidCoordinate { reason, .. }) if reason.contains("latitude")
    ));
    assert!(matches!(
        GeoCoordinate::try_new(-90.001, 0.0),
        Err(GeometryError::InvalidCoordinate { reason, .. }) if reason.contains("latitude")
    ));

    // Invalid longitudes
    assert!(matches!(
        GeoCoordinate::try_new(0.0, 180.001),
        Err(GeometryError::InvalidCoordinate { reason, .. }) if reason.contains("longitude")
    ));
    assert!(matches!(
        GeoCoordinate::try_new(0.0, -180.001),
        Err(GeometryError::InvalidCoordinate { reason, .. }) if reason.contains("longitude")
    ));

    // Non-finite coordinates
    assert!(matches!(
        GeoCoordinate::try_new(f64::NAN, 0.0),
        Err(GeometryError::InvalidCoordinate { .. })
    ));
    assert!(matches!(
        GeoCoordinate::try_new(0.0, f64::INFINITY),
        Err(GeometryError::InvalidCoordinate { .. })
    ));
}

#[test]
fn test_coordinate_ordering_and_gis_point_conversion() {
    // Standard GIS point: x = longitude, y = latitude
    let coord = GeoCoordinate::try_new(52.5200, 13.4050).unwrap(); // Berlin: lat 52.52, lon 13.405
    let point = coord.to_point();

    assert_eq!(point.x(), 13.4050); // x is lon
    assert_eq!(point.y(), 52.5200); // y is lat

    let recovered = GeoCoordinate::from_point(point).unwrap();
    assert_eq!(recovered.lat(), 52.5200);
    assert_eq!(recovered.lon(), 13.4050);

    // CoordinateSystem declaration
    let wgs84 = CoordinateSystem::GeographicWgs84;
    assert_eq!(wgs84, CoordinateSystem::GeographicWgs84);

    let planar = CoordinateSystem::ProjectedPlanar {
        projection_name: "EPSG:3857".to_string(),
    };
    assert!(matches!(planar, CoordinateSystem::ProjectedPlanar { .. }));
}

#[test]
fn test_geodesic_and_haversine_benchmarks() {
    // Benchmark: London to Paris
    let london = GeoCoordinate::try_new(51.5074, -0.1278).unwrap();
    let paris = GeoCoordinate::try_new(48.8566, 2.3522).unwrap();

    let geodesic_dist = london.geodesic_distance_to(&paris);
    let haversine_dist = london.haversine_distance_to(&paris);

    // Geodesic distance on WGS84 ellipsoid is ~343.5 km
    assert!(
        (geodesic_dist - 343_556.0).abs() < 1_000.0,
        "Geodesic distance London-Paris was {geodesic_dist} m"
    );

    // Spherical Haversine distance is within 0.5% of geodesic
    let rel_diff = (geodesic_dist - haversine_dist).abs() / geodesic_dist;
    assert!(
        rel_diff < 0.005,
        "Relative difference between Haversine and Geodesic was {rel_diff}"
    );

    // Initial bearing from London to Paris is ~149.0 degrees (SSE)
    let geodesic_bearing = london.geodesic_bearing_to(&paris);
    let haversine_bearing = london.haversine_bearing_to(&paris);

    assert!(
        (geodesic_bearing - 149.0).abs() < 2.0,
        "Geodesic bearing was {geodesic_bearing} deg"
    );
    assert!(
        (haversine_bearing - 149.0).abs() < 2.0,
        "Haversine bearing was {haversine_bearing} deg"
    );

    // Equator 1-degree meridian arc is approximately 111,139 meters
    let eq0 = GeoCoordinate::try_new(0.0, 0.0).unwrap();
    let eq1 = GeoCoordinate::try_new(1.0, 0.0).unwrap();
    let meridian_1deg = eq0.geodesic_distance_to(&eq1);
    assert!(
        (meridian_1deg - 110_574.0).abs() < 100.0,
        "Equatorial meridian 1-deg was {meridian_1deg} m"
    );
}

#[test]
fn test_local_tangent_plane_projection_roundtrip() {
    let origin = GeoCoordinate::try_new(37.7749, -122.4194).unwrap(); // San Francisco
    let ltp = LocalTangentPlaneProjection::new(origin);
    assert_eq!(ltp.origin(), origin);

    // Origin projects to (0, 0)
    let p0 = ltp.project(&origin);
    assert!(p0.x().abs() < 1e-9);
    assert!(p0.y().abs() < 1e-9);

    // Test a coordinate ~1.5 km offset
    let target = GeoCoordinate::try_new(37.7850, -122.4050).unwrap();
    let projected = ltp.project(&target);

    // Unproject back
    let unprojected = ltp.unproject(&projected).unwrap();
    assert!(
        (unprojected.lat() - target.lat()).abs() < 1e-9,
        "Lat roundtrip error: {}",
        (unprojected.lat() - target.lat()).abs()
    );
    assert!(
        (unprojected.lon() - target.lon()).abs() < 1e-9,
        "Lon roundtrip error: {}",
        (unprojected.lon() - target.lon()).abs()
    );

    // Euclidean distance in tangent plane matches projected coordinate distance
    let dist_planar = p0.euclidean_distance_to(&projected);
    assert!(dist_planar > 1_000.0 && dist_planar < 2_000.0);
}

#[test]
fn test_geo_route_distances_and_segments() {
    let p1 = GeoCoordinate::try_new(0.0, 0.0).unwrap();
    let p2 = GeoCoordinate::try_new(0.0, 1.0).unwrap(); // ~111.3 km East along equator
    let p3 = GeoCoordinate::try_new(1.0, 1.0).unwrap(); // ~110.6 km North

    // Empty route fails
    assert!(matches!(
        GeoRoute::try_new(vec![]),
        Err(GeometryError::EmptyRoute)
    ));

    // Single waypoint route has distance 0.0
    let single_route = GeoRoute::try_new(vec![p1]).unwrap();
    assert_eq!(single_route.waypoint_count(), 1);
    assert_eq!(single_route.total_distance_haversine(), 0.0);
    assert_eq!(single_route.total_distance_geodesic(), 0.0);

    // Multi-segment route
    let route = GeoRoute::try_new(vec![p1, p2, p3]).unwrap();
    assert_eq!(route.waypoint_count(), 3);

    let d12 = p1.haversine_distance_to(&p2);
    let d23 = p2.haversine_distance_to(&p3);
    let total_haversine = route.total_distance_haversine();
    assert!((total_haversine - (d12 + d23)).abs() < 1e-6);

    let total_geodesic = route.total_distance_geodesic();
    assert!(
        (total_geodesic - (p1.geodesic_distance_to(&p2) + p2.geodesic_distance_to(&p3))).abs()
            < 1e-6
    );

    // Zero-length segment (repeated waypoint) does not break calculation
    let route_with_dup = GeoRoute::try_new(vec![p1, p1, p2, p3]).unwrap();
    assert!((route_with_dup.total_distance_haversine() - total_haversine).abs() < 1e-6);
}

#[test]
fn test_geo_route_interpolation_and_monotonicity() {
    let p1 = GeoCoordinate::try_new(0.0, 0.0).unwrap();
    let p2 = GeoCoordinate::try_new(0.0, 1.0).unwrap();
    let p3 = GeoCoordinate::try_new(0.0, 2.0).unwrap();

    let route = GeoRoute::try_new(vec![p1, p2, p3]).unwrap();
    let total = route.total_distance_haversine();

    // Start of route
    let at_start = route.interpolate_distance(0.0).unwrap();
    assert!((at_start.lat() - p1.lat()).abs() < 1e-6);
    assert!((at_start.lon() - p1.lon()).abs() < 1e-6);

    // Exact midpoint
    let at_half = route.interpolate_normalized(0.5).unwrap();
    assert!((at_half.lat() - p2.lat()).abs() < 1e-6);
    assert!((at_half.lon() - p2.lon()).abs() < 1e-4);

    // End of route
    let at_end = route.interpolate_distance(total).unwrap();
    assert!((at_end.lat() - p3.lat()).abs() < 1e-6);
    assert!((at_end.lon() - p3.lon()).abs() < 1e-6);

    // Clamping beyond route length
    let clamped = route.interpolate_distance(total * 2.0).unwrap();
    assert_eq!(clamped, p3);

    // Negative distance error
    assert!(matches!(
        route.interpolate_distance(-5.0),
        Err(GeometryError::NegativeDistance(d)) if d == -5.0
    ));

    // Progress out of range error
    assert!(matches!(
        route.interpolate_normalized(1.05),
        Err(GeometryError::NormalizedProgressOutOfRange(p)) if p == 1.05
    ));
    assert!(matches!(
        route.interpolate_normalized(-0.01),
        Err(GeometryError::NormalizedProgressOutOfRange(p)) if p == -0.01
    ));

    // Non-finite parameter error
    assert!(matches!(
        route.interpolate_distance(f64::NAN),
        Err(GeometryError::InvalidParameter(_))
    ));
    assert!(matches!(
        route.interpolate_normalized(f64::INFINITY),
        Err(GeometryError::InvalidParameter(_))
    ));

    // Monotonicity check along 100 sample points
    let mut prev_lon = -1.0;
    for step in 0..=100 {
        let t = step as f64 / 100.0;
        let pos = route.interpolate_normalized(t).unwrap();
        assert!(
            pos.lon() >= prev_lon - 1e-9,
            "Monotonicity violation at t={t}: {} < {prev_lon}",
            pos.lon()
        );
        prev_lon = pos.lon();
    }
}

#[test]
fn test_projected_planar_route_and_interpolation() {
    let p1 = ProjectedCoordinate::try_new(0.0, 0.0).unwrap();
    let p2 = ProjectedCoordinate::try_new(100.0, 0.0).unwrap();
    let p3 = ProjectedCoordinate::try_new(100.0, 200.0).unwrap();

    let route = ProjectedRoute::try_new(vec![p1, p2, p3]).unwrap();
    assert_eq!(route.waypoint_count(), 3);
    assert_eq!(route.total_length(), 300.0);

    // Interpolate at 50m (on first segment)
    let pos_50 = route.interpolate_distance(50.0).unwrap();
    assert!((pos_50.x() - 50.0).abs() < 1e-6);
    assert!((pos_50.y() - 0.0).abs() < 1e-6);

    // Interpolate at 150m (on second segment)
    let pos_150 = route.interpolate_distance(150.0).unwrap();
    assert!((pos_150.x() - 100.0).abs() < 1e-6);
    assert!((pos_150.y() - 50.0).abs() < 1e-6);

    // Interpolate at normalized progress 1.0
    let pos_end = route.interpolate_normalized(1.0).unwrap();
    assert!((pos_end.x() - 100.0).abs() < 1e-6);
    assert!((pos_end.y() - 200.0).abs() < 1e-6);

    // Negative distance error
    assert!(matches!(
        route.interpolate_distance(-10.0),
        Err(GeometryError::NegativeDistance(d)) if d == -10.0
    ));

    // Normalized progress bounds
    assert!(matches!(
        route.interpolate_normalized(1.1),
        Err(GeometryError::NormalizedProgressOutOfRange(_))
    ));
}

#[test]
fn test_spatial_topology_with_geocoordinate_waypoints() {
    use kairo_ecs_abm::routing::AccessProfile;
    use kairo_ecs_types::EntityId;

    // Demonstrates integration of GeoCoordinate with SpatialTopology nodes
    let mut topo = SpatialTopology::new(CostUnit::Meters);

    let origin = GeoCoordinate::try_new(51.5074, -0.1278).unwrap(); // London
    let n0 = EntityId::new(10, 0);
    let n1 = EntityId::new(20, 0);
    let n2 = EntityId::new(30, 0);

    topo.add_node(n0, 0, SpatialNodeKind::WayPoint).unwrap();
    topo.add_node(n1, 0, SpatialNodeKind::WayPoint).unwrap();
    topo.add_node(n2, 0, SpatialNodeKind::WayPoint).unwrap();

    let ltp = LocalTangentPlaneProjection::new(origin);

    // Add edges with exact geodesic distance costs
    let coord0 = origin;
    let coord1 = GeoCoordinate::try_new(51.5100, -0.1250).unwrap();
    let coord2 = GeoCoordinate::try_new(51.5150, -0.1200).unwrap();

    let cost01 = coord0.geodesic_distance_to(&coord1).round() as u64;
    let cost12 = coord1.geodesic_distance_to(&coord2).round() as u64;

    topo.add_edge(
        1,
        n0,
        n1,
        cost01,
        SpatialEdgeKind::Walkway,
        EdgeAccessRule::default(),
    )
    .unwrap();
    topo.add_edge(
        2,
        n1,
        n2,
        cost12,
        SpatialEdgeKind::Walkway,
        EdgeAccessRule::default(),
    )
    .unwrap();

    // Route finding produces exact cumulative distance
    let profile = AccessProfile::default();
    let route = topo.find_route(n0, n2, &profile).unwrap();
    assert_eq!(route.total_cost, cost01 + cost12);
    assert_eq!(route.path, vec![n0, n1, n2]);

    // Construct GeoRoute from node coordinates
    let geo_route = GeoRoute::try_new(vec![coord0, coord1, coord2]).unwrap();
    assert!((geo_route.total_distance_geodesic() - (cost01 + cost12) as f64).abs() < 2.0);

    // Project route coordinates to planar Cartesian coordinates
    let planar_waypoints: Vec<ProjectedCoordinate> = geo_route
        .waypoints()
        .iter()
        .map(|c| ltp.project(c))
        .collect();
    let projected_route = ProjectedRoute::try_new(planar_waypoints).unwrap();
    assert_eq!(projected_route.waypoint_count(), 3);
}
