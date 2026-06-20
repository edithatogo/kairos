//! Game-theory runtime components and optional graph-relational ECS helpers.
//!
//! The graph-relational ECS surface is intentionally absent unless the
//! `graph-relations` feature is enabled.
//!
//! ```compile_fail
//! use kairo_ecs_game_theory::graph_relations::ChildOf;
//! ```

#[cfg(feature = "generated-components")]
pub mod generated_components {
    include!("../../../open-game-theory-ontology/fixtures/generated/rust/game_components.rs");
}

#[cfg(feature = "generated-components")]
pub use generated_components::*;
