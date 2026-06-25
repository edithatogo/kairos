#![forbid(unsafe_code)]

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::num::NonZeroUsize;

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

fn entity_index(entity: EntityId) -> Option<usize> {
    usize::try_from(entity.index).ok()
}

fn encode_dense_position(position: usize) -> NonZeroUsize {
    NonZeroUsize::new(position + 1).expect("dense positions are one-based")
}

fn decode_dense_position(position: NonZeroUsize) -> usize {
    position.get() - 1
}

#[derive(Clone, Debug, Default)]
struct EntitySlot {
    generation: u32,
    alive: bool,
}

/// Generational entity store with reusable indices and deterministic snapshots.
///
/// The chosen Track 01 storage shape is a minimal sparse-set-style allocator:
/// - entity indices are recycled through a free list,
/// - generations advance on despawn so stale handles are rejected,
/// - live entities are kept in a dense vector for cheap iteration and stable
///   snapshot construction.
#[derive(Debug, Default)]
pub struct World {
    slots: Vec<EntitySlot>,
    free_indices: Vec<u64>,
    live_entities: Vec<EntityId>,
    live_positions: Vec<Option<NonZeroUsize>>,
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(entity_capacity: usize) -> Self {
        Self {
            slots: Vec::with_capacity(entity_capacity),
            free_indices: Vec::with_capacity(entity_capacity),
            live_entities: Vec::with_capacity(entity_capacity),
            live_positions: Vec::with_capacity(entity_capacity),
        }
    }

    pub fn reserve(&mut self, additional_entities: usize) {
        self.slots.reserve(additional_entities);
        self.free_indices.reserve(additional_entities);
        self.live_entities.reserve(additional_entities);
        self.live_positions.reserve(additional_entities);
    }

    pub fn spawn(&mut self) -> EntityId {
        let index = if let Some(index) = self.free_indices.pop() {
            index as usize
        } else {
            self.slots.push(EntitySlot::default());
            self.slots.len() - 1
        };

        let slot = &mut self.slots[index];
        debug_assert!(!slot.alive);

        let entity = EntityId {
            index: index as u64,
            generation: slot.generation,
        };

        slot.alive = true;

        if index >= self.live_positions.len() {
            self.live_positions.resize(index + 1, None);
        }

        self.live_positions[index] = Some(encode_dense_position(self.live_entities.len()));
        self.live_entities.push(entity);

        entity
    }

    pub fn despawn(&mut self, id: EntityId) -> bool {
        let Some(index) = entity_index(id) else {
            return false;
        };

        let Some(slot) = self.slots.get_mut(index) else {
            return false;
        };
        if !slot.alive || slot.generation != id.generation {
            return false;
        }

        let Some(position) = self
            .live_positions
            .get_mut(index)
            .and_then(|slot| slot.take())
            .map(decode_dense_position)
        else {
            return false;
        };

        let removed = self.live_entities.swap_remove(position);
        debug_assert_eq!(removed, id);

        if let Some(swapped_entity) = self.live_entities.get(position).copied() {
            self.live_positions[swapped_entity.index as usize] =
                Some(encode_dense_position(position));
        }

        slot.alive = false;
        slot.generation = slot.generation.wrapping_add(1);
        self.free_indices.push(id.index);

        true
    }

    pub fn is_alive(&self, id: EntityId) -> bool {
        let Some(index) = entity_index(id) else {
            return false;
        };
        self.slots
            .get(index)
            .is_some_and(|slot| slot.alive && slot.generation == id.generation)
    }

    pub fn len(&self) -> usize {
        self.live_entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.live_entities.is_empty()
    }

    pub fn snapshot(&self) -> WorldSnapshot {
        let mut entities = self
            .live_entities
            .iter()
            .copied()
            .map(|id| EntitySnapshot { id })
            .collect::<Vec<_>>();
        entities.sort_unstable_by_key(|entity| (entity.id.index, entity.id.generation));

        WorldSnapshot { entities }
    }
}

/// A sparse set storing components of a single type.
/// Dense array: contiguous storage of components for alive entities.
/// Sparse array: maps EntityId index -> position in dense array.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SparseEntry {
    generation: u32,
    position: NonZeroUsize,
}

pub struct ComponentStore<T> {
    dense: Vec<T>,
    sparse: Vec<Option<SparseEntry>>,
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

    pub fn with_capacity(entity_capacity: usize) -> Self {
        Self {
            dense: Vec::with_capacity(entity_capacity),
            sparse: Vec::with_capacity(entity_capacity),
            entities: Vec::with_capacity(entity_capacity),
        }
    }

    pub fn reserve(&mut self, additional_entities: usize) {
        self.dense.reserve(additional_entities);
        self.sparse.reserve(additional_entities);
        self.entities.reserve(additional_entities);
    }

    fn sparse_position(&self, entity: EntityId) -> Option<usize> {
        let index = entity_index(entity)?;
        let entry = self.sparse.get(index).copied().flatten()?;
        if entry.generation != entity.generation {
            return None;
        }
        Some(decode_dense_position(entry.position))
    }

    fn sparse_entry_for_index(&self, index: usize) -> Option<SparseEntry> {
        self.sparse.get(index).copied().flatten()
    }

    /// Inserts or updates the component for a live entity.
    ///
    /// Returns `false` when `entity` is stale for the same index, which keeps
    /// older generations from overwriting the current row.
    #[must_use]
    pub fn insert(&mut self, entity: EntityId, component: T) -> bool {
        let idx = entity_index(entity).expect("entity index exceeds platform usize");

        if idx >= self.sparse.len() {
            self.sparse.resize(idx + 1, None);
        }

        if let Some(entry) = self.sparse_entry_for_index(idx) {
            if entity.generation < entry.generation {
                return false;
            }

            let pos = decode_dense_position(entry.position);
            debug_assert_eq!(self.entities[pos].index, entity.index);
            self.entities[pos] = entity;
            self.dense[pos] = component;
            self.sparse[idx] = Some(SparseEntry {
                generation: entity.generation,
                position: entry.position,
            });
            return true;
        }

        let dense_position = self.dense.len();
        self.sparse[idx] = Some(SparseEntry {
            generation: entity.generation,
            position: encode_dense_position(dense_position),
        });
        self.dense.push(component);
        self.entities.push(entity);
        true
    }

    pub fn remove(&mut self, entity: EntityId) -> Option<T> {
        let pos = self.sparse_position(entity)?;
        if self.entities[pos] != entity {
            return None;
        }
        Some(self.remove_at(pos))
    }

    fn remove_at(&mut self, pos: usize) -> T {
        let removed_entity = self.entities.swap_remove(pos);
        let removed = self.dense.swap_remove(pos);

        self.sparse[removed_entity.index as usize] = None;

        if pos < self.entities.len() {
            let swapped_entity_idx = self.entities[pos].index as usize;
            self.sparse[swapped_entity_idx] = Some(SparseEntry {
                generation: self.entities[pos].generation,
                position: encode_dense_position(pos),
            });
        }

        removed
    }

    pub fn get(&self, entity: EntityId) -> Option<&T> {
        let pos = self.sparse_position(entity)?;
        if self.entities.get(pos).copied()? != entity {
            return None;
        }
        self.dense.get(pos)
    }

    pub fn get_mut(&mut self, entity: EntityId) -> Option<&mut T> {
        let pos = self.sparse_position(entity)?;
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

#[cfg(feature = "numa")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ComponentLocalityPlan {
    pub numa_node_hint: Option<u32>,
    pub rows: usize,
    pub dense_bytes: usize,
    pub cache_local: bool,
}

#[cfg(feature = "numa")]
impl ComponentLocalityPlan {
    pub fn for_store<T>(store: &ComponentStore<T>, numa_node_hint: u32) -> Self {
        Self {
            numa_node_hint: Some(numa_node_hint),
            rows: store.len(),
            dense_bytes: store.len() * std::mem::size_of::<T>(),
            cache_local: true,
        }
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
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(ComponentStore::<T>::new()));
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

    #[must_use]
    pub fn insert<T: 'static>(&mut self, entity: EntityId, component: T) -> bool {
        if self.store::<T>().is_none() {
            self.register::<T>();
        }
        self.store_mut::<T>().unwrap().insert(entity, component)
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
mod numa_tests {
    use super::*;

    #[cfg(feature = "numa")]
    #[test]
    fn component_locality_plan_preserves_dense_order_and_node_hint() {
        let mut store = ComponentStore::with_capacity(4);
        let first = EntityId::new(0, 0);
        let second = EntityId::new(1, 0);
        assert!(store.insert(first, 10_i32));
        assert!(store.insert(second, 20_i32));

        let plan = ComponentLocalityPlan::for_store(&store, 2);

        assert_eq!(plan.numa_node_hint, Some(2));
        assert_eq!(plan.rows, 2);
        assert_eq!(plan.dense_bytes, 2 * std::mem::size_of::<i32>());
        assert!(plan.cache_local);
    }
}
