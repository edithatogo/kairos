# Track 68 Agent Contract

## Owner

- `des-api-agent + contracts-agent + streaming-agent`

## Owned paths

- `crates/kairo-ecs-des/`
- `crates/kairo-ecs-streaming/`
- `docs/design/boundary-channels.md`
- `conductor/tracks/68-inter-unit-boundary-channels-handover-sinks/`

## Rules

- Preserve deterministic virtual-time ordering and seed replay invariants across all changes.
- In-flight entities in boundary channels must remain strictly conserved during save/restore checkpoints.
- Follow the single-maintainer harness and automatic phase closeout gate before advancing status.

## Cross-owner planning handoff

The 20 October composition revision changes planning only. Freeze shared contracts with existing owners before any runtime write; coordinate Arrow through 04/62 and public/core changes through 01/25. Preserve parallel writers and old evidence. Never infer atomic grants from condition events, or completed transfer from message acceptance.
