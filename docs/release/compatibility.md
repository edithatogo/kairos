# Release Compatibility and Deprecation Notes

This file is the compatibility note for the current release train. It must stay
aligned with `conductor/contracts/versioning-compatibility.md`.

## Current public roots

| Surface | Roots | R2 status |
|---|---|---|
| Rust crates | `crates/kairo-ecs-types`, `crates/kairo-ecs-core`, `crates/kairo-ecs-state`, `crates/kairo-ecs-rng` | Checked in; semver policy applies |
| C ABI | `crates/kairo-ecs-ffi`, `include/` | Compatibility-sensitive when present in release artifacts |
| Host bindings | `bindings/python`, `bindings/r`, `bindings/julia`, `bindings/typescript`, `bindings/csharp`, `bindings/go` | Checked in; dry-run packaging only until Track 15 clears publish gates |
| Arrow schemas | `schemas/arrow/` | Schema changes require versioning and compatibility notes |
| Conformance fixtures | `conformance/fixtures/` | Fixture output drift requires a compatibility note and version bump |
| Release artifacts | manifests, checksums, SBOM/provenance, archive metadata | Dry-run evidence only until publication gates pass |

## R2 compatibility posture

- No stable 1.0 compatibility promise is made by this R2 governance slice.
- Public roots must still be named in changelog and release notes when changed.
- Breaking changes require an ADR if they alter API, ABI, schema semantics,
  fixture determinism, package roots, or release-stage classification.
- Migration notes are required when a user must change code, fixtures, package
  names, or deployment scripts.

## Deprecation register

| Affected root | Deprecated item | Replacement | First noticed in | Earliest removal | Evidence |
|---|---|---|---|---|---|
| None | None recorded for R2 | Not applicable | Not applicable | Not applicable | Not applicable |

Add a row before removing any public feature, root, schema field, fixture
contract, or release artifact path.
