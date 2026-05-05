# KairoECS Documentation

KairoECS is a Rust-first simulation engine for deterministic event scheduling, ECS-style state, Arrow telemetry, and polyglot bindings.

## Local workflow

- `just docs-build`
- `just docs-bootstrap`
- `just docs-dev`
- `just check-docs`
- `just validate-conductor`
- `just validate-tracks`
- `just validate-track-docs`
- `just validate-conformance`

## Current docs tree

- `docs/adr/` for architecture decisions, naming, and release staging.
- `docs/api/api-review-template.md` for API review intake.
- `docs/benchmarks/benchmark-policy.md` for benchmark policy and comparability.
- `docs/community/` for contributor onboarding, governance, adoption, and roadmap notes.
- `docs/design/api-review.md` for design review guidance.
- `docs/interoperability/standards-review.md` for interoperability standards review.
- `docs/release/` for release and supply-chain checklists.
- `docs/research/` for citation and reproducibility guidance.
- `docs/trustworthy-simulation/` for replay, seeds, and uncertainty notes.

## Contributor commands

- `cd website && npm ci && npm run build`
- `cd website && npm start`
- `just docs-dev`
