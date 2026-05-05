# Test Matrix — 08 Julia Binding

## Required tests

| Gate | Command | Status | Evidence |
|---|---|---|---|
| Package tests | `julia --project=. -e 'using Pkg; Pkg.test()'` from `bindings/julia/` | blocked locally | `Get-Command julia` failed on 2026-05-06 because Julia is not on PATH. Tests are present in `bindings/julia/test/runtests.jl`. |
| Environment resolution | `julia --project=. -e 'using Pkg; Pkg.instantiate()'` from `bindings/julia/` | blocked locally | `Get-Command julia` failed on 2026-05-06. Package has no registry dependencies in this slice. |
| Precompile smoke | `julia --project=. -e 'using Pkg; Pkg.precompile()'` from `bindings/julia/` | blocked locally | `Get-Command julia` failed on 2026-05-06. |
| Conformance bridge | `julia --project=. -e 'include("test/runtests.jl")'` from `bindings/julia/` | blocked locally | Uses local deterministic ordering and schema facade checks until Track 12 fixture runner is wired; local execution requires Julia. |

## Implemented coverage

- `ordered_events` returns events sorted by `(time_ticks, priority, sequence)`.
- `arrow_event_log_schema` exposes the `kairo_ecs.event_log.v1` field order without requiring Arrow.jl at package load time.
- `ffi_status` and `is_ffi_configured` explicitly report that native FFI is not configured.

## Focused local validation

- `node tests/conformance/track07_13_hardening_check.mjs` verifies this track no longer claims packaging ownership and records the no-release boundary.
- `julia --project=. -e 'using Pkg; Pkg.test()'` remains the package smoke command once Julia is on `PATH`.

## Future-surface controls

- Do not add General registry publishing, package server release automation, or credentials here.
- Do not expand into Python, R, TypeScript, C#, or Go surfaces.
- Do not widen beyond the Julia package and its conformance adapters.
- Stop at local package validation until Track 12 owns fixture parity and Track 15 owns dry-run release planning.

## CI command

```bash
cd bindings/julia
julia --project=. -e 'using Pkg; Pkg.instantiate()'
julia --project=. -e 'using Pkg; Pkg.test()'
```

