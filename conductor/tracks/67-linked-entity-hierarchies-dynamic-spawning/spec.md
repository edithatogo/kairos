# Track 67: Linked Entity Hierarchies, Dynamic Spawning & Cascaded Lifecycles

## Objective

Provide native ECS and ABM support for runtime linked entities (`LinkedEntityPair`, `ParentChildBinding`, `EntitySpawningRelationship`), allowing dynamic entity instantiation during discrete-event execution (e.g., maternal entity dynamically instantiating neonate entity, transport spawning cargo, host spawning agent) with synchronized or independent state transitions, component inheritance, dual-entity telemetry recording, and deterministic cascaded lifecycle management.

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

Non-breaking extension to ECS core and ABM spatial runtime. Existing independent entities continue functioning without modification. Opt-in via `LinkedEntity` and `HierarchyComponent` annotations.
