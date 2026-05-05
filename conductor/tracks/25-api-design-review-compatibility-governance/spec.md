# Track 25: API Design Review & Compatibility Governance

## Purpose

Own the compatibility rules and review surface for Rust, C ABI, Arrow schemas, and host APIs so release planning can decide whether a change is safe to publish.

## Why this track exists

KairoECS is not only a Rust kernel. It is a multi-language research and engineering ecosystem. This track protects the project from shipping APIs before the compatibility contract, migration notes, and public review surface exist.

## Primary subagent

`api-governance-agent`

## Parallelization model

This track is designed to run in parallel with core implementation. The subagent owns compatibility policy, review templates, and release-gating docs only. It must not change runtime code, binding internals, or another worker's owned docs.

## Inputs

- `conductor/contracts/core-contract.md`
- `conductor/contracts/ffi-contract.md`
- `conductor/contracts/arrow-schema-contract.md`
- `conductor/contracts/conformance-contract.md`
- `conductor/package-ecosystem-plan.md`
- `reviews/red-team-report.md`

## Outputs

- A compatibility matrix that names the protected surfaces and the supported break policy.
- A review checklist for Rust, FFI, Arrow, and host APIs.
- Release-stage rules for alpha, beta, RC, and 1.0.
- Handoff notes for release, docs, and red-team subagents.

## Owned paths

- `conductor/api-design-review.md`
- `docs/design/`
- `conductor/tracks/25-api-design-review-compatibility-governance/`
- `conductor/delivery-readiness-checklist.md`
- `conductor/quality-gates.md`
- `conductor/contracts/versioning-compatibility.md`

## Blocked paths

- Implementation code in `crates/` — owned by Tracks 01-05.
- Binding source files in `bindings/` — owned by Tracks 06-11.
- Package manifests and release workflows — owned by Track 15.

## Acceptance criteria

- A release manager can see which API or schema changes require ADRs, migration notes, or a release hold.
- The compatibility rules are testable and tied to named surfaces.
- The track does not duplicate Track 01-05 implementation work.
- The review output is specific enough to block or permit release planning.

## Release implications

- Any change to a protected public surface requires explicit compatibility review before beta or later.
- Any breaking Rust, FFI, Arrow, or host API change must have a migration note and release decision.
- If the surface inventory is incomplete, release planning treats the change as unreviewed.

## Non-goals

- Replacing the core scheduler or ECS design.
- Publishing packages before naming, legal, security, and compatibility gates pass.
- Adding domain-specific complexity to `kairo-ecs-core`.
- Inventing new product behavior in the compatibility review doc.



