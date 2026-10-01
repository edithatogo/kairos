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


## Release implications

Track 58 may support public game-theory ontology API claims only after deterministic generation, generated compile gates, API review, local validation, push evidence, and GitHub Actions review are recorded. It does not by itself certify graph traversal, normal-form solvers, extensive-form solvers, or ontology publication; those claims remain owned by Tracks 57 and 59-61.

## Blocked paths

Track 58 remains blocked from Done until remaining generated-code API integration tasks complete, fallback-disabled feature gates are covered, generated public surfaces are reviewed, local gates pass, GitHub Actions pass from the final head, and handoff evidence records commit SHAs and any waivers.
