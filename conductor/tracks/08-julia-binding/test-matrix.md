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
- `ConformanceFixture`, `binding_fixture_ids`, `ready_fixture_ids`, `fixture_status`, and `conformance_report` expose a local fixture bridge that keeps ready and planned Track 08 coverage separate.
- `ffi_status` and `is_ffi_configured` explicitly report that native FFI is not configured.

## Focused local validation

- `node tests/conformance/track07_13_hardening_check.mjs` verifies this track no longer claims packaging ownership and records the no-release boundary.
- `git diff --check -- bindings/julia conductor/tracks/08-julia-binding` verifies the owned Julia binding/doc diff has no whitespace errors.
- `rg -n "ConformanceFixture|binding_fixture_ids|ready_fixture_ids|conformance_report|fixture_status" bindings/julia -S` verifies the fixture bridge is exported and covered.
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
## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, phase-closeout ledger evidence, cleaned commit/push evidence, and blocker recording. At actual closeout, run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.