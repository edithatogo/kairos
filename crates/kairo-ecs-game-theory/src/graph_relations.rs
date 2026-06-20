//! Feature-gated graph-relational ECS helpers.
//!
//! Graph edges are stored as ordinary components keyed by `EntityId`; the
//! module does not own graph nodes or link entities through memory pointers.

use kairo_ecs_state::ComponentStore;
use kairo_ecs_types::EntityId;

/// Relationship component whose owning entity is a child of the target entity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ChildOf(pub EntityId);

impl ChildOf {
    pub const fn target(&self) -> EntityId {
        self.0
    }
}

/// Relationship component whose owning entity transitions to the target entity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TransitionTo(pub EntityId);

impl TransitionTo {
    pub const fn target(&self) -> EntityId {
        self.0
    }
}

/// Returns direct child entities for `parent` in dense-store iteration order.
pub fn children_of(
    parent: EntityId,
    child_of: &ComponentStore<ChildOf>,
) -> impl Iterator<Item = EntityId> + '_ {
    child_of
        .iter()
        .filter_map(move |(entity, edge)| (edge.target() == parent).then_some(entity))
}

/// Returns the transition target attached to `entity`, when present.
pub fn transition_target(
    entity: EntityId,
    transitions: &ComponentStore<TransitionTo>,
) -> Option<EntityId> {
    transitions.get(entity).map(TransitionTo::target)
}

/// Walks descendants through `ChildOf` components without recursive ownership.
pub fn depth_first_descendants(
    root: EntityId,
    child_of: &ComponentStore<ChildOf>,
) -> Vec<EntityId> {
    let mut descendants = Vec::new();
    let mut stack = children_of(root, child_of).collect::<Vec<_>>();
    stack.reverse();

    while let Some(entity) = stack.pop() {
        descendants.push(entity);
        let mut children = children_of(entity, child_of).collect::<Vec<_>>();
        children.reverse();
        stack.extend(children);
    }

    descendants
}
