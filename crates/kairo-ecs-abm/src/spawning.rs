#![forbid(unsafe_code)]

use kairo_ecs_core::{
    LifecyclePolicy, LinkedEntityError, ModelEntityId, RelationshipKind, SpawnRelationship,
};
use kairo_ecs_rng::DeterministicStream;
use kairo_ecs_state::{EntityHierarchy, World};
use kairo_ecs_types::{EntityId, SimTime};
use std::collections::VecDeque;

/// Request to spawn a child entity dynamically during simulation execution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DynamicSpawnRequest {
    pub parent: EntityId,
    pub kind: RelationshipKind,
    pub policy: LifecyclePolicy,
    pub child_model_id: Option<ModelEntityId>,
    pub at: SimTime,
}

impl DynamicSpawnRequest {
    pub const fn new(
        parent: EntityId,
        kind: RelationshipKind,
        policy: LifecyclePolicy,
        child_model_id: Option<ModelEntityId>,
        at: SimTime,
    ) -> Self {
        Self {
            parent,
            kind,
            policy,
            child_model_id,
            at,
        }
    }
}

/// Bounded buffer for deferred spawning during inner loops and query iteration,
/// preventing iterator invalidation and ensuring memory bounds.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpawnBuffer {
    capacity: usize,
    queue: VecDeque<DynamicSpawnRequest>,
}

impl SpawnBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            queue: VecDeque::with_capacity(capacity),
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Queues a spawn request. Returns `Err(LinkedEntityError::CapacityExceeded)` if full.
    pub fn queue_spawn(&mut self, request: DynamicSpawnRequest) -> Result<(), LinkedEntityError> {
        if self.queue.len() >= self.capacity {
            return Err(LinkedEntityError::CapacityExceeded {
                max_capacity: self.capacity,
                requested: self.queue.len() + 1,
            });
        }
        self.queue.push_back(request);
        Ok(())
    }

    /// Drains all buffered requests for application.
    pub fn drain(&mut self) -> impl Iterator<Item = DynamicSpawnRequest> + '_ {
        self.queue.drain(..)
    }
}

/// Deterministic spawner managing dynamic runtime entity instantiation with bounded capacity,
/// deterministic handle generation, child RNG stream derivation, and hierarchy integration.
pub struct DynamicSpawner {
    budget: usize,
    spawned_count: usize,
    base_seed: u64,
}

impl DynamicSpawner {
    pub fn new(base_seed: u64, budget: usize) -> Self {
        Self {
            budget,
            spawned_count: 0,
            base_seed,
        }
    }

    pub fn budget(&self) -> usize {
        self.budget
    }

    pub fn spawned_count(&self) -> usize {
        self.spawned_count
    }

    pub fn remaining_capacity(&self) -> usize {
        self.budget.saturating_sub(self.spawned_count)
    }

    /// Spawns a child entity linked to `parent`.
    /// Rejects stale parent handles and enforces capacity budget.
    pub fn spawn_child(
        &mut self,
        world: &mut World,
        hierarchy: &mut EntityHierarchy,
        request: DynamicSpawnRequest,
    ) -> Result<(SpawnRelationship, DeterministicStream), LinkedEntityError> {
        if !world.is_alive(request.parent) {
            return Err(LinkedEntityError::StaleEntityHandle(request.parent));
        }

        if self.spawned_count >= self.budget {
            return Err(LinkedEntityError::CapacityExceeded {
                max_capacity: self.budget,
                requested: self.spawned_count + 1,
            });
        }

        // Spawn child entity in World
        let child = world.spawn();
        self.spawned_count += 1;

        // Register stable ID if provided
        if let Some(model_id) = request.child_model_id {
            hierarchy.register_stable_id(model_id, child, world)?;
        }

        // Attach relationship in Hierarchy
        hierarchy.attach_child(
            request.parent,
            child,
            request.kind,
            request.policy,
            world,
            request.at,
        )?;

        let parent_model_id = hierarchy.get_stable_id(request.parent);

        // Derive deterministic RNG stream for the child
        let child_seed = self
            .base_seed
            .wrapping_add(request.parent.index.wrapping_mul(1_000_003))
            .wrapping_add(self.spawned_count as u64);
        let child_stream = DeterministicStream::new(child_seed);

        let relationship = SpawnRelationship {
            parent: request.parent,
            child,
            parent_model_id,
            child_model_id: request.child_model_id,
            kind: request.kind,
            policy: request.policy,
            spawned_at: request.at,
        };

        Ok((relationship, child_stream))
    }

    /// Flushes all requests in `SpawnBuffer` into the world and hierarchy deterministically.
    pub fn flush_buffer(
        &mut self,
        buffer: &mut SpawnBuffer,
        world: &mut World,
        hierarchy: &mut EntityHierarchy,
    ) -> Result<Vec<(SpawnRelationship, DeterministicStream)>, LinkedEntityError> {
        let mut results = Vec::with_capacity(buffer.len());
        for req in buffer.drain() {
            let res = self.spawn_child(world, hierarchy, req)?;
            results.push(res);
        }
        Ok(results)
    }
}
