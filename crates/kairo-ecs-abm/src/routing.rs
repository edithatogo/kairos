#![forbid(unsafe_code)]

use kairo_ecs_types::{EntityId, SimDuration};
use petgraph::algo::dijkstra;
use petgraph::graph::{EdgeIndex, EdgeReference, NodeIndex};
use petgraph::visit::{EdgeFiltered, EdgeRef};
use petgraph::Direction;
use std::collections::{BTreeMap, HashMap, HashSet};

/// Cost unit declared for a spatial topology and its edges.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CostUnit {
    Ticks,
    Meters,
    Seconds,
    Arbitrary,
}

/// Category of spatial node (location / waypoint).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SpatialNodeKind {
    Room,
    Doorway,
    Corridor,
    Stairwell,
    LiftLobby,
    Portal,
    WayPoint,
}

/// Category of spatial edge (traversal pathway).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SpatialEdgeKind {
    Walkway,
    Doorway,
    Stairs,
    Lift,
    ServiceShaft,
}

/// Access requirements for traversing a spatial edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EdgeAccessRule {
    pub min_clearance: u32,
    pub step_free: bool,
    pub is_open: bool,
}

impl Default for EdgeAccessRule {
    fn default() -> Self {
        Self {
            min_clearance: 0,
            step_free: true,
            is_open: true,
        }
    }
}

/// Node attributes stored in the spatial topology graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpatialNodeData {
    pub entity: EntityId,
    pub floor: i32,
    pub kind: SpatialNodeKind,
    pub label: Option<String>,
}

/// Edge attributes stored in the spatial topology graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpatialEdgeData {
    pub edge_id: u64,
    pub cost: u64,
    pub unit: CostUnit,
    pub kind: SpatialEdgeKind,
    pub access: EdgeAccessRule,
}

/// Profile representing the mobility and authorization of an agent querying routes.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct AccessProfile {
    pub clearance_level: u32,
    pub requires_step_free: bool,
}

impl AccessProfile {
    pub const fn standard() -> Self {
        Self {
            clearance_level: 0,
            requires_step_free: false,
        }
    }

    pub const fn step_free() -> Self {
        Self {
            clearance_level: 0,
            requires_step_free: true,
        }
    }

    pub const fn staff(clearance_level: u32) -> Self {
        Self {
            clearance_level,
            requires_step_free: false,
        }
    }

    pub const fn staff_step_free(clearance_level: u32) -> Self {
        Self {
            clearance_level,
            requires_step_free: true,
        }
    }

    pub fn can_traverse(&self, edge: &SpatialEdgeData) -> bool {
        if !edge.access.is_open {
            return false;
        }
        if self.clearance_level < edge.access.min_clearance {
            return false;
        }
        if self.requires_step_free && !edge.access.step_free {
            return false;
        }
        true
    }
}

/// Errors produced during topology manipulation or routing queries.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum RoutingError {
    #[error("node entity {0:?} not found in spatial topology")]
    NodeNotFound(EntityId),

    #[error("duplicate node entity {0:?} in spatial topology")]
    DuplicateNode(EntityId),

    #[error("duplicate edge ID {0} in spatial topology")]
    DuplicateEdgeId(u64),

    #[error("edge ID {0} not found in spatial topology")]
    EdgeNotFound(u64),

    #[error("cost unit mismatch: expected {expected:?}, got {actual:?}")]
    UnitMismatch {
        expected: CostUnit,
        actual: CostUnit,
    },

    #[error("integer overflow encountered during routing cost accumulation")]
    CostOverflow,

    #[error("no route found from {from:?} to {to:?}")]
    NoRouteFound { from: EntityId, to: EntityId },
}

/// Directed traversal segment within an extracted route.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteSegment {
    pub edge_id: u64,
    pub source: EntityId,
    pub target: EntityId,
    pub cost: u64,
    pub kind: SpatialEdgeKind,
}

/// Extracted deterministic shortest route between two spatial nodes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpatialRoute {
    pub path: Vec<EntityId>,
    pub segments: Vec<RouteSegment>,
    pub total_cost: u64,
    pub hop_count: usize,
    pub unit: CostUnit,
    pub topology_version: u64,
}

impl SpatialRoute {
    pub fn is_empty(&self) -> bool {
        self.path.is_empty()
    }

    pub fn duration(&self) -> Result<SimDuration, RoutingError> {
        if self.unit != CostUnit::Ticks {
            return Err(RoutingError::UnitMismatch {
                expected: CostUnit::Ticks,
                actual: self.unit,
            });
        }
        Ok(SimDuration::from_ticks(self.total_cost as u128))
    }

    pub fn contains_node(&self, entity: EntityId) -> bool {
        self.path.contains(&entity)
    }
}

/// Internal metric for Dijkstra with lexicographic tie-breaking by (cost, hops).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
struct DijkstraMetric {
    cost: u128,
    hops: u32,
}

impl std::ops::Add for DijkstraMetric {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            cost: self.cost.saturating_add(rhs.cost),
            hops: self.hops.saturating_add(rhs.hops),
        }
    }
}

/// Pinned petgraph-backed spatial topology and routing solver.
pub struct SpatialTopology {
    graph: petgraph::graph::DiGraph<SpatialNodeData, SpatialEdgeData>,
    entity_to_node: HashMap<EntityId, NodeIndex>,
    edge_id_to_edge: HashMap<u64, EdgeIndex>,
    unit: CostUnit,
    topology_version: u64,
    route_cache: HashMap<(EntityId, EntityId, AccessProfile), (u64, SpatialRoute)>,
}

impl SpatialTopology {
    /// Creates an empty spatial topology with declared cost unit.
    pub fn new(unit: CostUnit) -> Self {
        Self {
            graph: petgraph::graph::DiGraph::new(),
            entity_to_node: HashMap::new(),
            edge_id_to_edge: HashMap::new(),
            unit,
            topology_version: 1,
            route_cache: HashMap::new(),
        }
    }

    pub fn unit(&self) -> CostUnit {
        self.unit
    }

    pub fn version(&self) -> u64 {
        self.topology_version
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    pub fn contains_node(&self, entity: EntityId) -> bool {
        self.entity_to_node.contains_key(&entity)
    }

    pub fn node_floor(&self, entity: EntityId) -> Option<i32> {
        let idx = self.entity_to_node.get(&entity)?;
        Some(self.graph[*idx].floor)
    }

    /// Adds a spatial node to the topology.
    pub fn add_node(
        &mut self,
        entity: EntityId,
        floor: i32,
        kind: SpatialNodeKind,
    ) -> Result<(), RoutingError> {
        self.add_node_with_label(entity, floor, kind, None)
    }

    /// Adds a spatial node with an optional debug/display label.
    pub fn add_node_with_label(
        &mut self,
        entity: EntityId,
        floor: i32,
        kind: SpatialNodeKind,
        label: Option<String>,
    ) -> Result<(), RoutingError> {
        if self.entity_to_node.contains_key(&entity) {
            return Err(RoutingError::DuplicateNode(entity));
        }
        let data = SpatialNodeData {
            entity,
            floor,
            kind,
            label,
        };
        let idx = self.graph.add_node(data);
        self.entity_to_node.insert(entity, idx);
        self.bump_version();
        Ok(())
    }

    /// Removes a node and its incident edges.
    pub fn remove_node(&mut self, entity: EntityId) -> Result<(), RoutingError> {
        let Some(&idx) = self.entity_to_node.get(&entity) else {
            return Err(RoutingError::NodeNotFound(entity));
        };

        // Collect incident edge IDs to clean up lookup map
        let mut edges_to_remove = Vec::new();
        for edge_ref in self.graph.edges_directed(idx, Direction::Outgoing) {
            edges_to_remove.push(edge_ref.weight().edge_id);
        }
        for edge_ref in self.graph.edges_directed(idx, Direction::Incoming) {
            edges_to_remove.push(edge_ref.weight().edge_id);
        }

        self.graph.remove_node(idx);
        self.entity_to_node.remove(&entity);

        // Rebuild entity_to_node and edge_id_to_edge indices since petgraph NodeIndex swap-removes
        self.rebuild_indices();
        self.bump_version();
        Ok(())
    }

    /// Adds a directed edge between two spatial nodes with checked cost and access rule.
    pub fn add_edge(
        &mut self,
        edge_id: u64,
        from: EntityId,
        to: EntityId,
        cost: u64,
        kind: SpatialEdgeKind,
        access: EdgeAccessRule,
    ) -> Result<(), RoutingError> {
        if self.edge_id_to_edge.contains_key(&edge_id) {
            return Err(RoutingError::DuplicateEdgeId(edge_id));
        }
        let Some(&from_idx) = self.entity_to_node.get(&from) else {
            return Err(RoutingError::NodeNotFound(from));
        };
        let Some(&to_idx) = self.entity_to_node.get(&to) else {
            return Err(RoutingError::NodeNotFound(to));
        };

        let edge_data = SpatialEdgeData {
            edge_id,
            cost,
            unit: self.unit,
            kind,
            access,
        };
        let edge_idx = self.graph.add_edge(from_idx, to_idx, edge_data);
        self.edge_id_to_edge.insert(edge_id, edge_idx);
        self.bump_version();
        Ok(())
    }

    /// Removes a directed edge by ID.
    pub fn remove_edge(&mut self, edge_id: u64) -> Result<(), RoutingError> {
        let Some(&edge_idx) = self.edge_id_to_edge.get(&edge_id) else {
            return Err(RoutingError::EdgeNotFound(edge_id));
        };
        self.graph.remove_edge(edge_idx);
        self.rebuild_indices();
        self.bump_version();
        Ok(())
    }

    /// Updates the cost of an existing edge.
    pub fn set_edge_cost(&mut self, edge_id: u64, new_cost: u64) -> Result<(), RoutingError> {
        let Some(&edge_idx) = self.edge_id_to_edge.get(&edge_id) else {
            return Err(RoutingError::EdgeNotFound(edge_id));
        };
        self.graph[edge_idx].cost = new_cost;
        self.bump_version();
        Ok(())
    }

    /// Toggles door state (open/closed) on an edge.
    pub fn set_door_state(&mut self, edge_id: u64, is_open: bool) -> Result<(), RoutingError> {
        let Some(&edge_idx) = self.edge_id_to_edge.get(&edge_id) else {
            return Err(RoutingError::EdgeNotFound(edge_id));
        };
        self.graph[edge_idx].access.is_open = is_open;
        self.bump_version();
        Ok(())
    }

    /// Sets the access rule for an edge.
    pub fn set_edge_access(
        &mut self,
        edge_id: u64,
        access: EdgeAccessRule,
    ) -> Result<(), RoutingError> {
        let Some(&edge_idx) = self.edge_id_to_edge.get(&edge_id) else {
            return Err(RoutingError::EdgeNotFound(edge_id));
        };
        self.graph[edge_idx].access = access;
        self.bump_version();
        Ok(())
    }

    /// Invalidates all cached routes.
    pub fn invalidate_cache(&mut self) {
        self.bump_version();
        self.route_cache.clear();
    }

    /// Queries the shortest route between two nodes, consulting and updating the route cache.
    pub fn find_route(
        &mut self,
        from: EntityId,
        to: EntityId,
        profile: &AccessProfile,
    ) -> Result<SpatialRoute, RoutingError> {
        let cache_key = (from, to, profile.clone());
        if let Some((ver, route)) = self.route_cache.get(&cache_key) {
            if *ver == self.topology_version {
                return Ok(route.clone());
            }
        }

        let route = self.find_route_uncached(from, to, profile)?;
        self.route_cache
            .insert(cache_key, (self.topology_version, route.clone()));
        Ok(route)
    }

    /// Solves the shortest route without using or mutating the route cache.
    pub fn find_route_uncached(
        &self,
        from: EntityId,
        to: EntityId,
        profile: &AccessProfile,
    ) -> Result<SpatialRoute, RoutingError> {
        let Some(&start_idx) = self.entity_to_node.get(&from) else {
            return Err(RoutingError::NodeNotFound(from));
        };
        let Some(&goal_idx) = self.entity_to_node.get(&to) else {
            return Err(RoutingError::NodeNotFound(to));
        };

        if start_idx == goal_idx {
            return Ok(SpatialRoute {
                path: vec![from],
                segments: Vec::new(),
                total_cost: 0,
                hop_count: 0,
                unit: self.unit,
                topology_version: self.topology_version,
            });
        }

        // Filter edges using EdgeFiltered adapter
        let filtered_graph = EdgeFiltered(
            &self.graph,
            |edge_ref: EdgeReference<'_, SpatialEdgeData>| profile.can_traverse(edge_ref.weight()),
        );

        // Run petgraph Dijkstra with (cost, hops) metric
        let scores = dijkstra(&filtered_graph, start_idx, Some(goal_idx), |edge_ref| {
            DijkstraMetric {
                cost: edge_ref.weight().cost as u128,
                hops: 1,
            }
        });

        let Some(&goal_metric) = scores.get(&goal_idx) else {
            return Err(RoutingError::NoRouteFound { from, to });
        };

        if goal_metric.cost > u64::MAX as u128 {
            return Err(RoutingError::CostOverflow);
        }

        // Backward pass: identify all nodes and edges belonging to an optimal shortest path to goal
        let mut optimal_nodes = HashSet::new();
        optimal_nodes.insert(goal_idx);

        let mut queue = vec![goal_idx];
        let mut visited = HashSet::new();
        visited.insert(goal_idx);

        // Store optimal incoming edges per node
        let mut optimal_edges = HashSet::new();

        while let Some(curr) = queue.pop() {
            let curr_metric = scores[&curr];
            for edge_ref in self.graph.edges_directed(curr, Direction::Incoming) {
                let edge_data = edge_ref.weight();
                if !profile.can_traverse(edge_data) {
                    continue;
                }
                let pred = edge_ref.source();
                let Some(&pred_metric) = scores.get(&pred) else {
                    continue;
                };

                // Check exact metric optimality: pred_metric + edge == curr_metric
                let expected_cost = match pred_metric.cost.checked_add(edge_data.cost as u128) {
                    Some(c) => c,
                    None => continue,
                };
                if expected_cost == curr_metric.cost
                    && pred_metric.hops.saturating_add(1) == curr_metric.hops
                {
                    optimal_edges.insert(edge_ref.id());
                    optimal_nodes.insert(pred);
                    if visited.insert(pred) {
                        queue.push(pred);
                    }
                }
            }
        }

        // Forward extraction from start_idx to goal_idx:
        // At each step, among all optimal outgoing edges, choose the one with the smallest edge_id
        let mut current = start_idx;
        let mut path_entities = vec![from];
        let mut route_segments = Vec::with_capacity(goal_metric.hops as usize);
        let mut loop_guard = 0usize;
        let max_hops = goal_metric.hops as usize + 1;

        while current != goal_idx {
            loop_guard += 1;
            if loop_guard > max_hops {
                // Safeguard against impossible loop
                return Err(RoutingError::NoRouteFound { from, to });
            }

            // Gather all candidate outgoing edges that are in optimal_edges
            let mut candidates: Vec<_> = self
                .graph
                .edges_directed(current, Direction::Outgoing)
                .filter(|e| optimal_edges.contains(&e.id()))
                .collect();

            if candidates.is_empty() {
                return Err(RoutingError::NoRouteFound { from, to });
            }

            // Tie-break: sort candidates by edge_id ascending
            candidates.sort_by_key(|e| e.weight().edge_id);
            let chosen_edge = candidates[0];

            let next_node = chosen_edge.target();
            let chosen_edge_data = chosen_edge.weight();

            path_entities.push(self.graph[next_node].entity);
            route_segments.push(RouteSegment {
                edge_id: chosen_edge_data.edge_id,
                source: self.graph[current].entity,
                target: self.graph[next_node].entity,
                cost: chosen_edge_data.cost,
                kind: chosen_edge_data.kind,
            });

            current = next_node;
        }

        Ok(SpatialRoute {
            path: path_entities,
            segments: route_segments,
            total_cost: goal_metric.cost as u64,
            hop_count: goal_metric.hops as usize,
            unit: self.unit,
            topology_version: self.topology_version,
        })
    }

    /// Exports adjacency list representation compatible with GraphSpace.
    pub fn to_csr_edges(&self) -> Vec<(EntityId, Vec<EntityId>)> {
        let mut map: BTreeMap<(u64, u32), Vec<EntityId>> = BTreeMap::new();
        for &idx in self.entity_to_node.values() {
            let entity = self.graph[idx].entity;
            map.entry((entity.index, entity.generation)).or_default();
        }
        for edge in self.graph.raw_edges() {
            let src = self.graph[edge.source()].entity;
            let target = self.graph[edge.target()].entity;
            map.entry((src.index, src.generation))
                .or_default()
                .push(target);
        }
        map.into_iter()
            .map(|((idx, gen), targets)| (EntityId::new(idx, gen), targets))
            .collect()
    }

    /// Access reference to underlying petgraph DiGraph for advanced graph algorithms.
    pub fn petgraph_graph(&self) -> &petgraph::graph::DiGraph<SpatialNodeData, SpatialEdgeData> {
        &self.graph
    }

    fn bump_version(&mut self) {
        self.topology_version = self.topology_version.wrapping_add(1);
    }

    fn rebuild_indices(&mut self) {
        self.entity_to_node.clear();
        for idx in self.graph.node_indices() {
            self.entity_to_node.insert(self.graph[idx].entity, idx);
        }
        self.edge_id_to_edge.clear();
        for edge_idx in self.graph.edge_indices() {
            self.edge_id_to_edge
                .insert(self.graph[edge_idx].edge_id, edge_idx);
        }
    }
}
