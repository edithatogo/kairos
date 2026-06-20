//! Feature-gated graph-relational ECS helpers.
//!
//! Graph edges are stored as ordinary components keyed by `EntityId`; the
//! module does not own graph nodes or link entities through memory pointers.

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
