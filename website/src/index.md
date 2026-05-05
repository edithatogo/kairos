# KairoECS Documentation

KairoECS is a Rust-first simulation engine for deterministic event scheduling, ECS-style state, Arrow telemetry, and polyglot bindings.

## Start Here

- [Community adoption](../../docs/community/adoption.md)
- [Model zoo](../../docs/community/model-zoo.md)
- [Playground](../../docs/community/playground.md)
- [Citation and archival](../../docs/research/citation.md)
- [Trustworthy simulation](../../docs/trustworthy-simulation/verification-validation-uncertainty.md)

## Local workflow

- `just docs-build`
- `just docs-dev`
- `just check-docs`
- `just validate-conductor`
- `just validate-tracks`
- `just validate-track-docs`
- `just validate-conformance`
- `just dev-validate`

## Current docs tree

- `docs/adr/` for architecture decisions, naming, and release staging.
- `docs/api/api-review-template.md` for API review intake.
- `docs/benchmarks/benchmark-policy.md` for benchmark policy and comparability.
- `docs/community/` for contributor onboarding, governance, adoption, model-zoo guidance, roadmap notes, and the playground.
- `docs/community/adoption.md` for the adoption path.
- `docs/community/model-zoo.md` for the example inventory.
- `docs/community/playground.md` for the interactive demo surface.
- `docs/design/api-review.md` for design review guidance.
- `docs/interoperability/standards-review.md` for interoperability standards review.
- `docs/release/` for release and supply-chain checklists.
- `docs/research/citation.md` for citation and archival guidance.
- `docs/trustworthy-simulation/` for replay, seeds, and uncertainty notes.

## Contributor commands

- `cd website && npm ci && npm run build`
- `cd website && npm start`
- `just docs-dev`

## Site owner

Track 14 owns the public docs surface, while Track 27 owns the contributor workflow commands that keep the site buildable.
