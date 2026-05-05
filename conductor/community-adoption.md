# KairoECS Community Adoption Strategy

KairoECS should compete on trust and usability, not just raw speed. The project must make it easy to discover, install, learn, cite, benchmark, and contribute.

## Adoption priorities

1. A polished landing page with honest maturity labels.
2. A model zoo with runnable examples and named inventory paths.
3. Tutorials that start with a real simulation problem.
4. Reproducible benchmark comparisons.
5. Clear compatibility promises.
6. Contributor onboarding and public governance.

## Owned surfaces

- `website/src/index.md`
- `docs/community/adoption.md`
- `docs/community/model-zoo.md`
- `docs/community/playground.md`
- `docs/community/contributor-onboarding.md`
- `examples/model-zoo/model-zoo.yaml`

## Community release gate

No public beta should ship without:

- at least 3 runnable examples: one DES, one ABM, one hybrid
- a docs site
- a citation file
- a contribution guide
- issue templates
- a security policy
- a benchmark harness skeleton
- a conformance fixture users can run locally
