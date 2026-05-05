# Handoff: Track 25 API Design Review & Compatibility Governance

## Summary

Captured the compatibility policy surface so release planning can distinguish stable, experimental, and migration-only APIs across the current Rust crates and binding package roots.
The policy now needs to be enforced against the exact crate and package root names in the repo, not abstract categories.

## Files changed

`conductor/tracks/25-api-design-review-compatibility-governance/plan.md`, `conductor/tracks/25-api-design-review-compatibility-governance/test-matrix.md`, `conductor/tracks/25-api-design-review-compatibility-governance/handoff.md`

## Contracts consumed

`conductor/delivery-readiness-checklist.md`, `conductor/quality-gates.md`, `conductor/contracts/versioning-compatibility.md`

## Release gates affected

Compatibility review, ADR requirements, migration-note requirements, and release-hold decisions now sit on the public release path for `crates/kairo-ecs-types`, `crates/kairo-ecs-core`, `crates/kairo-ecs-state`, `crates/kairo-ecs-rng`, `bindings/python`, `bindings/r`, `bindings/julia`, `bindings/typescript`, `bindings/csharp`, and `bindings/go`.
Any rename, split, merge, or removal of one of those roots should be treated as a breaking change unless an ADR says otherwise.

## Risks and unresolved questions

The main risk is a later API change outrunning the compatibility policy and forcing a release hold. The policy should be consulted before any crate or package root rename.
Another failure mode is a release note claiming compatibility while the package catalog or matrix still points at the old root.
