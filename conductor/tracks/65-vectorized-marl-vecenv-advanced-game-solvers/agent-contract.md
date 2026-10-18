# Track 65 Agent Contract

## Owner

- `game-theory-agent + ml-integration-agent + performance-agent`

## Owned paths

- `crates/kairo-ecs-game-theory/`
- `crates/kairo-ecs-ml/`
- `bindings/python/`
- `docs/game-theory/`
- `docs/ml/`
- `conductor/tracks/65-vectorized-marl-vecenv-advanced-game-solvers/`

## Rules

- Preserve deterministic virtual-time ordering and seed replay invariants across all changes.
- Admit official upstream dependencies behind explicit feature flags where appropriate and update `deny.toml` and lockfiles in the same reviewed slice.
- Follow the single-maintainer harness and automatic phase closeout gate before advancing status.
