# Track 62 Agent Contract

## Owner

- `arrow-agent + ffi-agent + interop-agent`

## Owned paths

- `crates/kairo-ecs-arrow/`
- `crates/kairo-ecs-ffi/`
- `include/kairo_ecs.h`
- `schemas/arrow/`
- `docs/parallel-io/`
- `conductor/tracks/62-official-arrow-parquet-zero-copy-ffi-spine/`

## Rules

- Preserve deterministic virtual-time ordering and seed replay invariants across all changes.
- Admit official upstream dependencies behind explicit feature flags where appropriate and update `deny.toml` and lockfiles in the same reviewed slice.
- Follow the single-maintainer harness and automatic phase closeout gate before advancing status.
