# Track 67 Handoff: Linked Entity Hierarchies, Dynamic Spawning & Cascaded Lifecycles

Status: Planned

Freshness: 2026-10-20

## Summary

Track proposed to introduce runtime linked entities, dynamic mid-simulation entity instantiation, and cascaded lifecycle management in `kairo-ecs-core` and `kairo-ecs-abm`.

## Files changed

- `conductor/tracks.yaml`
- `conductor/tracks.md`
- `conductor/tracks/67-linked-entity-hierarchies-dynamic-spawning/*`

## Contracts consumed

- `crates/kairo-ecs-core` entity ID and generation allocator.
- `crates/kairo-ecs-state` archetype tables.

## Contracts changed

- Added `LinkedEntityPair` and `EntitySpawningRelationship` traits in `kairo-ecs-core`.

## Tests added

- Proposed in `test-matrix.md`.

## Known risks

- Deferred spawning buffer overhead; see `risk-register.md`.

## Follow-up issues

- None currently identified.

## Integration notes

- Directly consumed by `careops-core` and downstream clinical tracks (`careops-birthing`) for maternal-neonate pair modeling.
