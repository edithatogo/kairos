# Track 60: Normal-Form Multi-Game Runtime and Solvers

## Objective

Build the flat ECS runtime layer for normal-form games using ontology-derived components and solver systems that operate over contiguous component arrays.

## Scope

- Implement `PayoffMatrix`, `StrategySpace`, `Utility`, player, action, and game-profile components.
- Add validation for matrix shape, player/action cardinality, utility domains, and deterministic ordering.
- Implement solver systems for dominated-strategy elimination, best response, pure Nash equilibrium detection, and mixed-strategy extension points.
- Keep solver systems compatible with the existing Kairos ECS scheduling model.

## Release implications

- Public release notes may describe only the component-model and validation surface until solver fixtures and benchmark evidence land in later phases.
- Normal-form execution must remain clearly experimental until best-response, pure Nash, dominated-strategy elimination, benchmark, documentation, phase review, push, and GitHub Actions evidence are all attached.
- This track does not claim extensive-form, Graph-ECS traversal, or full multi-game framework parity; those release claims remain blocked on Track 61.

## Blocked paths

- Do not expose a solver-complete public claim from Phase 0 component work.
- Do not accept malformed payoff shapes, non-finite utilities, empty player/action sets, duplicate strategy names, or nondeterministic profile ordering.
- Do not move Track 60 to Done until all solver phases, benchmark evidence, documentation, local gates, pushed GitHub Actions review, and handoff evidence are complete.

## Done

Normal-form solver parity fixtures must pass, malformed game definitions must fail with typed errors, and benchmarks must record flat-array traversal costs. Handoff must include task commits, review, pushed ref, and GitHub Actions review.
