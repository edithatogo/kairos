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
- Dynamic spawning must not invalidate iteration; define capacity bounds and failures. Prove preallocation where an allocation-free hot path is required.
- Follow the single-maintainer harness and automatic phase closeout gate before advancing status.

## Cross-owner planning handoff

The 20 October composition revision changes planning only. Freeze shared contracts with existing owners before any runtime write; coordinate Arrow through 04/62 and public/core changes through 01/25. Preserve parallel writers and old evidence. Never infer atomic grants from condition events, or completed transfer from message acceptance.
