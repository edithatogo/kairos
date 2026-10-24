# Handoff: geo Geometry and Route-Position Primitives

Updated: 2026-10-25.

## Summary

Track 87 qualifies and adopts `geo = { version = "0.33.1", default-features = false }` in `crates/kairo-ecs-abm` for spatial geometry and continuous route interpolation. It introduces `GeoCoordinate` (WGS84 degrees validation, geodesic and spherical Haversine distances and bearings), `ProjectedCoordinate` (2D Cartesian planar meters), `CoordinateSystem` declarations, `LocalTangentPlaneProjection` (millimetric forward and inverse unprojection), `GeoRoute` and `ProjectedRoute` (cumulative path distances and continuous position interpolation with zero-length segment resilience, endpoint clamping, and negative distance validation), and seamless integration with `SpatialTopology` waypoints.

## Follow-up issues

Downstream CareOps and KairoECS ABM spatial simulation tracks (e.g. Track 86 `rstar` spatial indexing, agent continuous movement systems) consume `GeoCoordinate` and `GeoRoute` primitives directly from `kairo_ecs_abm`.

## Integration notes

Spatial geometry primitives are isolated within `crates/kairo-ecs-abm::geometry`. Geodesic calculations use pure Rust `geographiclib-rs` and `geo-types`, omitting heavy C dependencies (e.g. PROJ/GDAL). Spatial routes operate purely geometrically and never schedule discrete-event simulation events in the DES kernel.

## Phase closeout evidence

- Conductor review: Passed (`$conductor-review` in `review-report.md`).
- Accepted fixes: No outstanding findings; 3 findings documented and resolved in review report (`accepted fixes` verified).
- Validation commands:
  - `cargo test -p kairo-ecs-abm` (8/8 new independent oracles pass, 38/38 total tests pass)
  - `cargo test --workspace` (all workspace tests pass)
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` (clean)
  - `cargo fmt --all --check` (clean)
  - `cargo deny check advisories sources licenses bans` (advisories ok, bans ok, licenses ok, sources ok)
  - `cargo audit` (0 vulnerabilities found)
  - `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` (pass)
  - `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_dag.ps1` (pass)
  - `pwsh -NoProfile -File scripts/validate_conductor_artifacts.ps1` (pass)
- Closeout ledger: commit SHA `c37c1c2f124a72e7645d311259899c77095dc3c4` recorded upon slice creation.
- Pushed ref: pushed ref `origin/feat/track-87-geo-primitives`.
- Git closeout verification: `pwsh scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`.
- Next-phase decision: Track 87 is Done; next-phase decision advances to Track 86 (`rstar spatial indexing evaluation`) / Track 79 (`zip archive format reader/writer adoption`).

## Files changed

- `crates/kairo-ecs-abm/Cargo.toml`
- `crates/kairo-ecs-abm/src/lib.rs`
- `crates/kairo-ecs-abm/src/geometry.rs`
- `crates/kairo-ecs-abm/tests/geo_geometry_oracles.rs`
- `Cargo.lock`
- `conductor/tracks/87-geo-robustness/spec.md`
- `conductor/tracks/87-geo-robustness/plan.md`
- `conductor/tracks/87-geo-robustness/metadata.json`
- `conductor/tracks/87-geo-robustness/handoff.md`
- `conductor/tracks/87-geo-robustness/review-report.md`
- `conductor/tracks/87-geo-robustness/adr-api-compatibility-and-redteam.md`
- `conductor/tracks/87-geo-robustness/evidence/phase0-qualification.md`
- `conductor/tracks.yaml`
- `conductor/tracks.md`
- `conductor/phase-closeout.yaml`
- `CHANGELOG.md`

## Contracts consumed

- `conductor/tracks/87-geo-robustness/spec.md`
- `conductor/dependency-value-policy.md`
- `conductor/dependency-sequencing.md`
- `geo` 0.33.1 algorithms (`Haversine`, `Geodesic`, `InterpolatePoint`, `Euclidean`, `Point`)

## Contracts changed

- `crates/kairo-ecs-abm`: Exported `geometry` module and types `GeoCoordinate`, `ProjectedCoordinate`, `CoordinateSystem`, `LocalTangentPlaneProjection`, `GeoRoute`, `ProjectedRoute`, `GeometryError`.

## Tests added

- `crates/kairo-ecs-abm/tests/geo_geometry_oracles.rs`:
  - `test_coordinate_validation_and_bounds`
  - `test_coordinate_ordering_and_gis_point_conversion`
  - `test_geodesic_and_haversine_benchmarks`
  - `test_local_tangent_plane_projection_roundtrip`
  - `test_geo_route_distances_and_segments`
  - `test_geo_route_interpolation_and_monotonicity`
  - `test_projected_planar_route_and_interpolation`
  - `test_spatial_topology_with_geocoordinate_waypoints`

## Known risks

No remaining risks. Zero unvetted dependencies; clean cargo deny and audit; GIS ordering and route interpolation invariants verified.
