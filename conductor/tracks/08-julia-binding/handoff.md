# Handoff — 08 Julia Binding

## Summary

Julia binding work now has a minimal real package slice and stops before registry publication or native runtime loading.

The package exposes pure-Julia deterministic event ordering, an event-log schema facade aligned with the Track 04 `kairo_ecs.event_log.v1` field order, and explicit FFI status reporting that remains not configured until a safe Track 02 native artifact handoff exists.

## Files changed

`bindings/julia/Project.toml`
`bindings/julia/README.md`
`bindings/julia/src/KairoECS.jl`
`bindings/julia/test/runtests.jl`
`conductor/tracks/08-julia-binding/test-matrix.md`
`conductor/tracks/08-julia-binding/handoff.md`

## Contracts consumed

- Track 01 core types and scheduler contracts.
- Track 12 conformance fixture contracts.
- Track 14 docs workflow only if the Julia package ships documentation.

## Contracts changed

- Julia package exports and adapter compatibility only.
- No native FFI ABI or Arrow schema contract changes.

## Tests added

- Package test coverage for exported Julia entrypoints.
- Deterministic scheduler ordering smoke coverage using `(time_ticks, priority, sequence)`.
- Event-log schema facade field-order coverage.
- Native FFI status coverage proving the binding reports not configured rather than attempting an unsafe load.

## Validation

- `Get-Command julia` failed on 2026-05-06 because Julia is not on PATH.
- `julia --project=. -e 'using Pkg; Pkg.test()'` from `bindings/julia/` is blocked locally until Julia is available.
- `git diff --check -- bindings\julia conductor\tracks\08-julia-binding` passed with only line-ending normalization warnings.
- `rg -n "ordered_events|arrow_event_log_schema|ffi_status|blocked locally|not configured" bindings\julia conductor\tracks\08-julia-binding -S` confirmed the implemented API and evidence markers.

## Known risks

- Environment resolution drift between local development and future registry-ready packaging.
- Cross-language fixture drift if Track 12 changes after this binding is implemented.
- Scope creep into registry or release automation before Track 15 owns it.
- Native library loading remains intentionally blocked until Track 02 provides a safe artifact layout.

## Integration notes

- Keep the Julia surface isolated from other language bindings and from release automation.
- Do not add General registry publication or package-server controls here.
- No release, registry, or remote publication side effects were performed.
