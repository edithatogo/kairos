# Handoff — 09 TypeScript/Wasm Binding

## Summary

TypeScript/Wasm work should remain a local binding surface with explicit bundle validation and no release or registry side effects.

## Files changed

`conductor/tracks/09-typescript-wasm-binding/test-matrix.md`
`conductor/tracks/09-typescript-wasm-binding/handoff.md`

## Contracts consumed

- Track 01 core type and scheduler contracts.
- Track 12 conformance fixture contracts.
- Track 14 docs workflow only if the bundle publishes usage docs.

## Contracts changed

- TypeScript-facing API surface and Wasm adapter boundaries only.

## Tests added

- Binding tests and bundle validation.
- Conformance parity tests against shared fixtures when available.

## Known risks

- Wasm toolchain drift across Node and package-manager environments.
- Package layout drift before Track 15 owns dry-run release validation.
- Cross-language fixture mismatch if Track 12 changes after bundle code is written.

## Integration notes

- Keep this track focused on local TypeScript/Wasm validation.
- Do not add npm publish automation or registry credentials here.

