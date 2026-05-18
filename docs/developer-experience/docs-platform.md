# Docs Platform Parity Note

KairoECS currently uses a dependency-light custom Node docs site under
`website/`. That site builds from the checked-in `docs/` tree and the
`website/docs-link-manifest.json` manifest, so the repository has one
consistent source of truth for navigation, quality checks, and generated
docs-index output.

## Current site

- The live site is the custom Node-based implementation, not Astro/Starlight.
- `npm --prefix website run check:links` validates the manifest and source
  docs tree.
- `npm --prefix website run check:quality` validates build outputs, home-page
  coverage, generated pages, and the size budget.
- `npm --prefix website run check:all` wraps the link, build, and quality
  checks.

## Roadmap target

The public roadmap still names Astro/Starlight as the target documentation
framework. That target is the intended replacement for the custom Node site,
but the repo should not treat the migration as complete until the same docs
surface behavior is proven in the new stack.

## Explicit parity gaps

These are the current gaps that keep the custom Node site and the Astro/
Starlight roadmap distinct:

- The current site is still responsible for the checked-in docs tree, link
  manifest, generated docs-index output, and the docs quality gates.
- Route and navigation parity must be preserved for the required docs paths
  listed in `website/docs-link-manifest.json`.
- The generated home page and docs index must continue to surface the same
  contributor, learning, release, and examples entry points.
- The migration must not regress the offline-first, dependency-light local
  workflow that the current site supports.

## Track 41 closure decision

Track 41 closes on a parity-boundary decision, not on a framework migration.
The current custom Node site remains the active implementation because it
already preserves the docs tree, route manifest, generated docs index, and local
quality gates. A future Astro/Starlight migration must replace those behaviors
before it can claim to supersede the current site.

The parity boundary is complete for this track when:

- `website/docs-link-manifest.json` names the required docs routes.
- `npm --prefix website run check:all` validates links, build output, and
  docs-quality checks.
- `node scripts/dx/validate-docs-workflow.mjs` proves the local docs workflow
  and dev-server smoke path.
- `node scripts/validation/validate-learning-coverage.mjs` and
  `python notebooks/validate_notebooks.py` keep the learning inventory and
  notebook assets synchronized.

## What this page is for

This is a current-state parity note, not a design proposal. It keeps the repo
honest until the docs-stack migration is explicitly executed and verified in a
future migration slice.
