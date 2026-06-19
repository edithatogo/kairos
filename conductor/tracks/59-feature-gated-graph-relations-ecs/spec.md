# Track 59: Feature-Gated Graph Relations ECS Module

## Objective

Add a `graph-relations` feature-gated Graph-ECS module using Entity IDs as edge data while preserving flat ECS storage and cache locality.

## Scope

- Add relationship components such as `ChildOf(Entity)` and `TransitionTo(Entity)`.
- Provide traversal helpers over flat component arrays.
- Ensure the graph module is absent unless the feature is enabled.
- Forbid raw pointer, self-referential, and boxed graph topology.

## Done

Feature-boundary tests, traversal tests, no-pointer scans, and all relevant crate tests must pass. Handoff must record task commits, review, push, and GitHub Actions review.
