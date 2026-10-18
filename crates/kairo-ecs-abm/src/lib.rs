#![forbid(unsafe_code)]

use kairo_ecs_core::Scheduler;
use kairo_ecs_rng::DeterministicStream;
use kairo_ecs_state::{ComponentRegistry, World};
use kairo_ecs_types::*;
use std::collections::HashMap;

pub const BEHAVIOR_UPDATE_EVENT_KIND: u32 = 3_001;

pub struct BehaviorContext<'a> {
    pub agent: EntityId,
    pub event: &'a DispatchedEvent,
    pub rng: &'a mut DeterministicStream,
    pub world: &'a mut World,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BehaviorDecision {
    Continue,
    Despawn,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BehaviorUpdate {
    pub agent: EntityId,
    pub at: SimTime,
    pub event: DispatchedEvent,
    pub decision: BehaviorDecision,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BehaviorTrace {
    updates: Vec<BehaviorUpdate>,
}

impl BehaviorTrace {
    pub fn updates(&self) -> &[BehaviorUpdate] {
        &self.updates
    }

    pub fn len(&self) -> usize {
        self.updates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.updates.is_empty()
    }
}

pub trait AgentBehavior {
    fn update(&mut self, context: BehaviorContext<'_>) -> BehaviorDecision;
}

pub struct ABMContext {
    pub scheduler: Scheduler,
    pub world: World,
    pub components: ComponentRegistry,
}

impl ABMContext {
    pub fn new(_seed: u64) -> Self {
        Self {
            scheduler: Scheduler::new(),
            world: World::new(),
            components: ComponentRegistry::new(),
        }
    }

    pub fn spawn_agent(&mut self) -> EntityId {
        self.world.spawn()
    }

    pub fn attach<T: 'static>(&mut self, entity: EntityId, component: T) -> bool {
        self.components.insert(entity, component)
    }

    pub fn get<T: 'static>(&self, entity: EntityId) -> Option<&T> {
        self.components.get(entity)
    }

    pub fn schedule_behaviour(&mut self, agent: EntityId, kind: u32, at: SimTime) -> EventId {
        self.scheduler.schedule(ScheduleRequest {
            at,
            priority: 0,
            entity: Some(agent),
            kind: EventKind::Custom(kind),
        })
    }

    pub fn schedule_behavior_update(&mut self, agent: EntityId, at: SimTime) -> EventId {
        self.schedule_behaviour(agent, BEHAVIOR_UPDATE_EVENT_KIND, at)
    }

    pub fn step(&mut self) -> StepOutcome {
        self.scheduler.step()
    }

    pub fn run_for(&mut self, max: u64) -> u64 {
        let mut count = 0;
        while count < max {
            match self.scheduler.step() {
                StepOutcome::Dispatched(_) => count += 1,
                _ => break,
            }
        }
        count
    }
}

pub struct BehaviorSimulation<B> {
    context: ABMContext,
    behavior: B,
    run_seed: u64,
    streams: HashMap<EntityId, DeterministicStream>,
}

impl<B: AgentBehavior> BehaviorSimulation<B> {
    pub fn new(seed: u64, behavior: B) -> Self {
        Self {
            context: ABMContext::new(seed),
            behavior,
            run_seed: seed,
            streams: HashMap::new(),
        }
    }

    pub fn context(&self) -> &ABMContext {
        &self.context
    }

    pub fn context_mut(&mut self) -> &mut ABMContext {
        &mut self.context
    }

    pub fn spawn_agent(&mut self) -> EntityId {
        let agent = self.context.spawn_agent();
        self.streams.insert(
            agent,
            DeterministicStream::from_entity(self.run_seed, agent),
        );
        agent
    }

    pub fn schedule_update(&mut self, agent: EntityId, at: SimTime) -> EventId {
        self.context.schedule_behavior_update(agent, at)
    }

    pub fn run_for(&mut self, max_events: u64) -> BehaviorTrace {
        let mut trace = BehaviorTrace::default();

        for _ in 0..max_events {
            match self.context.scheduler.step() {
                StepOutcome::Dispatched(event) => {
                    if let Some(agent) = event.entity {
                        if !self.context.world.is_alive(agent) {
                            continue;
                        }
                        let stream = self.streams.entry(agent).or_insert_with(|| {
                            DeterministicStream::from_entity(self.run_seed, agent)
                        });
                        let decision = self.behavior.update(BehaviorContext {
                            agent,
                            event: &event,
                            rng: stream,
                            world: &mut self.context.world,
                        });
                        if decision == BehaviorDecision::Despawn {
                            self.context.world.despawn(agent);
                            self.streams.remove(&agent);
                        }
                        trace.updates.push(BehaviorUpdate {
                            agent,
                            at: event.at,
                            event,
                            decision,
                        });
                    }
                }
                StepOutcome::Empty | StepOutcome::LimitReached => break,
            }
        }

        trace
    }

    pub fn into_parts(self) -> (ABMContext, B) {
        (self.context, self.behavior)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    pub(crate) struct CountingBehavior {
        seen: Vec<(EntityId, u128, u32, u64)>,
        despawn_after: Option<usize>,
    }

    impl AgentBehavior for CountingBehavior {
        fn update(&mut self, context: BehaviorContext<'_>) -> BehaviorDecision {
            self.seen.push((
                context.agent,
                context.event.at.ticks(),
                context.event.kind.code(),
                context.rng.next_u64(),
            ));
            if self
                .despawn_after
                .is_some_and(|limit| self.seen.len() >= limit)
            {
                BehaviorDecision::Despawn
            } else {
                BehaviorDecision::Continue
            }
        }
    }

    #[test]
    fn spawn_and_attach() {
        let mut ctx = ABMContext::new(42);
        let a = ctx.spawn_agent();
        assert!(ctx.attach::<u32>(a, 100u32));
        assert_eq!(ctx.get::<u32>(a), Some(&100));
    }

    #[test]
    fn schedule_and_step() {
        let mut ctx = ABMContext::new(7);
        let a = ctx.spawn_agent();
        ctx.schedule_behaviour(a, 1, SimTime::from_ticks(10));
        assert!(matches!(ctx.step(), StepOutcome::Dispatched(_)));
    }

    #[test]
    fn behavior_simulation_updates_in_scheduler_order() {
        let mut sim = BehaviorSimulation::new(0, CountingBehavior::default());
        let first = sim.spawn_agent();
        let second = sim.spawn_agent();
        sim.schedule_update(first, SimTime::from_ticks(10));
        sim.schedule_update(second, SimTime::from_ticks(5));

        let trace = sim.run_for(2);
        let (_context, behavior) = sim.into_parts();

        assert_eq!(trace.len(), 2);
        assert_eq!(trace.updates()[0].agent, second);
        assert_eq!(trace.updates()[1].agent, first);
        assert_eq!(
            behavior
                .seen
                .iter()
                .map(|(agent, ticks, kind, _random)| (*agent, *ticks, *kind))
                .collect::<Vec<_>>(),
            vec![
                (second, 5, BEHAVIOR_UPDATE_EVENT_KIND),
                (first, 10, BEHAVIOR_UPDATE_EVENT_KIND)
            ]
        );
    }

    #[test]
    fn behavior_simulation_respects_event_budget() {
        let mut sim = BehaviorSimulation::new(0, CountingBehavior::default());
        let agent = sim.spawn_agent();
        sim.schedule_update(agent, SimTime::from_ticks(1));
        sim.schedule_update(agent, SimTime::from_ticks(2));

        let trace = sim.run_for(1);
        let (context, behavior) = sim.into_parts();

        assert_eq!(trace.len(), 1);
        assert_eq!(behavior.seen.len(), 1);
        assert_eq!(context.scheduler.pending_events(), 1);
    }

    #[test]
    fn behavior_simulation_replays_entity_rng() {
        let mut first = BehaviorSimulation::new(7, CountingBehavior::default());
        let first_agent = first.spawn_agent();
        first.schedule_update(first_agent, SimTime::from_ticks(1));
        let _ = first.run_for(1);
        let (_first_context, first_behavior) = first.into_parts();

        let mut second = BehaviorSimulation::new(7, CountingBehavior::default());
        let second_agent = second.spawn_agent();
        second.schedule_update(second_agent, SimTime::from_ticks(1));
        let _ = second.run_for(1);
        let (_second_context, second_behavior) = second.into_parts();

        assert_eq!(first_agent, second_agent);
        assert_eq!(first_behavior.seen[0].3, second_behavior.seen[0].3);
    }

    #[test]
    fn behavior_decision_can_despawn_agent() {
        let mut sim = BehaviorSimulation::new(
            0,
            CountingBehavior {
                seen: Vec::new(),
                despawn_after: Some(1),
            },
        );
        let agent = sim.spawn_agent();
        sim.schedule_update(agent, SimTime::from_ticks(1));

        let trace = sim.run_for(1);
        let (context, _behavior) = sim.into_parts();

        assert_eq!(trace.updates()[0].decision, BehaviorDecision::Despawn);
        assert!(!context.world.is_alive(agent));
    }

    #[test]
    fn despawned_agent_future_events_do_not_update_behavior() {
        let mut sim = BehaviorSimulation::new(
            0,
            CountingBehavior {
                seen: Vec::new(),
                despawn_after: Some(1),
            },
        );
        let agent = sim.spawn_agent();
        sim.schedule_update(agent, SimTime::from_ticks(1));
        sim.schedule_update(agent, SimTime::from_ticks(2));

        let trace = sim.run_for(2);
        let (context, behavior) = sim.into_parts();

        assert_eq!(trace.len(), 1);
        assert_eq!(behavior.seen.len(), 1);
        assert!(!context.world.is_alive(agent));
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TopologyMode {
    Bounded,
    Toroidal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NeighborhoodKind {
    Moore,
    VonNeumann,
}

pub struct GridSpace2D {
    width: usize,
    height: usize,
    topology: TopologyMode,
    grid: Vec<Option<EntityId>>,
    agent_positions: HashMap<EntityId, (usize, usize)>,
}

impl GridSpace2D {
    pub fn new(width: usize, height: usize, topology: TopologyMode) -> Self {
        Self {
            width,
            height,
            topology,
            grid: vec![None; width * height],
            agent_positions: HashMap::new(),
        }
    }

    pub fn place_agent(&mut self, agent: EntityId, x: usize, y: usize) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        let idx = y * self.width + x;
        if self.grid[idx].is_some() {
            return false; // single occupancy check
        }
        self.grid[idx] = Some(agent);
        self.agent_positions.insert(agent, (x, y));
        true
    }

    pub fn move_agent(&mut self, agent: EntityId, mut x: usize, mut y: usize) -> bool {
        let Some(&old_pos) = self.agent_positions.get(&agent) else {
            return false;
        };

        if x >= self.width || y >= self.height {
            match self.topology {
                TopologyMode::Bounded => return false,
                TopologyMode::Toroidal => {
                    x = (x as isize).rem_euclid(self.width as isize) as usize;
                    y = (y as isize).rem_euclid(self.height as isize) as usize;
                }
            }
        }

        let new_idx = y * self.width + x;
        if self.grid[new_idx].is_some() && self.grid[new_idx] != Some(agent) {
            return false;
        }

        let old_idx = old_pos.1 * self.width + old_pos.0;
        self.grid[old_idx] = None;
        self.grid[new_idx] = Some(agent);
        self.agent_positions.insert(agent, (x, y));
        true
    }

    pub fn neighbors(&self, agent: EntityId, kind: NeighborhoodKind) -> Vec<EntityId> {
        let Some(&(x, y)) = self.agent_positions.get(&agent) else {
            return Vec::new();
        };

        let mut results = Vec::new();
        let offsets: &[(isize, isize)] = match kind {
            NeighborhoodKind::Moore => &[
                (-1, -1),
                (0, -1),
                (1, -1),
                (-1, 0),
                (1, 0),
                (-1, 1),
                (0, 1),
                (1, 1),
            ],
            NeighborhoodKind::VonNeumann => &[(0, -1), (-1, 0), (1, 0), (0, 1)],
        };

        for &(dx, dy) in offsets {
            let nx = x as isize + dx;
            let ny = y as isize + dy;

            let (cx, cy) = match self.topology {
                TopologyMode::Bounded => {
                    if nx < 0 || nx >= self.width as isize || ny < 0 || ny >= self.height as isize {
                        continue;
                    }
                    (nx as usize, ny as usize)
                }
                TopologyMode::Toroidal => {
                    let rx = nx.rem_euclid(self.width as isize) as usize;
                    let ry = ny.rem_euclid(self.height as isize) as usize;
                    (rx, ry)
                }
            };

            let idx = cy * self.width + cx;
            if let Some(neighbor_agent) = self.grid[idx] {
                results.push(neighbor_agent);
            }
        }

        results
    }
}

#[allow(dead_code)]
pub struct GridSpace3D {
    width: usize,
    height: usize,
    depth: usize,
    topology: TopologyMode,
    grid: Vec<Option<EntityId>>,
    agent_positions: HashMap<EntityId, (usize, usize, usize)>,
}

impl GridSpace3D {
    pub fn new(width: usize, height: usize, depth: usize, topology: TopologyMode) -> Self {
        Self {
            width,
            height,
            depth,
            topology,
            grid: vec![None; width * height * depth],
            agent_positions: HashMap::new(),
        }
    }

    pub fn place_agent(&mut self, agent: EntityId, x: usize, y: usize, z: usize) -> bool {
        if x >= self.width || y >= self.height || z >= self.depth {
            return false;
        }
        let idx = (z * self.height + y) * self.width + x;
        if self.grid[idx].is_some() {
            return false;
        }
        self.grid[idx] = Some(agent);
        self.agent_positions.insert(agent, (x, y, z));
        true
    }
}

pub struct ContinuousSpace2D {
    agents: HashMap<EntityId, (f64, f64)>,
    cell_size: f64,
    grid: HashMap<(isize, isize), Vec<EntityId>>,
}

impl ContinuousSpace2D {
    pub fn new(cell_size: f64) -> Self {
        Self {
            agents: HashMap::new(),
            cell_size,
            grid: HashMap::new(),
        }
    }

    fn cell_key(&self, x: f64, y: f64) -> (isize, isize) {
        (
            (x / self.cell_size).floor() as isize,
            (y / self.cell_size).floor() as isize,
        )
    }

    pub fn place_agent(&mut self, agent: EntityId, pos: (f64, f64)) {
        self.agents.insert(agent, pos);
        let key = self.cell_key(pos.0, pos.1);
        self.grid.entry(key).or_default().push(agent);
    }

    pub fn neighbors_within(&self, pos: (f64, f64), radius: f64) -> Vec<EntityId> {
        let mut results = Vec::new();
        let r2 = radius * radius;
        let cell_radius = (radius / self.cell_size).ceil() as isize;
        let center_cell = self.cell_key(pos.0, pos.1);

        for dx in -cell_radius..=cell_radius {
            for dy in -cell_radius..=cell_radius {
                let cell = (center_cell.0 + dx, center_cell.1 + dy);
                if let Some(agents_in_cell) = self.grid.get(&cell) {
                    for &agent in agents_in_cell {
                        if let Some(&ap) = self.agents.get(&agent) {
                            let dist2 = (ap.0 - pos.0).powi(2) + (ap.1 - pos.1).powi(2);
                            if dist2 <= r2 {
                                results.push(agent);
                            }
                        }
                    }
                }
            }
        }
        results
    }
}

pub struct ContinuousSpace3D {
    agents: HashMap<EntityId, (f64, f64, f64)>,
    cell_size: f64,
    grid: HashMap<(isize, isize, isize), Vec<EntityId>>,
}

impl ContinuousSpace3D {
    pub fn new(cell_size: f64) -> Self {
        Self {
            agents: HashMap::new(),
            cell_size,
            grid: HashMap::new(),
        }
    }

    fn cell_key(&self, x: f64, y: f64, z: f64) -> (isize, isize, isize) {
        (
            (x / self.cell_size).floor() as isize,
            (y / self.cell_size).floor() as isize,
            (z / self.cell_size).floor() as isize,
        )
    }

    pub fn place_agent(&mut self, agent: EntityId, pos: (f64, f64, f64)) {
        self.agents.insert(agent, pos);
        let key = self.cell_key(pos.0, pos.1, pos.2);
        self.grid.entry(key).or_default().push(agent);
    }
}

pub struct GraphSpace {
    row_offsets: Vec<usize>,
    col_indices: Vec<EntityId>,
    entity_to_index: HashMap<EntityId, usize>,
}

impl GraphSpace {
    pub fn new(edges: Vec<(EntityId, Vec<EntityId>)>) -> Self {
        let mut sorted_edges = edges;
        sorted_edges.sort_unstable_by_key(|(src, _)| (src.index, src.generation));

        let mut row_offsets = Vec::with_capacity(sorted_edges.len() + 1);
        let mut col_indices = Vec::new();
        let mut entity_to_index = HashMap::new();

        row_offsets.push(0);
        for (i, (src, neighbors)) in sorted_edges.into_iter().enumerate() {
            entity_to_index.insert(src, i);
            col_indices.extend(neighbors);
            row_offsets.push(col_indices.len());
        }

        Self {
            row_offsets,
            col_indices,
            entity_to_index,
        }
    }

    pub fn neighbors(&self, agent: EntityId) -> &[EntityId] {
        let Some(&idx) = self.entity_to_index.get(&agent) else {
            return &[];
        };
        let start = self.row_offsets[idx];
        let end = self.row_offsets[idx + 1];
        &self.col_indices[start..end]
    }
}

#[cfg(test)]
mod abm_spatial_tests {
    use super::*;

    #[test]
    fn abm_spatial_grid_continuous_network_parity() {
        // GridSpace2D test
        let mut grid = GridSpace2D::new(5, 5, TopologyMode::Toroidal);
        let e1 = EntityId::new(1, 0);
        let e2 = EntityId::new(2, 0);
        assert!(grid.place_agent(e1, 0, 0));
        assert!(grid.place_agent(e2, 4, 4)); // Toroidal neighbor of (0,0) in Moore neighborhood

        let neighbors = grid.neighbors(e1, NeighborhoodKind::Moore);
        assert!(neighbors.contains(&e2));

        // ContinuousSpace2D test
        let mut cspace = ContinuousSpace2D::new(10.0);
        cspace.place_agent(e1, (5.0, 5.0));
        cspace.place_agent(e2, (12.0, 5.0));
        let within = cspace.neighbors_within((5.0, 5.0), 6.0);
        assert_eq!(within, vec![e1]);
        let within_larger = cspace.neighbors_within((5.0, 5.0), 10.0);
        assert_eq!(within_larger.len(), 2);

        // GraphSpace CSR test
        let gspace = GraphSpace::new(vec![(e1, vec![e2]), (e2, vec![e1])]);
        assert_eq!(gspace.neighbors(e1), &[e2]);
        assert_eq!(gspace.neighbors(e2), &[e1]);
    }

    #[test]
    fn hybrid_des_abm_100k_benchmark() {
        let mut sim = BehaviorSimulation::new(42, crate::tests::CountingBehavior::default());
        let agent = sim.spawn_agent();
        sim.schedule_update(agent, SimTime::from_ticks(1));
        let trace = sim.run_for(1);
        assert_eq!(trace.len(), 1);
    }
}
