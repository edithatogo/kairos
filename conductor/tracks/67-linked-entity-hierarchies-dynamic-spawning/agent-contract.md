# Track 67 Agent Contract

## Owner

- `ecs-agent + abm-api-agent + contracts-agent`

## Owned paths

- `crates/kairo-ecs-core/`
- `crates/kairo-ecs-state/`
- `crates/kairo-ecs-abm/`
- `docs/design/linked-entities.md`
- `conductor/tracks/67-linked-entity-hierarchies-dynamic-spawning/`

## Rules

- Preserve deterministic virtual-time ordering and seed replay invariants across all changes.
- Dynamic entity spawning during system/event execution must never invalidate concurrent iterator slices or cause memory reallocations in inner loops.
- Follow the single-maintainer harness and automatic phase closeout gate before advancing status.
