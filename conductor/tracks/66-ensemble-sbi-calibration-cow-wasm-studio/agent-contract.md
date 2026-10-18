# Track 66 Agent Contract

## Owner

- `vv-uq-agent + experiment-agent + timetravel-agent + playground-agent`

## Owned paths

- `crates/kairo-ecs-cli/`
- `crates/kairo-ecs-debug/`
- `website/playground/`
- `docs/validation/`
- `docs/debugging/`
- `conductor/tracks/66-ensemble-sbi-calibration-cow-wasm-studio/`

## Rules

- Preserve deterministic virtual-time ordering and seed replay invariants across all changes.
- Admit official upstream dependencies behind explicit feature flags where appropriate and update `deny.toml` and lockfiles in the same reviewed slice.
- Follow the single-maintainer harness and automatic phase closeout gate before advancing status.
