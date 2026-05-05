#![forbid(unsafe_code)]

use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};

use kairo_ecs_types::EntityId;

/// Deterministic snapshot of one live entity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EntitySnapshot {
    pub id: EntityId,
}

/// Deterministic world snapshot for downstream telemetry and visualization.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WorldSnapshot {
    entities: Vec<EntitySnapshot>,
}

impl WorldSnapshot {
    pub fn entities(&self) -> &[EntitySnapshot] {
        &self.entities
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }
}

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

    pub fn snapshot(&self) -> WorldSnapshot {
        let mut entities: Vec<EntitySnapshot> = self
            .alive
            .iter()
            .copied()
            .map(|id| EntitySnapshot { id })
            .collect();
        entities.sort_by_key(|entity| (entity.id.index, entity.id.generation));

        WorldSnapshot { entities }
    }
}

/// A sparse set storing components of a single type.
/// Dense array: contiguous storage of components for alive entities.
/// Sparse array: maps EntityId index -> position in dense array.
pub struct ComponentStore<T> {
    dense: Vec<T>,
    sparse: Vec<Option<usize>>,
    entities: Vec<EntityId>,
}

impl<T> ComponentStore<T> {
    pub fn new() -> Self {
        Self {
            dense: Vec::new(),
            sparse: Vec::new(),
            entities: Vec::new(),
        }
    }

    pub fn insert(&mut self, entity: EntityId, component: T) {
        let idx = entity.index as usize;
        if idx >= self.sparse.len() {
            self.sparse.resize(idx + 1, None);
        }

        if let Some(pos) = self.sparse[idx] {
            if self.entities[pos] == entity {
                self.dense[pos] = component;
                return;
            }

            let _ = self.remove_at(pos);
        }

        self.sparse[idx] = Some(self.dense.len());
        self.dense.push(component);
        self.entities.push(entity);
    }

    pub fn remove(&mut self, entity: EntityId) -> Option<T> {
        let idx = entity.index as usize;
        if idx >= self.sparse.len() {
            return None;
        }
        let pos = self.sparse[idx]?;
        if self.entities[pos] != entity {
            return None;
        }
        Some(self.remove_at(pos))
    }

    fn remove_at(&mut self, pos: usize) -> T {
        let removed_entity_idx = self.entities[pos].index as usize;
        self.sparse[removed_entity_idx] = None;

        let last = self.dense.len() - 1;
        self.dense.swap(pos, last);
        self.entities.swap(pos, last);
        let removed = self.dense.pop().unwrap();
        self.entities.pop();
        if pos < self.entities.len() {
            let swapped_entity_idx = self.entities[pos].index as usize;
            self.sparse[swapped_entity_idx] = Some(pos);
        }
        removed
    }

    pub fn get(&self, entity: EntityId) -> Option<&T> {
        let idx = entity.index as usize;
        let pos = *self.sparse.get(idx)?.as_ref()?;
        if self.entities.get(pos).copied()? != entity {
            return None;
        }
        self.dense.get(pos)
    }

    pub fn get_mut(&mut self, entity: EntityId) -> Option<&mut T> {
        let idx = entity.index as usize;
        let pos = *self.sparse.get(idx)?.as_ref()?;
        if self.entities.get(pos).copied()? != entity {
            return None;
        }
        self.dense.get_mut(pos)
    }

    pub fn contains(&self, entity: EntityId) -> bool {
        self.get(entity).is_some()
    }

    pub fn len(&self) -> usize {
        self.dense.len()
    }

    pub fn is_empty(&self) -> bool {
        self.dense.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (EntityId, &T)> {
        self.entities.iter().copied().zip(self.dense.iter())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (EntityId, &mut T)> {
        self.entities.iter().copied().zip(self.dense.iter_mut())
    }
}

impl<T> Default for ComponentStore<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Type-erased component storage registry
pub struct ComponentRegistry {
    stores: HashMap<TypeId, Box<dyn Any>>,
}

impl ComponentRegistry {
    pub fn new() -> Self {
        Self {
            stores: HashMap::new(),
        }
    }

    pub fn register<T: 'static>(&mut self) {
        self.stores
            .insert(TypeId::of::<T>(), Box::new(ComponentStore::<T>::new()));
    }

    pub fn store<T: 'static>(&self) -> Option<&ComponentStore<T>> {
        self.stores
            .get(&TypeId::of::<T>())
            .and_then(|b| b.downcast_ref::<ComponentStore<T>>())
    }

    pub fn store_mut<T: 'static>(&mut self) -> Option<&mut ComponentStore<T>> {
        self.stores
            .get_mut(&TypeId::of::<T>())
            .and_then(|b| b.downcast_mut::<ComponentStore<T>>())
    }

    pub fn insert<T: 'static>(&mut self, entity: EntityId, component: T) {
        if self.store::<T>().is_none() {
            self.register::<T>();
        }
        self.store_mut::<T>().unwrap().insert(entity, component);
    }

    pub fn remove<T: 'static>(&mut self, entity: EntityId) -> Option<T> {
        self.store_mut::<T>()?.remove(entity)
    }

    pub fn get<T: 'static>(&self, entity: EntityId) -> Option<&T> {
        self.store::<T>()?.get(entity)
    }
}

impl Default for ComponentRegistry {
    fn default() -> Self {
        Self::new()
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

    #[test]
    fn snapshot_is_deterministically_ordered() {
        let mut world = World::new();
        let first = world.spawn();
        let second = world.spawn();
        let third = world.spawn();

        assert!(world.despawn(second));

        let snapshot = world.snapshot();

        assert_eq!(
            snapshot.entities(),
            &[EntitySnapshot { id: first }, EntitySnapshot { id: third },]
        );
        assert_eq!(snapshot.len(), 2);
        assert!(!snapshot.is_empty());
    }

    #[test]
    fn component_store_insert_get() {
        let mut store = ComponentStore::<u32>::new();
        let entity = EntityId {
            index: 0,
            generation: 0,
        };
        store.insert(entity, 42u32);
        assert_eq!(store.get(entity), Some(&42));
    }

    #[test]
    fn component_store_remove() {
        let mut store = ComponentStore::<String>::new();
        let e1 = EntityId {
            index: 0,
            generation: 0,
        };
        let e2 = EntityId {
            index: 1,
            generation: 0,
        };
        store.insert(e1, "hello".to_string());
        store.insert(e2, "world".to_string());
        assert_eq!(store.len(), 2);
        assert_eq!(store.remove(e1), Some("hello".to_string()));
        assert_eq!(store.len(), 1);
        assert!(store.get(e1).is_none());
        assert_eq!(store.get(e2), Some(&"world".to_string()));
    }

    #[test]
    fn component_store_replaces_same_entity_without_duplicate_row() {
        let mut store = ComponentStore::<u32>::new();
        let entity = EntityId {
            index: 0,
            generation: 0,
        };

        store.insert(entity, 1);
        store.insert(entity, 2);

        assert_eq!(store.len(), 1);
        assert_eq!(store.get(entity), Some(&2));
        assert_eq!(store.iter().collect::<Vec<_>>(), vec![(entity, &2)]);
    }

    #[test]
    fn component_store_rejects_stale_generation_for_same_index() {
        let mut store = ComponentStore::<u32>::new();
        let stale = EntityId {
            index: 7,
            generation: 0,
        };
        let current = EntityId {
            index: 7,
            generation: 1,
        };

        store.insert(current, 42);

        assert_eq!(store.get(stale), None);
        assert_eq!(store.get(current), Some(&42));
        assert_eq!(store.remove(stale), None);
        assert_eq!(store.remove(current), Some(42));
        assert!(store.is_empty());
    }

    #[test]
    fn component_store_new_generation_supersedes_old_index() {
        let mut store = ComponentStore::<u32>::new();
        let stale = EntityId {
            index: 2,
            generation: 0,
        };
        let current = EntityId {
            index: 2,
            generation: 1,
        };

        store.insert(stale, 10);
        store.insert(current, 20);

        assert_eq!(store.len(), 1);
        assert_eq!(store.get(stale), None);
        assert_eq!(store.get(current), Some(&20));
    }

    #[test]
    fn component_registry_multi_type() {
        let mut reg = ComponentRegistry::new();
        let e = EntityId {
            index: 0,
            generation: 0,
        };
        reg.insert(e, 42i32);
        reg.insert(e, "text".to_string());
        assert_eq!(reg.get::<i32>(e), Some(&42));
        assert_eq!(reg.get::<String>(e), Some(&"text".to_string()));
    }
}
