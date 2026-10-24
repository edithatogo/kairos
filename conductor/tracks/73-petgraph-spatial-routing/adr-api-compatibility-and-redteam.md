# ADR: Track 73 — Spatial Routing Public API Compatibility Assessment & Red-Team Review

**Track:** 73-petgraph-spatial-routing  
**Date:** 2026-10-25  
**Author:** Principal Software Engineer / ABM API & Contracts Agent  
**Status:** Approved & Implemented  

---

## 1. Context & Scope

Track 73 introduces `SpatialTopology`, `SpatialRoute`, and related routing types in `crates/kairo-ecs-abm` backed by pinned `petgraph = "0.7.1"`. Pursuant to `AGENTS.md` and `CONTRIBUTING.md`, introducing or modifying public Rust APIs, schema contracts, and algorithms requires formal architectural documentation, compatibility assessment, and red-team objection analysis.

---

## 2. Public API Stability & Types Contract

### 2.1 Public Types Introduced

The following types are declared in `crates/kairo-ecs-abm::routing` and re-exported at the crate root:
- `SpatialTopology`: Graph builder and Dijkstra shortest-path solver with internal caching and automatic topology-version invalidation.
- `SpatialRoute`: Extracted path containing sequence of `EntityId` nodes, directed `RouteSegment` edges, total checked cost, hop count, declared `CostUnit`, and `topology_version`.
- `RouteSegment`: Directed traversal edge containing `edge_id`, `source`, `target`, integer `cost`, and `SpatialEdgeKind`.
- `AccessProfile`: Query constraints defining `clearance_level: u32` and `requires_step_free: bool`.
- `EdgeAccessRule`: Edge traversal requirements defining `min_clearance: u32`, `step_free: bool`, and `is_open: bool`.
- `SpatialNodeKind`: Node classification (`Room`, `Doorway`, `Corridor`, `Stairwell`, `LiftLobby`, `Portal`, `WayPoint`).
- `SpatialEdgeKind`: Traversal kind (`Walkway`, `Doorway`, `Stairs`, `Lift`, `ServiceShaft`).
- `CostUnit`: Declared unit for topology costs (`Ticks`, `Meters`, `Seconds`, `Arbitrary`).
- `RoutingError`: Typed enumeration deriving `thiserror::Error` for missing nodes, duplicate IDs, unit mismatches, arithmetic cost overflow, and unreachable destinations.

### 2.2 Backward Compatibility with `GraphSpace`
- The existing `GraphSpace` type in `crates/kairo-ecs-abm/src/lib.rs` is fully retained with its unchanged CSR slice lookup API (`neighbors(&self, agent: EntityId) -> &[EntityId]`).
- A bidirectional interoperability constructor `GraphSpace::from_spatial_topology(topology: &SpatialTopology) -> Self` is provided. Existing agent diffusion, infection, and network benchmarks are 100% backward-compatible and unaffected.

### 2.3 Unit Safety & Simulation Boundary
- Simulation time in KairoECS is strictly measured in discrete `SimDuration` (ticks).
- Calling `route.duration()` on a `SpatialRoute` validates that `route.unit == CostUnit::Ticks`. Topologies declared in `Meters`, `Seconds`, or `Arbitrary` return `Err(RoutingError::UnitMismatch)` rather than silently reinterpreting non-temporal distance or arbitrary weights as simulation ticks.
- Route calculation is purely spatial and schedules zero DES events. Agents schedule discrete movement events explicitly via `SimDuration`, preserving DES queue arbitration in `kairo-ecs-des`.

---

## 3. Conformance & Determinism Assessment

| Requirement | Enforcement Mechanism | Verification |
|---|---|---|
| Replay Determinism | 3-level lexicographic tie-breaking: minimum cost, minimum hop count, ascending `edge_id`. | `test_cost_hops_edge_id_tie_breaking` |
| Permutation Invariance | Shuffled node and edge insertion orders produce bit-identical paths. | `test_shuffled_insertion_order_invariance` across 10 random permutations |
| Zero-Cost Cycle Resilience | Multi-hop progress requirement strictly prevents looping in zero-cost edge cycles. | `test_zero_cost_cycle_resilience` |
| Exact Cost Overflow Distinction | Accumulates costs in `u128` to distinguish valid `u64::MAX` paths from arithmetic overflow. | `test_cost_overflow_and_exact_u64_max_handling` |
| DES Scheduler Isolation | Route computation takes `&self` and never enqueues events in `Scheduler`. | `test_des_boundary_route_rendering_does_not_schedule_events` |

---

## 4. Red-Team Review & Attack Vector Mitigations

| Attack / Failure Mode | Risk | Mitigation / Architectural Decision |
|:---|:---|:---|
| **Zero-Cost Cycle DoS** | Malicious or cyclical graph configurations (e.g. revolving doors or bidirectional walkways with 0 cost) cause Dijkstra or path extraction to hang in an infinite loop. | Dijkstra tracks `(cost: u128, hops: u32)`. Traversing any cycle increases hops, strictly worsening the metric score and preventing cycling. Backward extraction strictly enforces descending hops ($h, h-1, \dots, 0$), guaranteeing termination in exactly $h$ steps. |
| **Compiler / Platform HashMap Non-Determinism** | Standard `HashMap` iteration order is randomized by SipHash, causing different workers or platforms to select different equal-cost routes. | Forward path extraction collects all candidate optimal edges and sorts them strictly by ascending `edge_id` before selection. Shuffled graph permutations yield 100% bit-identical routes. |
| **Arithmetic Overflow via Saturating Addition** | Conflating `u64::MAX` path costs with genuine arithmetic overflow prevents legitimate maximal graphs from routing. | Edge costs are accumulated losslessly in `u128`. `CostOverflow` is returned only when path cost genuinely exceeds `u64::MAX as u128`. Exact `u64::MAX` routes are valid and supported. |
| **Unit Confusion in DES Movement** | Physical distance (e.g. 100 meters) silently treated as 100 simulation ticks, corrupting clinical simulation timelines. | `route.duration()` strictly enforces `route.unit == CostUnit::Ticks`, rejecting other units with `RoutingError::UnitMismatch`. |
| **Stale Route Caching after Door Lock** | Agent uses cached route through a door that was locked or an out-of-service lift. | `SpatialTopology` maintains a monotonic `topology_version`. Any mutation (`add_edge`, `set_door_state`, `set_edge_cost`) increments the version, instantly invalidating cached queries. |
