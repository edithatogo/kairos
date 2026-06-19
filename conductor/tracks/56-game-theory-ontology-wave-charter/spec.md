# Track 56: Game Theory Ontology Wave Charter and Evidence Gates

## Objective

Define the governance, evidence, and lifecycle gates for the open game theory ontology and multi-game runtime wave. This track does not implement runtime code; it prevents Tracks 57-61 from making public ontology, Graph-ECS, or solver claims without checked-in evidence.

## Scope

- Establish the accepted external parity targets for ontology-backed game theory execution.
- Define a machine-readable evidence manifest for ontology schemas, generated Rust APIs, feature gates, solver parity, benchmark fixtures, review outcomes, pushed refs, and GitHub Actions checks.
- Require task-level commits using `track NN task X.Y: <short outcome>`.
- Require `$conductor-review`, accepted in-scope fixes, handoff updates, push, and GitHub Actions review at every phase boundary.
- Define release wording boundaries for ontology, Graph-ECS, normal-form solver, and extensive-form solver claims.

## Non-goals

- No ontology parser implementation.
- No generated Rust component implementation.
- No graph-relations or solver runtime implementation.

## Done

Track 56 can move to `In Review` only when the charter, evidence manifest schema, lifecycle validator, and claim-boundary checks are implemented and locally validated. It can move to `Done` only after pushed CI passes and the handoff records commit SHAs, commands, evidence paths, and any waivers.
