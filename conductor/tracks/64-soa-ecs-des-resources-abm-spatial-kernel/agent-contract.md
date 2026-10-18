# Track 64 Agent Contract

## Owner

- `ecs-agent + des-api-agent + abm-api-agent + core-scheduler-agent`

## Owned paths

- `crates/kairo-ecs-state/`
- `crates/kairo-ecs-des/`
- `crates/kairo-ecs-abm/`
- `examples/flow/`
- `conductor/tracks/64-soa-ecs-des-resources-abm-spatial-kernel/`

## Rules

- Preserve deterministic virtual-time ordering and seed replay invariants across all changes.
- Admit official upstream dependencies behind explicit feature flags where appropriate and update `deny.toml` and lockfiles in the same reviewed slice.
- Follow the single-maintainer harness and automatic phase closeout gate before advancing status.
