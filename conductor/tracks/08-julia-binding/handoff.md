# Handoff — 08 Julia Binding

## Summary

Julia binding work stays at the package level and stops before registry publication or broader runtime work.

## Files changed

`conductor/tracks/08-julia-binding/test-matrix.md`
`conductor/tracks/08-julia-binding/handoff.md`

## Contracts consumed

- Track 01 core types and scheduler contracts.
- Track 12 conformance fixture contracts.
- Track 14 docs workflow only if the Julia package ships documentation.

## Contracts changed

- Julia package exports and adapter compatibility only.

## Tests added

- Package test coverage for exported Julia entrypoints.
- Fixture parity checks for shared conformance inputs when available.

## Known risks

- Environment resolution drift between local development and future registry-ready packaging.
- Cross-language fixture drift if Track 12 changes after this binding is implemented.
- Scope creep into registry or release automation before Track 15 owns it.

## Integration notes

- Keep the Julia surface isolated from other language bindings and from release automation.
- Do not add General registry publication or package-server controls here.
