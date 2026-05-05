# Handoff: Track 17 Community Adoption, Education & Ecosystem

## Summary

Anchored the community adoption surface to a concrete discovery and onboarding path: landing page, install guidance, quickstarts, example gallery, contribution entry points, and maturity labels all point at the same published docs and repo surfaces.

## Files changed

`conductor/tracks/17-community-adoption-education-ecosystem/plan.md`, `conductor/tracks/17-community-adoption-education-ecosystem/test-matrix.md`, `conductor/tracks/17-community-adoption-education-ecosystem/handoff.md`

## Contracts consumed

`conductor/workflow.md`, `conductor/package-catalog.md`, `conductor/delivery-readiness-checklist.md`, `website/src/index.md`, `website/package.json`

## Release gates affected

Community docs, link integrity, maturity labels, and `just docs-build` now gate the adoption path before release notes or contributor guidance point at it. Public beta should not ship unless the landing page, contributor guide, issue templates, and at least one runnable example for DES, ABM, and hybrid flows are visible.

## Risks and unresolved questions

The concrete risk is drift between the discovery page, the package catalog, and the contributor entry points. Rerun `just docs-build` and `just check-docs` after any docs-tree move or maturity-label update.
