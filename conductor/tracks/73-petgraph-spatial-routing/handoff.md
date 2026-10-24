# Handoff

Updated: 2026-10-25.

## Summary

Track 73 adopts pinned `petgraph = "0.7.1"` (features `["graphmap", "stable_graph"]`) in `crates/kairo-ecs-abm` for spatial topology modeling and deterministic Dijkstra routing. Features include 3-level deterministic route tie-breaking (cost -> hops -> edge ID), random insertion order invariance, zero-cost cycle resilience, multi-floor access filtering (clearance levels, step-free/wheelchair vertical transit), door state toggling, topology version cache invalidation, and DES event boundary preservation.

## Follow-up issues

Downstream CareOps routing tracks (R29–R31) may adopt the accepted Kairos Track 73 receipt for multi-floor hospital layouts and mobility profiles.

## Integration notes

Spatial topology and routing are self-contained within `crates/kairo-ecs-abm`. `GraphSpace::from_spatial_topology` provides bidirectional interop with CSR adjacency views. Route queries remain purely spatial and never schedule simulation events in the DES kernel.

## Phase closeout evidence

- Conductor review: Passed (`review-report.md`).
- Validation commands:
  - `cargo test -p kairo-ecs-abm` (12/12 new independent oracles pass, 29/29 total tests pass)
  - `cargo test --workspace` (all workspace tests pass)
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` (clean)
  - `cargo fmt --all --check` (clean)
  - `cargo deny check advisories sources licenses bans` (advisories ok, bans ok, licenses ok, sources ok)
  - `cargo audit` (0 vulnerabilities found)
  - `node tests/conformance/conformance-check.mjs` (pass)
  - `pwsh -NoProfile -File scripts/validate_cross_binding_conformance.ps1` (pass)
  - `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` (pass)
  - `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_dag.ps1` (pass)
  - `pwsh -NoProfile -File scripts/validate_conductor_artifacts.ps1` (pass)
- Git closeout verification: `pwsh scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`.
- Next-phase decision: Track 73 is Done; advance to Track 87 (`geo geometry and route-position primitives`) / Track 79 (`zip archive format reader/writer adoption`).

## Files changed

- `crates/kairo-ecs-abm/Cargo.toml`
- `crates/kairo-ecs-abm/src/lib.rs`
- `crates/kairo-ecs-abm/src/routing.rs`
- `crates/kairo-ecs-abm/tests/petgraph_routing_oracles.rs`
- `Cargo.lock`
- `conductor/tracks/73-petgraph-spatial-routing/plan.md`
- `conductor/tracks/73-petgraph-spatial-routing/metadata.json`
- `conductor/tracks/73-petgraph-spatial-routing/handoff.md`
- `conductor/tracks/73-petgraph-spatial-routing/review-report.md`
- `conductor/tracks/73-petgraph-spatial-routing/evidence/phase0-qualification.md`
- `conductor/tracks.yaml`
- `conductor/tracks.md`
- `conductor/status.md`
- `conductor/phase-closeout.yaml`
- `CHANGELOG.md`

## Contracts consumed

- `conductor/tracks/73-petgraph-spatial-routing/spec.md`
- `conductor/dependency-value-policy.md`
- `conductor/dependency-sequencing.md`

## Contracts changed

- `crates/kairo-ecs-abm`: Exported `routing` module with `SpatialTopology`, `SpatialRoute`, `RouteSegment`, `AccessProfile`, `EdgeAccessRule`, `CostUnit`, `RoutingError`, and `GraphSpace::from_spatial_topology`.

## Tests added

- `crates/kairo-ecs-abm/tests/petgraph_routing_oracles.rs`:
  - `test_small_graph_optimality`
  - `test_cost_hops_edge_id_tie_breaking`
  - `test_shuffled_insertion_order_invariance`
  - `test_zero_cost_cycle_resilience`
  - `test_directionality_and_disconnected`
  - `test_cost_overflow_rejection`
  - `test_multifloor_vertical_connectivity_and_step_free_filtering`
  - `test_clearance_and_door_state_filtering`
  - `test_topology_revision_and_route_cache_invalidation`
  - `test_node_and_edge_validation_errors`
  - `test_des_boundary_route_rendering_does_not_schedule_events`
  - `test_graphspace_csr_interoperability`

## Known risks

No remaining risks. Zero unvetted dependencies; clean cargo deny and audit; DES causality strictly preserved.
