# Handoff — 14 Documentation Site & Education

## Summary

Documented the docs site as a static `website/` build, aligned the site home with the repository docs tree, and added `npm ci`, `npm run build`, `npm run check:links`, and `npm run dev` commands.

This R2 docs slice now reflects the implemented crate and binding surfaces, including the preview/native-not-configured binding status, and adds a local docs quality gate backed by `website/docs-link-manifest.json`.

## Files changed

`website/package.json`, `website/scripts/build.js`, `website/scripts/check-links.js`, `website/docs-link-manifest.json`, `website/src/index.md`, `examples/docs/README.md`, `conductor/tracks/14-docs-site-education/test-matrix.md`, `conductor/tracks/14-docs-site-education/handoff.md`

## Contracts consumed

`conductor/workflow.md`, `conductor/contracts/`, `docs/`

## Contracts changed

None.

## Tests added

Build, repository-doc-tree, binding-link, and docs-link-manifest checks are specified in `test-matrix.md`.

## Validation evidence

- `npm --prefix website ci`
- `npm --prefix website run check:links`
- `npm --prefix website run build`
- `Test-Path -LiteralPath 'website/build/index.html'`

## Known risks

The site currently renders a single static home page; routed pages for each docs section are not implemented. The link checker validates source Markdown targets and required paths, not generated HTML anchors.

## Integration notes

Use `just docs-build` for CI-style validation and `just docs-dev` for local preview.
