# ADR, Compatibility Assessment & Red-Team Review: geo Geometry and Route-Position Primitives

## Track: 87-geo-robustness

## Status
Accepted

## Context
KairoECS discrete-event simulation (DES) and agent-based modeling (ABM) require robust spatial primitives to represent geographic locations (WGS84 latitude/longitude), local tangent plane projected Cartesian coordinates, and continuous route interpolation along multi-segment paths. Previous approaches risk ad-hoc GIS formulas with spherical inaccuracies, coordinate order confusion (latitude vs longitude order), unvalidated out-of-range degrees, and conflation between straight-line distance and network travel time.

Track 87 qualifies `geo = { version = "0.33.1", default-features = false }` to supply pure Rust implementations of WGS84 ellipsoidal Geodesic calculations (`geographiclib-rs`), spherical Haversine distances and bearings, and line measures interpolation without C dependencies or unvetted unsafe code.

## Architecture Decision
1. **Explicit Coordinate Types with Invariant Validation**:
   - `GeoCoordinate`: Validated WGS84 coordinates in degrees. Latitude is strictly bounded to $[-90.0, +90.0]$ and longitude to $[-180.0, +180.0]$. NaN and infinite values are rejected with explicit `GeometryError::InvalidCoordinate`.
   - `ProjectedCoordinate`: Validated 2D Cartesian planar coordinates in meters. Non-finite values are rejected.
   - `CoordinateSystem`: Explicit enum distinguishing `GeographicWgs84` from `ProjectedPlanar { projection_name }`.

2. **Standard GIS Point Interoperability**:
   - Standard GIS convention uses `(x = lon, y = lat)`.
   - `GeoCoordinate::to_point()` outputs `geo::Point::new(lon, lat)`.
   - `GeoCoordinate::from_point(p)` extracts `lat = p.y(), lon = p.x()`.
   - Constructor `GeoCoordinate::try_new(lat, lon)` uses standard human-readable `(latitude, longitude)` ordering, eliminating transposition errors.

3. **Geodesic and Haversine Distance & Bearing**:
   - `geodesic_distance_to(&self, other)`: High-precision ellipsoidal geodesic distance in meters via Karney's algorithm on WGS84.
   - `haversine_distance_to(&self, other)`: Fast spherical great-circle distance in meters.
   - `geodesic_bearing_to(&self, other)` and `haversine_bearing_to(&self, other)`: Initial azimuth in degrees $[0.0, 360.0)$.

4. **Local Tangent Plane Projection**:
   - `LocalTangentPlaneProjection`: Projects WGS84 geographic coordinates onto a local planar Cartesian frame centered at an origin coordinate.
   - Unprojection recovers original coordinates with sub-millimeter precision ($< 10^{-4}$ meters) within regional simulation extents.

5. **Multi-Segment Route Interpolation**:
   - `GeoRoute` and `ProjectedRoute`: Support continuous position interpolation via `interpolate_distance(meters)` and `interpolate_normalized(t)` where $t \in [0.0, 1.0]$.
   - Negative distances return `GeometryError::NegativeDistance`.
   - Normalized parameters outside $[0.0, 1.0]$ return `GeometryError::NormalizedProgressOutOfRange`.
   - Distances exceeding total path length are clamped safely to the terminal waypoint.
   - Monotonic progress along routes is preserved.

6. **Contract Boundaries**:
   - Geometry code strictly deals with spatial distances, bearings, and positions.
   - Straight-line distance is never conflated with road travel time.
   - No traffic rules, speed limits, or simulation events are introduced in geometric primitives.

## Compatibility Assessment
- **Breaking Changes**: None. This introduces new types under `kairo_ecs_abm::geometry` and exports them at crate root.
- **Interoperability with SpatialTopology**: `SpatialTopology` nodes and edge costs can directly store or derive from `GeoCoordinate` geodesic distances, as validated in test oracles.
- **FFI & Polyglot Readiness**: Primitives are structured as plain data with float accessors, readily mappable across C/UniFFI/Diplomat bridges.

## Red-Team / Devil's Advocate Review
- *Risk 1: Could polar singularities or longitude anti-meridian wrap-around cause undefined behaviour in route interpolation?*
  - **Mitigation**: Bounds checking enforces finite coordinates within standard WGS84 ranges. Interpolation uses `geo::line_measures::InterpolatePoint` which operates along spherical/ellipsoidal geodesics without polar singularities.
- *Risk 2: Could zero-length segments or duplicate waypoints cause division by zero during ratio computation?*
  - **Mitigation**: Route traversal loops explicitly check `if seg_dist == 0.0 { continue; }`, avoiding any division by zero when duplicate waypoints exist.
- *Risk 3: Could excessive dependencies or native C libraries bloat the binary or break MSRV?*
  - **Mitigation**: `geo` is imported with `default-features = false`. Pure Rust dependencies only (`geo-types`, `geographiclib-rs`). Zero C bindings. `cargo deny check` passes with 0 license or ban violations.
