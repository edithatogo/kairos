# Handoff — 14 Documentation Site & Education

## Summary

Documented the docs site as a static `website/` build, aligned the site home with the repository docs tree, and added `npm ci`, `npm run build`, `npm run check:links`, `npm run check:quality`, `npm run check:all`, and `npm run dev` commands.

This R2 docs slice now reflects the implemented crate and binding surfaces, including the preview/native-not-configured binding status, and adds a manifest-driven docs navigation and quality gate backed by `website/docs-link-manifest.json`.

The dependency-light build now emits `website/build/index.html`, source-backed HTML pages for each Markdown navigation target, `website/build/docs-index.json`, `website/build/sitemap.xml`, and `website/build/robots.txt` using only Node.js standard library modules.

## Files changed

`docs/README.md`, `website/package.json`, `website/scripts/build.js`, `website/scripts/check-links.js`, `website/scripts/validate-quality.js`, `website/docs-link-manifest.json`, `website/src/index.md`, `conductor/tracks/14-docs-site-education/test-matrix.md`, `conductor/tracks/14-docs-site-education/validate-docs-site.ps1`, `conductor/tracks/14-docs-site-education/handoff.md`

## Contracts consumed

`conductor/workflow.md`, `conductor/contracts/`, `docs/`

## Contracts changed

None.

## Tests added

Build, repository-doc-tree, navigation-manifest, generated-index, generated-page-count, size-budget, binding-link, and docs-link-manifest checks are specified in `test-matrix.md`.

## Validation evidence

- `powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/14-docs-site-education/validate-docs-site.ps1`
- `npm --prefix website ci`
- `npm --prefix website run check:links`
- `npm --prefix website run build`
- `npm --prefix website run check:quality`
- `npm --prefix website run check:all`
- `node -e "const fs=require('fs'); const p=JSON.parse(fs.readFileSync('website/build/docs-index.json','utf8')); console.log(JSON.stringify({entries:p.entries.length, generatedPages:p.generatedPages.length, sample:p.entries.slice(0,3)}, null, 2));"` reported 23 entries and 23 generated pages.
- `node tests\conformance\track12_20_evidence_check.mjs`
- `Test-Path -LiteralPath 'website/build/index.html'`
- `Test-Path -LiteralPath 'website/build/docs-index.json'`
- `Test-Path -LiteralPath 'website/build/sitemap.xml'`
- `Test-Path -LiteralPath 'website/build/robots.txt'`

## Known risks

The site currently renders static HTML pages for the manifest navigation targets, but it does not yet render every Markdown file under `docs/`. The link checker validates source Markdown targets, required paths, and manifest navigation targets; generated HTML anchor validation remains a future enhancement.

## Integration notes

Use `just docs-build` for CI-style validation and `just docs-dev` for local preview.

## Review-hardening update

Added a track-local offline validator that checks the docs package scripts,
link manifest paths, site sources, generated navigation, quality outputs, and current binding/docs tree references. This keeps Track 14 evidence executable without changing central quality gates.
