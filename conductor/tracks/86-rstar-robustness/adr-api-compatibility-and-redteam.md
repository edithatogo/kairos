# ADR, Compatibility Assessment & Red-Team Review: rstar Continuous Spatial Index Qualification

## Track: 86-rstar-robustness

## Status
Accepted

## Context
Agent-based models (ABM) and spatial discrete-event simulations in KairoECS require efficient spatial indexing for continuous 2D coordinate spaces ($O(\log N)$ updates, nearest-neighbor lookups, and radial distance queries). Previously, `ContinuousSpace2D` relied exclusively on uniform spatial grid hashing with coarse bucketing, which suffered from:
1. No removal or update lifecycle support (moving an agent leaked stale references in previous cells).
2. Arbitrary non-deterministic hash iteration ordering in neighbor queries.
3. Degraded performance on non-uniform, sparse, or highly clustered agent distributions where uniform grid cells become skewed.

Track 86 qualifies and adopts `rstar = { version = "0.12.2", default-features = false }` to introduce `ContinuousSpatialIndex2D` while upgrading `ContinuousSpace2D` with deterministic sorting, clean agent removals, and seamless bidirectional conversion.

## Architecture Decision
1. **R-Tree Backed Continuous Spatial Index (`ContinuousSpatialIndex2D`)**:
   - Encapsulates `rstar::RTree<SpatialEntity2D>` alongside an internal `HashMap<EntityId, [f64; 2]>` for $O(1)$ position lookups and clean updates.
   - `insert(&mut self, entity: EntityId, x: f64, y: f64)`: Validates finite coordinates. If the entity was already indexed at an old position, it atomically removes the previous point from the R-tree, guaranteeing zero duplicate ghost nodes.
   - `remove(&mut self, entity: EntityId) -> Option<(f64, f64)>`: Removes from both R-tree and entity tracking table.
   - `position(&self, entity: EntityId) -> Option<(f64, f64)>`: Queries current position.

2. **Deterministic Query Oracles with Multi-Level Tie-Breaking**:
   - `locate_within_distance(x, y, radius)`: Finds all entities within Euclidean distance. Returns results deterministically sorted by:
     1. Ascending Euclidean distance.
     2. `EntityId` (numeric index and generation).
   - `k_nearest_neighbors(x, y, k)`: Finds the $k$ nearest entities. Employs distance-cutoff buffering to ensure that equidistant boundary points are deterministically tie-broken by `EntityId`, invariant to R-tree internal insertion order.
   - `nearest_neighbor(x, y)`: Single nearest neighbor delegate via deterministic $k=1$ nearest search.
   - `locate_in_envelope(min_x, min_y, max_x, max_y)`: Axis-aligned bounding box search, sorted deterministically by `EntityId`.

3. **Input Validation**:
   - Rejects non-finite coordinates (NaN, $\pm\infty$) with `SpatialIndexError::NonFiniteCoordinate`.
   - Rejects non-finite and negative query radii with `SpatialIndexError::NonFiniteRadius` and `SpatialIndexError::NegativeRadius`.
   - Rejects inverted bounding box bounds (`min_x > max_x` or `min_y > max_y`) with `SpatialIndexError::InvalidBoundingBox`.

4. **Integration with ContinuousSpace2D & GeoCoordinate**:
   - Upgraded `ContinuousSpace2D` in `crates/kairo-ecs-abm`:
     - Added `remove_agent(&mut self, agent: EntityId) -> Option<(f64, f64)>`.
     - `place_agent` now cleans previous cells upon agent relocation.
     - `neighbors_within` sorts results by `EntityId` for replay determinism.
     - `to_spatial_index(&self) -> Result<ContinuousSpatialIndex2D, SpatialIndexError>` provides zero-friction migration.
   - Integrates with Track 87 `LocalTangentPlaneProjection`, enabling direct spatial indexing and radius queries over projected WGS84 geographic coordinates.

## Compatibility Assessment
- **Breaking Changes**: None. All existing APIs in `kairo_ecs_abm` remain intact.
- **Dependency Footprint**: `rstar 0.12.2` was already pulled transitively by `geo 0.33.1`; declaring it with `default-features = false` adds 0 new transitive crates or binary overhead.
- **Safety**: Pure Rust with `#![forbid(unsafe_code)]`. Clean `cargo deny` and `cargo audit`.

## Red-Team / Devil's Advocate Review
- *Risk 1: Could R-tree tree rebalancing cause non-deterministic query results across different runs?*
  - **Mitigation**: Verified via `test_shuffled_insertion_order_invariance`. Regardless of insertion order or tree shape, query results are sorted by distance and EntityId, ensuring bit-for-bit replay determinism.
- *Risk 2: Could repeated movement in dynamic agent simulations cause memory leakage or ghost duplicates?*
  - **Mitigation**: `insert` checks the entity map and removes the exact previous position from the R-tree before inserting the new coordinate. Tested in `test_movement_and_removal_lifecycle`.
- *Risk 3: Could equidistant points cause k-nearest query truncation variance?*
  - **Mitigation**: `k_nearest_neighbors` collects all equidistant candidates at the $k$-th boundary distance before applying `(distance, EntityId)` sorting and truncation.
