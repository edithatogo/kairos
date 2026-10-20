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
