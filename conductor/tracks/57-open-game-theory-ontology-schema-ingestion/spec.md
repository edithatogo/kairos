# Track 57: Open Game Theory Ontology Subrepo and Schema Ingestion

## Objective

Create the standalone `open-game-theory-ontology/` subrepo surface and an ingestion crate that parses Turtle and JSON-LD game-theory schemas into a canonical intermediate representation.

## Scope

- Initialize `open-game-theory-ontology/` with README, schema layout, fixtures, and provenance policy.
- Add `crates/kairo-ecs-game-ontology/` for parser and IR code.
- Support Turtle and JSON-LD fixture ingestion without lossy string-only parsing.
- Preserve ontology class IDs, labels, relationships, datatypes, source files, and version metadata.
- Produce deterministic normalized output for codegen in Track 58.

## Release implications

- Public release wording may claim fixture-backed Turtle and JSON-LD ingestion only for the checked-in open-game-theory ontology fixtures and parser APIs covered by Track 57 tests.
- The ontology subrepo surface is source-controlled inside this repository; it is not a separately published registry artifact until a later release or publication track records that evidence.
- The parser crate may be used by Track 58 code generation only through deterministic normalized IR output; downstream generated-component claims remain owned by Track 58.
- Any release claim about broad ontology compatibility must remain blocked until external research-agent schemas are ingested, normalized, and validated through the same parser/test matrix.

## Blocked paths

- Do not mark Track 57 `Done` until the ingestion crate, fixture corpus, malformed-input tests, review/push records, strict git closeout, and GitHub Actions evidence are recorded in handoff and phase closeout.
- Do not claim full Turtle, RDF, OWL, JSON-LD framing, remote context resolution, or arbitrary ontology compatibility from the current minimal parser fixtures.
- Do not add pointer-owned or recursive graph topology while representing ontology relationships; downstream topology must remain Entity-ID and flat-ECS compatible.
- Do not publish generated game components from Track 57 alone; code generation and API review remain Track 58 scope.

## Done

Parser fixtures must pass for Turtle and JSON-LD, malformed inputs must fail with typed errors, and the handoff must record task commits, phase review, push, and GitHub Actions review.
