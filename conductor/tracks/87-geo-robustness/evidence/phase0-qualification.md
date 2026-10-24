# Phase 0 Qualification: geo Geometry and Route-Position Primitives

## Track: 87-geo-robustness
## Date: 2026-10-25

## 1. Candidate Qualification
- **Crate**: `geo`
- **Candidate Version**: `0.33.1`
- **Features Selected**: `default-features = false`
- **License**: `MIT OR Apache-2.0` (fully compliant with workspace license policy)
- **MSRV**: 1.70+ (compatible with workspace Rust 1.99.1)
- **Dependencies Introduced**: Pure Rust packages (`geo-types 0.7.20`, `geographiclib-rs 0.2.7`, `robust 1.2.0`, `float_next_after 2.0.0`, `approx 0.5.1`).
- **Exclusions**: Heavy C bindings (`proj`, `gdal`), rendering, and unused geometry algorithms omitted.

## 2. Supply Chain & Security Verification
- `cargo deny check advisories`: 0 security advisories detected.
- `cargo deny check sources`: all crates resolved from crates.io.
- `cargo deny check licenses`: Apache-2.0 / MIT confirmed across all transitive dependencies.
- `cargo deny check bans`: 0 banned crates or duplicate version violations.
- `cargo audit`: 0 vulnerabilities.

## 3. Scope of Replacement & Contract
- Replaces ad-hoc geographic coordinate formulas and unvalidated distance approximations with verified WGS84 geodesic algorithms (Karney's exact geodesic via `geographiclib-rs`) and spherical Haversine distances.
- Provides invariant-checked `GeoCoordinate`, `ProjectedCoordinate`, `CoordinateSystem`, `LocalTangentPlaneProjection`, `GeoRoute`, and `ProjectedRoute`.
- Explicit GIS ordering: point constructor and conversion strictly map `x = lon, y = lat`.
- Route interpolation handles zero-length segments, multi-segment cumulative progress, endpoint clamping, and negative/non-finite distance rejections.
- Integrates seamlessly with `SpatialTopology` without conflating straight-line distance with simulation ticks or traffic rules.

## 4. Test Oracles Defined
Executed in `crates/kairo-ecs-abm/tests/geo_geometry_oracles.rs`:
- `test_coordinate_validation_and_bounds`
- `test_coordinate_ordering_and_gis_point_conversion`
- `test_geodesic_and_haversine_benchmarks`
- `test_local_tangent_plane_projection_roundtrip`
- `test_geo_route_distances_and_segments`
- `test_geo_route_interpolation_and_monotonicity`
- `test_projected_planar_route_and_interpolation`
- `test_spatial_topology_with_geocoordinate_waypoints`
