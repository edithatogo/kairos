# KairoECS Docs Overview

This directory is the source tree for KairoECS user, contributor, and release documentation. The static website in `website/` builds from this tree and the link manifest at `website/docs-link-manifest.json`, including generated HTML pages for the manifest navigation targets and the generated docs index used by the quality gate.

## Reading Paths

- New users: start with `docs/scenarios/factory-bottleneck-run-replay.md`, `docs/starter-kits/README.md`, and the binding README for your language.
- Runner users: use `docs/cli/kairo-ecs-cli.md` for the scenario CLI quickstart and `docs/scenarios/factory-bottleneck-run-replay.md` for the committed smoke path.
- Simulation authors: use `docs/trustworthy-simulation/verification-validation-uncertainty.md`, `docs/trustworthy-simulation/replay-and-seeds.md`, and `docs/validation/factory-bottleneck-v1-vvuq-note.md`.
- Contributors: use `docs/developer-experience/docs-workflow.md`, `docs/community/contributor-onboarding.md`, and `docs/design/api-review.md`.
- Documentation platform: use `docs/developer-experience/docs-platform.md` for the current site vs Astro/Starlight boundary.
- Release reviewers: use `docs/release/release-checklist.md`, `docs/release/compatibility.md`, and `docs/release/supply-chain-verification.md`.
- Cloud/HPC users: use `docs/cloud-hpc/specialized-compute-options.md` and the provider-specific batch docs under `docs/cloud-hpc/`.
- AWS accelerator users: start with `docs/cloud-hpc/aws-trainium-inferentia.md`.
- Learning coverage: use `docs/tutorials/coverage-matrix.md` to see which languages have tutorials, examples, and notebook coverage.

## Documentation Inventory

- `adr/`: architectural decisions and release-staging decisions.
- `api/`: API review templates.
- `benchmarks/`: benchmark policy and reproduction guides.
- `community/`: contributor onboarding, adoption, governance, roadmap, model zoo, and playground notes.
- `design/`: compatibility governance and protected-surface review.
- `interoperability/`: standards review notes.
- `model-zoo/` and `starter-kits/`: example discovery and starter-kit maturity guidance.
- `cli/`: experiment runner command reference and scenario smoke quickstart.
- `cloud-hpc/`: cloud batch, TPU, and accelerator guidance.
- `playground/` and `scenarios/`: runnable example surfaces and scenario replay notes.
- `developer-experience/`: docs workflow and docs-platform status notes.
- `release/`: changelog, compatibility, maintenance, release, and supply-chain guidance.
- `research/`: citation and archival guidance.
- `trustworthy-simulation/` and `validation/`: reproducibility, uncertainty, and VVUQ evidence.
- `tutorials/`: learning paths, notebook guidance, and language coverage matrix.

## Quality Gate

Run these from the repository root:

```powershell
npm --prefix website ci
npm --prefix website run check:links
npm --prefix website run check:quality
npm --prefix website run check:all
npm --prefix website run build
powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\14-docs-site-education\validate-docs-site.ps1
```

The docs system is dependency-light by design: the site generator uses Node.js standard library modules only, so it can run offline after checkout. `website/build/docs-index.json` records both the navigation entries and generated source-backed pages, and `npm --prefix website run check:all` wraps the link, build, and quality checks into the same CI-style flow the track documents.
