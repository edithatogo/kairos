//! Game-theory runtime components and optional graph-relational ECS helpers.
//!
//! The graph-relational ECS surface is intentionally absent unless the
//! `graph-relations` feature is enabled.
//!
//! Build without default features to keep graph-relational types out of the
//! baseline crate surface, or enable `graph-relations` for `ChildOf`,
//! `TransitionTo`, and extensive-form helpers.

#[cfg(feature = "graph-relations")]
pub mod extensive_form;

#[cfg(feature = "graph-relations")]
pub mod graph_relations;

/// Normal-form game representations and retained equilibrium helpers.
///
/// Correlated equilibrium uses the optional Paignia solver. The legacy Kairos
/// product shortcut, welfare solver, result and incentive verifier are retired.
///
/// ```compile_fail
/// use kairo_ecs_game_theory::normal_form::CorrelatedEquilibriumSolver;
/// ```
///
/// ```compile_fail
/// use kairo_ecs_game_theory::normal_form::CorrelatedEquilibriumResult;
/// ```
///
/// ```compile_fail
/// use kairo_ecs_game_theory::normal_form::ClarabelConvexSolver;
/// let _ = ClarabelConvexSolver::solve_correlated_equilibrium_social_welfare(&[], &[]);
/// ```
///
/// ```compile_fail
/// use kairo_ecs_game_theory::normal_form::ClarabelConvexSolver;
/// let _ = ClarabelConvexSolver::verify_correlated_equilibrium_incentives(&[], &[], &[]);
/// ```
///
/// Paignia owns bounded mixed-strategy Nash solving and candidate admission.
///
/// ```compile_fail
/// use kairo_ecs_game_theory::normal_form::MixedStrategyProfile;
/// ```
///
/// ```compile_fail
/// use kairo_ecs_game_theory::normal_form::MixedNashSolver;
/// ```
///
/// ```compile_fail
/// use kairo_ecs_game_theory::normal_form::LemkeHowsonSolver;
/// ```
///
/// ```compile_fail
/// use kairo_ecs_game_theory::normal_form::nash_conv;
/// ```
///
/// ```compile_fail
/// use kairo_ecs_game_theory::normal_form::exploitability;
/// ```
pub mod normal_form;

#[cfg(feature = "generated-components")]
pub mod generated_components {
    include!("../../../open-game-theory-ontology/fixtures/generated/rust/game_components.rs");
}

#[cfg(feature = "generated-components")]
pub use generated_components::*;
