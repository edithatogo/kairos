# Track 63 Agent Contract

## Owner

- `python-agent + typescript-agent + r-agent + julia-agent + csharp-agent + go-agent + ffi-agent`

## Owned paths

- `bindings/python/`
- `bindings/typescript/`
- `bindings/r/`
- `bindings/julia/`
- `bindings/csharp/`
- `bindings/go/`
- `crates/kairo-ecs-wasm/`
- `crates/kairo-ecs-ffi/`
- `conductor/tracks/63-native-polyglot-wheels-wasm-bindgen-bridge/`

## Rules

- Preserve deterministic virtual-time ordering and seed replay invariants across all changes.
- Admit official upstream dependencies behind explicit feature flags where appropriate and update `deny.toml` and lockfiles in the same reviewed slice.
- Follow the single-maintainer harness and automatic phase closeout gate before advancing status.
