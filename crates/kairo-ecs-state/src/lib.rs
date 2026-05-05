#![forbid(unsafe_code)]

use std::collections::HashSet;

use kairo_ecs_types::EntityId;

/// Minimal entity store used until the Track 01 ECS storage ADR lands.
#[derive(Debug, Default)]
pub struct World {
    next_index: u64,
    next_generation: u32,
    alive: HashSet<EntityId>,
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn spawn(&mut self) -> EntityId {
        let id = EntityId {
            index: self.next_index,
            generation: self.next_generation,
        };
        self.next_index += 1;
        self.next_generation = self.next_generation.wrapping_add(1);
        self.alive.insert(id);
        id
    }

    pub fn despawn(&mut self, id: EntityId) -> bool {
        self.alive.remove(&id)
    }

    pub fn is_alive(&self, id: EntityId) -> bool {
        self.alive.contains(&id)
    }

    pub fn len(&self) -> usize {
        self.alive.len()
    }

    pub fn is_empty(&self) -> bool {
        self.alive.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_lifecycle_is_explicit() {
        let mut world = World::new();
        let entity = world.spawn();
        let next = world.spawn();

        assert!(world.is_alive(entity));
        assert_ne!(entity, next);
        assert_eq!(world.len(), 2);
        assert_eq!(next.generation, entity.generation.wrapping_add(1));

        assert!(world.despawn(entity));
        assert!(!world.is_alive(entity));
        assert_eq!(world.len(), 1);
        assert!(!world.is_empty());
    }
}
