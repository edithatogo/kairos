# Handoff — 14 Documentation Site & Education

## Summary

Documented the docs site as a static `website/` build, aligned the site home with the repository docs tree, and added `npm ci`, `npm run build`, and `npm run dev` commands.

## Files changed

`website/package.json`, `website/scripts/build.js`, `website/scripts/dev.js`, `website/src/index.md`, `templates/website/package.json`, `templates/website/scripts/build.js`, `templates/website/scripts/dev.js`, `templates/website/src/index.md`, `conductor/tracks/14-docs-site-education/spec.md`, `conductor/tracks/14-docs-site-education/plan.md`, `conductor/tracks/14-docs-site-education/agent-contract.md`, `conductor/tracks/14-docs-site-education/test-matrix.md`

## Contracts consumed

`conductor/workflow.md`, `conductor/contracts/`, `docs/`

## Contracts changed

None.

## Tests added

Build and repository-doc-tree checks are specified in `test-matrix.md`.

## Known risks

The site currently renders a single static home page; routed pages for each docs section are not implemented.

## Integration notes

Use `just docs-build` for CI-style validation and `just docs-dev` for local preview.
