# Track 69 Agent Contract

## Owner

- `des-api-agent + vv-uq-agent + core-scheduler-agent`

## Owned paths

- `crates/kairo-ecs-rng/`
- `crates/kairo-ecs-des/`
- `docs/design/competing-risks.md`
- `conductor/tracks/69-stochastic-competing-risks-hazard-engine/`

## Rules

- Preserve deterministic virtual-time ordering and seed replay invariants across all changes.
- Ensure hazard sampling draws strictly from an isolated RNG substream without leaking state into surrounding systems.
- Follow the single-maintainer harness and automatic phase closeout gate before advancing status.

## Cross-owner planning handoff

The 20 October composition revision changes planning only. Freeze shared contracts with existing owners before any runtime write; coordinate Arrow through 04/62 and public/core changes through 01/25. Preserve parallel writers and old evidence. Never infer atomic grants from condition events, or completed transfer from message acceptance.
