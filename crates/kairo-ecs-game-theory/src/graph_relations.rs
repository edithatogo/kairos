//! Feature-gated graph-relational ECS helpers.
//!
//! Graph edges are stored as ordinary components keyed by `EntityId`; the
//! module does not own graph nodes or link entities through memory pointers.
