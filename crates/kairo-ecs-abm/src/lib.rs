#![forbid(unsafe_code)]

use kairo_ecs_core::Scheduler;
use kairo_ecs_rng::{derive_entity_seed, DeterministicStream};
use kairo_ecs_state::World;
use kairo_ecs_types::{
    DispatchedEvent, EntityId, EventKind, ScheduleRequest, SimTime, StepOutcome,
};

pub const BEHAVIOR_UPDATE_KIND: u32 = 30_300;

/// Inputs exposed to one deterministic ABM behavior update.
#[derive(Debug)]
pub struct BehaviorContext<'a> {
    pub entity: EntityId,
    pub event: &'a DispatchedEvent,
    pub rng: &'a mut DeterministicStream,
}

/// Result requested by an agent behavior after an update event.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BehaviorDecision {
    Continue,
    Despawn,
}

/// Minimal ABM behavior contract over the shared event kernel.
pub trait AgentBehavior {
    fn update(&mut self, context: BehaviorContext<'_>) -> BehaviorDecision;
}

#[derive(Debug)]
struct AgentSlot<B> {
    entity: EntityId,
    behavior: B,
    rng: DeterministicStream,
    alive: bool,
}

/// Deterministic ABM behavior runner using the shared scheduler and entity store.
#[derive(Debug)]
pub struct BehaviorSimulation<B> {
    scheduler: Scheduler,
    world: World,
    agents: Vec<AgentSlot<B>>,
    run_seed: u64,
}

impl<B> BehaviorSimulation<B> {
    pub fn new(run_seed: u64) -> Self {
        Self {
            scheduler: Scheduler::new(),
            world: World::new(),
            agents: Vec::new(),
            run_seed,
        }
    }

    pub fn spawn_agent(&mut self, behavior: B) -> EntityId {
        let entity = self.world.spawn();
        let rng = DeterministicStream::new(derive_entity_seed(self.run_seed, entity));
        self.agents.push(AgentSlot {
            entity,
            behavior,
            rng,
            alive: true,
        });
        entity
    }

    pub fn schedule_update(&mut self, entity: EntityId, at: SimTime, priority: i32) {
        self.scheduler.schedule(ScheduleRequest {
            at,
            priority,
            entity: Some(entity),
            kind: EventKind::Custom(BEHAVIOR_UPDATE_KIND),
        });
    }

    pub fn alive_agents(&self) -> usize {
        self.world.len()
    }
}

impl<B: AgentBehavior> BehaviorSimulation<B> {
    pub fn run_for(&mut self, max_events: u64) -> BehaviorTrace {
        let mut updates = Vec::new();

        for _ in 0..max_events {
            let event = match self.scheduler.step() {
                StepOutcome::Dispatched(event) => event,
                StepOutcome::Empty | StepOutcome::LimitReached => break,
            };

            if event.kind != EventKind::Custom(BEHAVIOR_UPDATE_KIND) {
                continue;
            }

            let Some(entity) = event.entity else {
                continue;
            };

            let Some(slot) = self
                .agents
                .iter_mut()
                .find(|slot| slot.entity == entity && slot.alive)
            else {
                continue;
            };

            let decision = slot.behavior.update(BehaviorContext {
                entity,
                event: &event,
                rng: &mut slot.rng,
            });

            if decision == BehaviorDecision::Despawn {
                slot.alive = false;
                self.world.despawn(entity);
            }

            updates.push(BehaviorUpdate {
                entity,
                at: event.at,
                decision,
            });
        }

        BehaviorTrace { updates }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BehaviorUpdate {
    pub entity: EntityId,
    pub at: SimTime,
    pub decision: BehaviorDecision,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BehaviorTrace {
    updates: Vec<BehaviorUpdate>,
}

impl BehaviorTrace {
    pub fn updates(&self) -> &[BehaviorUpdate] {
        &self.updates
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default)]
    struct CounterBehavior {
        seen: Vec<u64>,
        stop_after: usize,
    }

    impl AgentBehavior for CounterBehavior {
        fn update(&mut self, context: BehaviorContext<'_>) -> BehaviorDecision {
            self.seen.push(context.rng.next_u64());
            if self.seen.len() >= self.stop_after {
                BehaviorDecision::Despawn
            } else {
                BehaviorDecision::Continue
            }
        }
    }

    #[test]
    fn behavior_updates_follow_scheduler_order() {
        let mut sim = BehaviorSimulation::new(42);
        let first = sim.spawn_agent(CounterBehavior {
            seen: Vec::new(),
            stop_after: 2,
        });
        let second = sim.spawn_agent(CounterBehavior {
            seen: Vec::new(),
            stop_after: 2,
        });

        sim.schedule_update(first, SimTime::from_ticks(10), 1);
        sim.schedule_update(second, SimTime::from_ticks(5), 1);
        sim.schedule_update(first, SimTime::from_ticks(10), 0);

        let trace = sim.run_for(3);

        assert_eq!(trace.updates().len(), 3);
        assert_eq!(trace.updates()[0].entity, second);
        assert_eq!(trace.updates()[1].entity, first);
        assert_eq!(trace.updates()[2].entity, first);
        assert_eq!(sim.alive_agents(), 1);
    }

    #[test]
    fn behavior_rng_replays_from_run_seed_and_entity() {
        #[derive(Debug)]
        struct ExpectedFirstRandom {
            expected: u64,
        }

        impl AgentBehavior for ExpectedFirstRandom {
            fn update(&mut self, context: BehaviorContext<'_>) -> BehaviorDecision {
                if context.rng.next_u64() == self.expected {
                    BehaviorDecision::Despawn
                } else {
                    BehaviorDecision::Continue
                }
            }
        }

        let mut sim = BehaviorSimulation::new(7);
        let entity = sim.spawn_agent(ExpectedFirstRandom { expected: 0 });
        let mut replay = DeterministicStream::new(derive_entity_seed(7, entity));
        let expected = replay.next_u64();
        sim.agents[0].behavior.expected = expected;

        sim.schedule_update(entity, SimTime::from_ticks(1), 0);
        let trace = sim.run_for(1);

        assert_eq!(trace.updates()[0].decision, BehaviorDecision::Despawn);
        assert_eq!(sim.alive_agents(), 0);
    }
}
