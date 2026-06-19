# Track 60: Normal-Form Multi-Game Runtime and Solvers

## Objective

Build the flat ECS runtime layer for normal-form games using ontology-derived components and solver systems that operate over contiguous component arrays.

## Scope

- Implement `PayoffMatrix`, `StrategySpace`, `Utility`, player, action, and game-profile components.
- Add validation for matrix shape, player/action cardinality, utility domains, and deterministic ordering.
- Implement solver systems for dominated-strategy elimination, best response, pure Nash equilibrium detection, and mixed-strategy extension points.
- Keep solver systems compatible with the existing Kairos ECS scheduling model.

## Done

Normal-form solver parity fixtures must pass, malformed game definitions must fail with typed errors, and benchmarks must record flat-array traversal costs. Handoff must include task commits, review, pushed ref, and GitHub Actions review.
