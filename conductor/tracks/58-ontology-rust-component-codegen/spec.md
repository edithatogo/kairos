# Track 58: Ontology-to-Rust Component Code Generation

## Objective

Generate deterministic Rust component definitions from the canonical ontology IR produced by Track 57.

## Scope

- Map ontology classes and properties to Rust structs/enums for game-theory components.
- Generate stable APIs for `PayoffMatrix`, `StrategySpace`, `Utility`, player/action identifiers, information sets, and game nodes.
- Keep generated output deterministic and reviewable.
- Record generated API review evidence before solver tracks depend on it.

## Done

Regeneration must be deterministic, generated code must compile, API review evidence must be recorded, and handoff must include task commits, phase review/push, and GitHub Actions review.
