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
