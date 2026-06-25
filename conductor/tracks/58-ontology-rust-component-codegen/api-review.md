# Track 58 Generated API Review

Status: Accepted for Phase 1 local implementation

## Reviewed Surface

- kairo-ecs-game-ontology exposes generate_rust_components and CodegenError only behind the codegen feature.
- kairo-ecs-game-theory exposes generated component types only behind the generated-components feature.
- The generated fixture is open-game-theory-ontology/fixtures/generated/rust/game_components.rs.

## Decisions

- Relationship topology is represented only with Entity IDs in Vec<Entity> fields.
- No generated type uses Box, Rc, Arc, raw pointers, or self-referential ownership for graph edges.
- Domain identifiers are stable u64 newtypes for PlayerId, StrategyId, ActionId, InformationSetId, and GameNodeId.
- Generated Rust remains a fixture-backed compile target until a later task promotes file emission into a Cargo build step.

## Evidence Gates

- rustfmt --check crates/kairo-ecs-game-ontology/src/lib.rs crates/kairo-ecs-game-ontology/tests/codegen.rs
- cargo check -p kairo-ecs-game-ontology --features codegen --tests
- rustfmt --check crates/kairo-ecs-game-theory/src/lib.rs crates/kairo-ecs-game-theory/tests/generated_components.rs
- cargo check -p kairo-ecs-game-theory --features generated-components --tests
- node scripts/validation/validate-game-theory-codegen.mjs
- scripts/validate_conductor_phase_gates.ps1
- scripts/validate_conductor_dag.ps1

## Waivers

- Local cargo test executable runs are blocked on this Windows host by Git link.exe shadowing the MSVC linker. The accepted local substitute for this phase is cargo check --tests plus CI execution on the PR runner.
