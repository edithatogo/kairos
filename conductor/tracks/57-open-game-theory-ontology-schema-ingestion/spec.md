# Track 57: Open Game Theory Ontology Subrepo and Schema Ingestion

## Objective

Create the standalone `open-game-theory-ontology/` subrepo surface and an ingestion crate that parses Turtle and JSON-LD game-theory schemas into a canonical intermediate representation.

## Scope

- Initialize `open-game-theory-ontology/` with README, schema layout, fixtures, and provenance policy.
- Add `crates/kairo-ecs-game-ontology/` for parser and IR code.
- Support Turtle and JSON-LD fixture ingestion without lossy string-only parsing.
- Preserve ontology class IDs, labels, relationships, datatypes, source files, and version metadata.
- Produce deterministic normalized output for codegen in Track 58.

## Done

Parser fixtures must pass for Turtle and JSON-LD, malformed inputs must fail with typed errors, and the handoff must record task commits, phase review, push, and GitHub Actions review.
