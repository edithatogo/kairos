# Handoff: Track 17 Community Adoption, Education & Ecosystem

## Summary

Anchored the community adoption surface to a concrete R2 onboarding slice: community index, contributor path, first-user adoption path, model-zoo inventory bridge, governance map, maturity roadmap, and explicit `onboarding-docs` gate evidence.

Documentation Worker B added the `docs-tutorials` slice: Rust, Python,
Wasm/TypeScript, and model-building tutorials with conservative claim
boundaries and source-backed example cross-links.

## Files changed

- `docs/community/README.md`
- `docs/community/adoption.md`
- `docs/community/contributor-onboarding.md`
- `docs/community/governance.md`
- `docs/community/model-zoo.md`
- `docs/community/roadmap.md`
- `docs/tutorials/index.md`
- `docs/tutorials/rust-getting-started.md`
- `docs/tutorials/python-getting-started.md`
- `docs/tutorials/wasm-getting-started.md`
- `docs/tutorials/model-building.md`
- `docs/tutorials/validate-tutorials.ps1`
- `examples/docs/README.md`
- `examples/model-zoo/README.md`
- `conductor/tracks/17-community-adoption-education-ecosystem/test-matrix.md`
- `conductor/tracks/17-community-adoption-education-ecosystem/handoff.md`

## Contracts consumed

`conductor/workflow.md`, `conductor/package-catalog.md`, `conductor/delivery-readiness-checklist.md`, `website/src/index.md`, `website/package.json`, `examples/model-zoo/model-zoo.yaml`

## Release gates affected

Community docs, link integrity, maturity labels, and `just docs-build` gate the adoption path before release notes or contributor guidance point at it. The Track 17 `onboarding-docs` gate now has an explicit file-existence and content check covering:

- community index
- contributor onboarding
- adoption path
- model-zoo docs and YAML inventory bridge
- docs tutorial index, Rust/Python/Wasm learning paths, model-building path,
  and tutorial/example cross-links
- Track 17 gate evidence

## Risks and unresolved questions

The concrete risk is drift between the discovery page, the package catalog, and the contributor entry points. Rerun `just docs-build` and `just check-docs` after any docs-tree move or maturity-label update.

## Validation commands

- `powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/17-community-adoption-education-ecosystem/validate-community-onboarding.ps1`
- `powershell -NoProfile -ExecutionPolicy Bypass -File docs/tutorials/validate-tutorials.ps1`
- `rg -n "onboarding-docs|First contribution path|First-user path|Inventory update rule" docs/community examples/model-zoo conductor/tracks/17-community-adoption-education-ecosystem`
- `rg -n "mm1_queue|factory_bottleneck|flocking|emergency_department_flow" docs/community/model-zoo.md examples/model-zoo/README.md examples/model-zoo/model-zoo.yaml`
- `pwsh -NoProfile -Command '$required = @("docs/community/README.md","docs/community/adoption.md","docs/community/contributor-onboarding.md","docs/community/model-zoo.md","examples/model-zoo/README.md","examples/model-zoo/model-zoo.yaml","conductor/tracks/17-community-adoption-education-ecosystem/test-matrix.md"); $missing = foreach ($p in $required) { if (-not (Test-Path -LiteralPath $p)) { $p } }; if ($missing) { $missing; exit 1 }; "onboarding-docs required files present"'`
- `just check-docs`

## Review-hardening update

Added a track-local onboarding validator that checks the community docs slice,
model-zoo inventory bridge, maturity labels, and first-user/contributor entry
points without requiring network or registry access.
