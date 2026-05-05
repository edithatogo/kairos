# Community Adoption

KairoECS adoption should read like a product path, not an internal note dump.

## First-user path

| Step | Action | Checked-in target |
|---|---|---|
| Discover | Read the docs home page and this adoption page. | `website/src/index.md`, `docs/community/README.md` |
| Choose an example | Pick a model by paradigm and maturity. | `docs/community/model-zoo.md` |
| Learn the workflow | Follow a Rust, Python, Wasm/TypeScript, or model-building tutorial. | `docs/tutorials/index.md` |
| Inspect the entry point | Open the example README before running anything. | `examples/model-zoo/README.md` |
| Run or review | Follow the example README command or smoke-check note. | `examples/des/`, `examples/abm/`, `examples/hybrid/` |
| Trust the result | Read replay, seed, uncertainty, and citation guidance. | `docs/trustworthy-simulation/`, `docs/research/citation.md` |
| Contribute | Follow the contributor onboarding path. | `docs/community/contributor-onboarding.md` |

## What a new user should find first

- A clear landing page with maturity labels.
- A model zoo with concrete example directories.
- Tutorial learning paths for Rust, Python, Wasm/TypeScript, and first model design.
- A quick path from example to local execution.
- A contributor path that points at issue labels, governance, and checks.
- Citation and trust guidance that explains what a successful run proves and how to reference it.

## Discovery points

- `docs/community/README.md`
- `website/src/index.md`
- `docs/community/model-zoo.md`
- `docs/tutorials/index.md`
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

## Claim boundary

This page is an onboarding and discovery guide. It does not claim registry availability, production support, benchmark superiority, or stable APIs. Those claims remain gated by release, packaging, benchmark, conformance, and compatibility evidence in the relevant tracks.
