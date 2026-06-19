#[cfg(feature = "generated-components")]
pub mod generated_components {
    include!("../../../open-game-theory-ontology/fixtures/generated/rust/game_components.rs");
}

#[cfg(feature = "generated-components")]
pub use generated_components::*;
