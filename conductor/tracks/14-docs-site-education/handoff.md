# Handoff — 14 Documentation Site & Education

## Summary

Documented the docs site as a static `website/` build, aligned the site home with the repository docs tree, and documented the `npm ci`, `npm run build`, `npm run check:links`, `npm run check:quality`, `npm run check:all`, and `npm run dev` commands that implement the site validation flow.

This R2 docs slice now reflects the implemented crate and binding surfaces, including the preview/native-not-configured binding status, and adds a manifest-driven docs navigation and quality gate backed by `website/docs-link-manifest.json`.

The dependency-light build now emits `website/build/index.html`, source-backed HTML pages for each Markdown navigation target, `website/build/docs-index.json`, `website/build/sitemap.xml`, and `website/build/robots.txt` using only Node.js standard library modules. The Markdown link checker now also validates same-file and cross-file fragment anchors against the rendered heading-id contract for source Markdown pages.

## Files changed

`docs/README.md`, `website/package.json`, `website/scripts/build.js`, `website/scripts/check-links.js`, `website/scripts/validate-quality.js`, `website/docs-link-manifest.json`, `website/src/index.md`, `notebooks/python_scheduler_tutorial.ipynb`, `conductor/tracks/14-docs-site-education/test-matrix.md`, `conductor/tracks/14-docs-site-education/validate-docs-site.ps1`, `conductor/tracks/14-docs-site-education/handoff.md`

## Contracts consumed

`conductor/workflow.md`, `conductor/contracts/`, `docs/`

## Contracts changed

None.

## Tests added

Build, repository-doc-tree, navigation-manifest, generated-index, generated-page-count, size-budget, binding-link, fragment-anchor, and docs-link-manifest checks are specified in `test-matrix.md`.

## Validation evidence

- `powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/14-docs-site-education/validate-docs-site.ps1`
- `npm --prefix website ci`
- `npm --prefix website run check:links`
- `node website/scripts/check-links.js --self-test`
- `npm --prefix website run build`
- `npm --prefix website run check:quality`
- `npm --prefix website run check:all`
- `node -e "const fs=require('fs'); const p=JSON.parse(fs.readFileSync('website/build/docs-index.json','utf8')); console.log(JSON.stringify({entries:p.entries.length, generatedPages:p.generatedPages.length, sample:p.entries.slice(0,3)}, null, 2));"` reported 79 entries and 79 generated manifest-backed pages.
- `node tests\conformance\track12_20_evidence_check.mjs`
- `node docs\assets\validate-playground-figures.mjs`
- `python notebooks\validate_notebooks.py`
- `Test-Path -LiteralPath 'website/build/index.html'`
- `Test-Path -LiteralPath 'website/build/docs-index.json'`
- `Test-Path -LiteralPath 'website/build/sitemap.xml'`
- `Test-Path -LiteralPath 'website/build/robots.txt'`

The 2026-05-07 validation pass also confirmed `npm --prefix website run check:all` rendered 105 total doc pages, wrote 95 search-index entries, indexed 23 crates / 457 public API items, and passed the docs quality gate.

The notebook validator initially found a stale assertion in `notebooks/python_scheduler_tutorial.ipynb`; the tutorial now expects `scheduled_events` and `cancelled_events` in the Python scheduler stats contract, matching the current binding tests.

## Known risks

The site currently renders static HTML pages for the manifest navigation targets, but it does not yet render every Markdown file under `docs/`. The link checker validates source Markdown targets, source Markdown fragment anchors, required paths, and manifest navigation targets; full generated HTML fragment crawling remains a future enhancement.

## Integration notes

Use `npm --prefix website run check:all` for CI-style validation and `npm --prefix website run dev` for local preview. The track-local `validate-docs-site.ps1` wrapper checks the package scripts, manifest paths, site sources, and generated quality outputs without touching website scripts or manifests.

## Review-hardening update

Added a track-local offline validator that checks the docs package scripts,
link manifest paths, site sources, generated navigation, quality outputs, and current binding/docs tree references. This keeps Track 14 evidence executable without changing central quality gates.

## Follow-up issues

Full generated HTML fragment crawling remains the main follow-up; current evidence covers source Markdown targets, source Markdown heading anchors, required paths, manifest navigation targets, and generated docs outputs.
## Phase closeout evidence

Pending for the next actual phase closeout. Before this track advances, record `$conductor-review` findings, accepted fixes, deferred or blocked fixes, validation commands, cleanup state, commit SHA or explicit push blocker, pushed ref, strict `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` result, and next-phase decision here.
