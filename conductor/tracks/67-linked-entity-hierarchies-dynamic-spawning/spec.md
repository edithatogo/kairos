# Track 67: Linked Entity Hierarchies, Dynamic Spawning & Cascaded Lifecycles

## Objective

Provide native ECS and ABM support for runtime linked entities (`EntityRelationship`, `ParentChildBinding`, `SpawnRelationship`), allowing dynamic entity instantiation during discrete-event execution (e.g., a job spawning subtasks or a transport spawning cargo records) with synchronized or independent state transitions, component inheritance, relationship-aware telemetry recording, and deterministic cascaded lifecycle management.

## Inputs

- `crates/kairo-ecs-core` entity and component registries.
- `crates/kairo-ecs-state` archetype and SoA storage columns.
- `crates/kairo-ecs-abm` spatial topology and agent interfaces.
- Tracks 01, 03, 64 architectural contracts.

## Outputs

- `crates/kairo-ecs-core/src/linked_entity.rs`
- `crates/kairo-ecs-state/src/hierarchy.rs`
- `crates/kairo-ecs-abm/src/spawning.rs`
- `docs/design/linked-entities.md`
- `conductor/tracks/67-linked-entity-hierarchies-dynamic-spawning/`

## Owned paths

- `crates/kairo-ecs-core/`
- `crates/kairo-ecs-state/`
- `crates/kairo-ecs-abm/`
- `docs/design/linked-entities.md`
- `conductor/tracks/67-linked-entity-hierarchies-dynamic-spawning/`

## Blocked paths

- `crates/kairo-ecs-des/` (owned by Track 68 for boundary channels).
- `crates/kairo-ecs-gpu/` (owned by Track 32).
- `crates/kairo-ecs-pdes/` (owned by Track 34/46).

## Release implications

Proposed additive extension; actual public API compatibility requires ADR and conformance review. Preserve independent-entity behaviour through conformance tests. Public type names and opt-in annotations remain provisional until owner review.

## Composition contract refinement — 20 October 2026

Relationships must support one-to-many cardinality and typed links rather than a universal pair. Stable model identities are distinct from recycled ECS handles. Specify termination and detach policy explicitly; independent children can survive their parent. Neutral parcel/job fixtures establish mechanism semantics; person/episode and maternal/infant meaning remain in CareOps. Review Track 59 placement without requiring downstream users to depend on game-theory solvers. Dynamic spawning needs safe iteration, bounded capacity and defined allocation failures; zero allocation in every inner loop is a benchmark/design goal only where preallocation is proven.

See [owner worklist and downstream join](../../design/careops-composition-plan-20261020.md). All work remains pending. Prospective source filenames are resolved at dispatch; they are not evidence that APIs exist.

## Acceptance and cross-owner gates

Require the phase test matrix, meaningful negative controls, exact input/source/output hashes, deterministic replay and failure/recovery receipts. Coordinate Arrow modifications through 04/62 and shared scheduler/public contracts through 01/25. Current broad registry paths are prospective: reserve bounded disjoint files and rebind on source drift. Publication, hardware runtime and clinical validation remain separate.
