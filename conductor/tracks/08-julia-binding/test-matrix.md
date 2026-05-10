# Test Matrix — 08 Julia Binding

## Required tests

| Gate | Command | Status | Evidence |
|---|---|---|---|
| Package tests | `julia --project=. -e 'using Pkg; Pkg.test()'` from `bindings/julia/` | pass | Passed on 2026-05-09 after `scoop install julia`; Julia 1.12.2 ran package tests, deterministic ordering, Arrow smoke roundtrip, schema facade, native FFI status, conformance fixture bridge, and event-log smoke payload tests. |
| Environment resolution | `julia --project=. -e 'using Pkg; Pkg.instantiate()'` from `bindings/julia/` | pass | `Pkg.test()` instantiated the package test environment on 2026-05-09 using the repo package with stdlib `Test`; no registry package dependencies were added to `Project.toml`. |
| Precompile smoke | `julia --project=. -e 'using Pkg; Pkg.precompile()'` from `bindings/julia/` | pass | `Pkg.test()` precompiled the package/test dependencies on 2026-05-09 before running tests. |
| Conformance bridge | `julia --project=. -e 'include("test/runtests.jl")'` from `bindings/julia/` | pass | Covered by `Pkg.test()` on 2026-05-09; conformance fixture bridge reported 12 passing assertions. |
| Arrow roundtrip smoke | `julia --project=. -e 'include("test/test_arrow.jl")'` from `bindings/julia/` | pass | Passed on 2026-05-09 with 3 passing event-log smoke payload roundtrip assertions. |

## Implemented coverage

- `ordered_events` returns events sorted by `(time_ticks, priority, sequence)`.
- `arrow_event_log_schema` exposes the `kairo_ecs.event_log.v1` field order without requiring Arrow.jl at package load time.
- `EventLogBatch`, `to_smoke_bytes`, and `from_smoke_bytes` provide a dependency-light event-log roundtrip gate that preserves field order, 128-bit tick encoding, nullable fields, and deterministic event ordering.
- `ConformanceFixture`, `binding_fixture_ids`, `ready_fixture_ids`, `fixture_status`, and `conformance_report` expose a local fixture bridge that keeps ready and planned Track 08 coverage separate.
- `ffi_status` and `is_ffi_configured` explicitly report that native FFI is not configured.

## Focused local validation

- `node tests/conformance/track07_13_hardening_check.mjs` verifies this track no longer claims package-publication ownership and records the no-release boundary.
- `git diff --check -- bindings/julia conductor/tracks/08-julia-binding conductor/tracks.yaml conductor/tracks.md conductor/phase-closeout.yaml conductor/status.md conductor/track-map.md` passed on 2026-05-08 and verifies the owned Julia binding/doc/status diff has no whitespace errors.
- `rg -n "EventLogBatch|to_smoke_bytes|from_smoke_bytes|test_arrow" bindings/julia conductor/tracks/08-julia-binding -S` passed on 2026-05-08 and verifies the roundtrip bridge is exported and covered.
- `rg -n "ConformanceFixture|binding_fixture_ids|ready_fixture_ids|conformance_report|fixture_status" bindings/julia -S` verifies the fixture bridge is exported and covered.
- `node tests/conformance/track07_13_hardening_check.mjs` passed on 2026-05-09 for binding-track publication-boundary checks.
- `git diff --check -- bindings/julia conductor/tracks/08-julia-binding` passed on 2026-05-09.
- `rg -n "EventLogBatch|to_smoke_bytes|from_smoke_bytes|test_arrow|ConformanceFixture|binding_fixture_ids|ready_fixture_ids|conformance_report|fixture_status" bindings/julia conductor/tracks/08-julia-binding -S` passed on 2026-05-09 and verifies the roundtrip and fixture bridge APIs remain exported and covered.
- `julia --version` returned `julia version 1.12.2` on 2026-05-09 after installing Julia through Scoop.
- `julia --project=. -e 'using Pkg; Pkg.test()'` passed on 2026-05-09 from `bindings/julia/`.
- `julia --project=. -e 'include("test/test_arrow.jl")'` passed on 2026-05-09 from `bindings/julia/`.

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
