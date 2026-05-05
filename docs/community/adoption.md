# Community Adoption

KairoECS adoption should read like a product path, not an internal note dump.

## What a new user should find first

- A clear landing page with maturity labels.
- A model zoo with concrete example directories.
- A quick path from example to local execution.
- A contributor path that points at issue labels, governance, and checks.
- Citation and trust guidance that explains what a successful run proves and how to reference it.

## Discovery points

- `website/src/index.md`
- `docs/community/model-zoo.md`
- `docs/community/playground.md`
- `docs/community/contributor-onboarding.md`
- `docs/research/citation.md`
- `docs/trustworthy-simulation/replay-and-seeds.md`
- `docs/trustworthy-simulation/verification-validation-uncertainty.md`

## Adoption milestones

1. Open the home page and follow the community, model-zoo, playground, citation, and trust links.
2. Discover the project and its maturity labels.
3. Open a concrete example from the model zoo.
4. Run or inspect the example README.
5. Read the replay, uncertainty, and archival guidance.
6. Contribute through the documented issue and PR path.

## Public beta gate

No public beta should ship unless the following are true:

- at least three runnable examples exist: one DES, one ABM, one hybrid
- the docs site is buildable and links the adoption path
- `CITATION.cff` exists
- a contribution guide exists
- issue templates exist
- `SECURITY.md` exists
- the benchmark harness skeleton exists
- at least one conformance fixture can be run locally

See `conductor/community-adoption.md` and Track 17 for the operational gate.
