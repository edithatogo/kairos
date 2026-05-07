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
    struct CountingBehavior {
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
}
