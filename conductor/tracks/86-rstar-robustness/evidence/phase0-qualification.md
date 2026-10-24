# Phase 0 Qualification: rstar Continuous Spatial Index Qualification

## Track: 86-rstar-robustness
## Date: 2026-10-25

## 1. Candidate Qualification
- **Crate**: `rstar`
- **Candidate Version**: `0.12.2`
- **Features Selected**: `default-features = false`
- **License**: `MIT OR Apache-2.0` (fully compliant with workspace license policy)
- **MSRV**: 1.65+ (compatible with workspace Rust 1.99.1)
- **Dependencies Introduced**: Pure Rust packages (`heapless 0.8.0`, `num-traits 0.2.19`, `smallvec 1.16.3`). Already present in `Cargo.lock` transitively from `geo 0.33.1`. Zero net new crates introduced.

## 2. Supply Chain & Security Verification
- `cargo deny check advisories`: 0 security advisories detected.
- `cargo deny check sources`: all crates resolved from crates.io.
- `cargo deny check licenses`: Apache-2.0 / MIT confirmed across all transitive dependencies.
- `cargo deny check bans`: 0 banned crates or duplicate version violations.
- `cargo audit`: 0 vulnerabilities.

## 3. Scope of Replacement & Contract
- Introduces `ContinuousSpatialIndex2D` in `crates/kairo-ecs-abm::spatial_index`.
- Provides $O(\log N)$ spatial indexing, radial queries, axis-aligned envelope queries, and $k$-nearest neighbors search.
- Solves ghost duplicate leaks in dynamic agent movement via atomic old-node deletion in R-tree.
- Enforces strict replay determinism via `(distance, EntityId)` sorting and equidistant boundary candidate collection.
- Upgrades existing `ContinuousSpace2D` with `remove_agent`, movement cleanup, deterministic sorting, and `to_spatial_index()`.

## 4. Test Oracles Defined
Executed in `crates/kairo-ecs-abm/tests/rstar_spatial_index_oracles.rs`:
- `test_brute_force_neighbour_oracle`
- `test_k_nearest_neighbors_oracle`
- `test_movement_and_removal_lifecycle`
- `test_duplicate_id_idempotence`
- `test_nonfinite_and_boundary_rejection`
- `test_bounding_box_envelope_oracle`
- `test_sparse_dense_skewed_workloads`
- `test_shuffled_insertion_order_invariance`
- `test_continuous_space_2d_interoperability`
- `test_geocoordinate_tangent_plane_spatial_indexing`
