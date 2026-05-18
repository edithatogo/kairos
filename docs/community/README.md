# Community

This directory is the public community entry point for KairoECS. It is intentionally small and points to checked-in repo surfaces rather than unpublished packages, registries, or future demos.

## Start here

| Goal | Page | What it proves |
|---|---|---|
| Decide whether KairoECS fits your use case | [Adoption path](adoption.md) | The project has a clear first-user path and honest maturity labels. |
| Follow a tutorial | [Tutorial learning paths](../tutorials/index.md) | Rust, Python, Wasm/TypeScript, and model-building paths are source-backed and claim-bounded. |
| Find a concrete example | [Model zoo](model-zoo.md) | Examples are listed with paradigm, maturity, and checked-in README paths. |
| Start from a domain | [Starter kits](../starter-kits/README.md) | Domain-oriented entry points point at real example paths and expected outputs. |
| Make a first contribution | [Contributor onboarding](contributor-onboarding.md) | A contributor can find setup, issue, branch, check, and PR expectations. |
| Understand project governance | [Governance](governance.md) | Decision-making, conduct, security, and maintainer paths are discoverable. |
| Track staged maturity | [Roadmap](roadmap.md) | Alpha, beta, and stable claims remain behind explicit gates. |
| Inspect demo intent | [Playground](playground.md) | Browser demos are described without implying unavailable assets exist. |
| Review docs platform status | [Docs platform status](../developer-experience/docs-platform.md) | The current Node docs site and the Astro/Starlight roadmap remain explicit. |
| Review learning coverage | [Learning Coverage Matrix](../tutorials/coverage-matrix.md) | Tutorials, examples, and notebooks are inventoried by language instead of by assumption. |

## R2 onboarding-docs gate

Track 17 treats these files as the minimum real community slice:

- `docs/community/README.md`
- `docs/community/adoption.md`
- `docs/community/contributor-onboarding.md`
- `docs/community/model-zoo.md`
- `examples/model-zoo/README.md`
- `examples/model-zoo/model-zoo.yaml`
- `conductor/tracks/17-community-adoption-education-ecosystem/test-matrix.md`

The gate passes only when the files exist, relative links resolve, the model-zoo docs agree with the YAML inventory, and the Track 17 test matrix names the `onboarding-docs` gate.

The gate also proves that the root contributor guide, code of conduct,
security path, issue templates, and discussion categories stay discoverable:

- `CONTRIBUTING.md`
- `CODE_OF_CONDUCT.md`
- `SECURITY.md`
- `.github/DISCUSSION_CATEGORIES.md`
- `.github/ISSUE_TEMPLATE/docs_issue.yml`
- `.github/ISSUE_TEMPLATE/model_contribution.yml`
- `.github/ISSUE_TEMPLATE/track.yml`

## R2 docs-tutorials gate

The tutorial layer is intentionally checked-in and offline:

- `docs/tutorials/index.md`
- `docs/tutorials/rust-getting-started.md`
- `docs/tutorials/python-getting-started.md`
- `docs/tutorials/wasm-getting-started.md`
- `docs/tutorials/model-building.md`
- `docs/tutorials/validate-tutorials.ps1`
- `examples/docs/README.md`

Run `powershell -NoProfile -ExecutionPolicy Bypass -File docs/tutorials/validate-tutorials.ps1`
after changing tutorial links, learning-path names, or example cross-links.
