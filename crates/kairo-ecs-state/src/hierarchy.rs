#![forbid(unsafe_code)]

use crate::World;
use kairo_ecs_core::{
    LifecyclePolicy, LinkedEntityError, ModelEntityId, RelationshipKind, RelationshipLifecycleEvent,
};
use kairo_ecs_types::{EntityId, SimTime};
use std::collections::HashMap;

/// Child edge descriptor stored on parent.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ChildLink {
    pub child: EntityId,
    pub kind: RelationshipKind,
    pub policy: LifecyclePolicy,
}

/// Parent edge descriptor stored on child.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ParentLink {
    pub parent: EntityId,
    pub kind: RelationshipKind,
    pub policy: LifecyclePolicy,
}

/// Native ECS hierarchy engine supporting 1-to-many typed links, stable model identities,
/// cycle prevention, and deterministic cascaded lifecycles.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EntityHierarchy {
    parents: HashMap<EntityId, ParentLink>,
    children: HashMap<EntityId, Vec<ChildLink>>,
    stable_to_entity: HashMap<ModelEntityId, EntityId>,
    entity_to_stable: HashMap<EntityId, ModelEntityId>,
    events: Vec<RelationshipLifecycleEvent>,
}

impl EntityHierarchy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            parents: HashMap::with_capacity(capacity),
            children: HashMap::with_capacity(capacity),
            stable_to_entity: HashMap::with_capacity(capacity),
            entity_to_stable: HashMap::with_capacity(capacity),
            events: Vec::with_capacity(capacity),
        }
    }

    /// Registers a stable external model identity for an entity.
    pub fn register_stable_id(
        &mut self,
        stable_id: ModelEntityId,
        entity: EntityId,
        world: &World,
    ) -> Result<(), LinkedEntityError> {
        if !world.is_alive(entity) {
            return Err(LinkedEntityError::StaleEntityHandle(entity));
        }
        if let Some(&existing_entity) = self.stable_to_entity.get(&stable_id) {
            if existing_entity != entity && world.is_alive(existing_entity) {
                return Err(LinkedEntityError::StableIdCollision(stable_id));
            }
        }
        self.stable_to_entity.insert(stable_id, entity);
        self.entity_to_stable.insert(entity, stable_id);
        Ok(())
    }

    /// Resolves a stable external model identity to a live `EntityId`.
    pub fn resolve_stable_id(&self, stable_id: ModelEntityId, world: &World) -> Option<EntityId> {
        self.stable_to_entity
            .get(&stable_id)
            .copied()
            .filter(|&e| world.is_alive(e))
    }

    /// Returns the stable model identity for an entity, if registered.
    pub fn get_stable_id(&self, entity: EntityId) -> Option<ModelEntityId> {
        self.entity_to_stable.get(&entity).copied()
    }

    /// Attaches `child` to `parent` with a typed relationship and lifecycle policy.
    pub fn attach_child(
        &mut self,
        parent: EntityId,
        child: EntityId,
        kind: RelationshipKind,
        policy: LifecyclePolicy,
        world: &World,
        at: SimTime,
    ) -> Result<(), LinkedEntityError> {
        if !world.is_alive(parent) {
            return Err(LinkedEntityError::StaleEntityHandle(parent));
        }
        if !world.is_alive(child) {
            return Err(LinkedEntityError::StaleEntityHandle(child));
        }
        if parent == child {
            return Err(LinkedEntityError::CycleDetected { parent, child });
        }

        // Cycle check: verify child is not already an ancestor of parent
        if self.is_ancestor_of(child, parent) {
            return Err(LinkedEntityError::CycleDetected { parent, child });
        }

        // Check for duplicate relationship
        if let Some(existing_parent) = self.parents.get(&child) {
            if existing_parent.parent == parent && existing_parent.kind == kind {
                return Err(LinkedEntityError::DuplicateRelationship { parent, child });
            }
            // Child already has a different parent; detach first
            self.detach_child_internal(existing_parent.parent, child);
        }

        let child_link = ChildLink {
            child,
            kind,
            policy,
        };
        let parent_link = ParentLink {
            parent,
            kind,
            policy,
        };

        self.children.entry(parent).or_default().push(child_link);
        self.parents.insert(child, parent_link);

        self.events.push(RelationshipLifecycleEvent::Attached {
            parent,
            child,
            kind,
            policy,
            at,
        });

        Ok(())
    }

    /// Detaches a child from its parent.
    pub fn detach_child(
        &mut self,
        parent: EntityId,
        child: EntityId,
        at: SimTime,
    ) -> Result<(), LinkedEntityError> {
        if let Some(p) = self.parents.get(&child) {
            if p.parent != parent {
                return Err(LinkedEntityError::ParentNotFound(parent));
            }
        } else {
            return Err(LinkedEntityError::ChildNotFound(child));
        }

        self.detach_child_internal(parent, child);

        self.events
            .push(RelationshipLifecycleEvent::Detached { parent, child, at });
        Ok(())
    }

    fn detach_child_internal(&mut self, parent: EntityId, child: EntityId) {
        self.parents.remove(&child);
        if let Some(list) = self.children.get_mut(&parent) {
            list.retain(|link| link.child != child);
            if list.is_empty() {
                self.children.remove(&parent);
            }
        }
    }

    /// Reparents `child` under `new_parent`.
    pub fn reparent(
        &mut self,
        child: EntityId,
        new_parent: EntityId,
        world: &World,
        at: SimTime,
    ) -> Result<(), LinkedEntityError> {
        if !world.is_alive(child) {
            return Err(LinkedEntityError::StaleEntityHandle(child));
        }
        if !world.is_alive(new_parent) {
            return Err(LinkedEntityError::StaleEntityHandle(new_parent));
        }
        if child == new_parent || self.is_ancestor_of(child, new_parent) {
            return Err(LinkedEntityError::CycleDetected {
                parent: new_parent,
                child,
            });
        }

        let (kind, policy) = if let Some(old_link) = self.parents.get(&child).copied() {
            let old_parent = old_link.parent;
            self.detach_child_internal(old_parent, child);
            self.events.push(RelationshipLifecycleEvent::Reparented {
                child,
                old_parent,
                new_parent,
                at,
            });
            (old_link.kind, old_link.policy)
        } else {
            (RelationshipKind::ParentChild, LifecyclePolicy::Survive)
        };

        let child_link = ChildLink {
            child,
            kind,
            policy,
        };
        let parent_link = ParentLink {
            parent: new_parent,
            kind,
            policy,
        };

        self.children
            .entry(new_parent)
            .or_default()
            .push(child_link);
        self.parents.insert(child, parent_link);

        Ok(())
    }

    /// Returns direct parent of child.
    pub fn parent_of(&self, child: EntityId) -> Option<EntityId> {
        self.parents.get(&child).map(|p| p.parent)
    }

    /// Returns direct parent link (including kind and policy).
    pub fn parent_link_of(&self, child: EntityId) -> Option<ParentLink> {
        self.parents.get(&child).copied()
    }

    /// Returns direct child entities for parent in insertion order.
    pub fn children_of(&self, parent: EntityId) -> Vec<EntityId> {
        self.children
            .get(&parent)
            .map(|links| links.iter().map(|l| l.child).collect())
            .unwrap_or_default()
    }

    /// Returns direct child links for parent.
    pub fn child_links_of(&self, parent: EntityId) -> &[ChildLink] {
        self.children
            .get(&parent)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Returns direct child entities of a specific kind.
    pub fn typed_children_of(&self, parent: EntityId, kind: RelationshipKind) -> Vec<EntityId> {
        self.children
            .get(&parent)
            .map(|links| {
                links
                    .iter()
                    .filter(|l| l.kind == kind)
                    .map(|l| l.child)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Returns true if `ancestor` is an ancestor of `descendant`.
    pub fn is_ancestor_of(&self, ancestor: EntityId, descendant: EntityId) -> bool {
        let mut curr = descendant;
        while let Some(parent) = self.parent_of(curr) {
            if parent == ancestor {
                return true;
            }
            curr = parent;
        }
        false
    }

    /// Returns all ancestors of `entity` in bottom-up order (immediate parent first).
    pub fn ancestors_of(&self, entity: EntityId) -> Vec<EntityId> {
        let mut ancestors = Vec::new();
        let mut curr = entity;
        while let Some(parent) = self.parent_of(curr) {
            ancestors.push(parent);
            curr = parent;
        }
        ancestors
    }

    /// Walks descendants of `root` in deterministic depth-first order.
    pub fn descendants_of(&self, root: EntityId) -> Vec<EntityId> {
        let mut descendants = Vec::new();
        let mut stack = self.children_of(root);
        stack.reverse();

        while let Some(entity) = stack.pop() {
            descendants.push(entity);
            let mut children = self.children_of(entity);
            children.reverse();
            stack.extend(children);
        }

        descendants
    }

    /// Deterministically cascades despawn of `root` and any descendants configured with `LifecyclePolicy::CascadeDelete`.
    /// Descendants configured with `LifecyclePolicy::Survive` or `LifecyclePolicy::Detach` survive
    /// and are detached into independent roots.
    ///
    /// Returns the list of all despawned entities (starting with `root`).
    pub fn cascade_despawn(
        &mut self,
        root: EntityId,
        world: &mut World,
        at: SimTime,
    ) -> Result<Vec<EntityId>, LinkedEntityError> {
        if !world.is_alive(root) {
            return Err(LinkedEntityError::StaleEntityHandle(root));
        }

        let mut to_despawn = Vec::new();
        let mut queue = vec![root];

        while let Some(current) = queue.pop() {
            to_despawn.push(current);

            if let Some(links) = self.children.remove(&current) {
                for link in links {
                    self.parents.remove(&link.child);
                    match link.policy {
                        LifecyclePolicy::CascadeDelete => {
                            if world.is_alive(link.child) {
                                queue.push(link.child);
                            }
                        }
                        LifecyclePolicy::Survive | LifecyclePolicy::Detach => {
                            // Child survives: parent link removed, remains alive in world
                            self.events.push(RelationshipLifecycleEvent::Detached {
                                parent: current,
                                child: link.child,
                                at,
                            });
                        }
                    }
                }
            }
        }

        // If root had a parent, detach it from its parent
        if let Some(parent_link) = self.parents.remove(&root) {
            if let Some(siblings) = self.children.get_mut(&parent_link.parent) {
                siblings.retain(|l| l.child != root);
                if siblings.is_empty() {
                    self.children.remove(&parent_link.parent);
                }
            }
        }

        // Despawn all marked entities from World and clean up maps
        let despawn_count = to_despawn.len();
        for &entity in &to_despawn {
            world.despawn(entity);
            self.parents.remove(&entity);
            self.children.remove(&entity);
            if let Some(stable_id) = self.entity_to_stable.remove(&entity) {
                self.stable_to_entity.remove(&stable_id);
            }
        }

        self.events
            .push(RelationshipLifecycleEvent::CascadedDespawn {
                entity: root,
                cascade_count: despawn_count,
                at,
            });

        Ok(to_despawn)
    }

    /// Drains recorded relationship lifecycle events for telemetry export.
    pub fn drain_events(&mut self) -> Vec<RelationshipLifecycleEvent> {
        std::mem::take(&mut self.events)
    }

    /// Access recorded events.
    pub fn events(&self) -> &[RelationshipLifecycleEvent] {
        &self.events
    }
}
