# Test Matrix — 14 Documentation Site & Education

## Required tests

- `npm ci` in `website`
- `npm run build` in `website`
- `npm run check:links` in `website`
- `node website/scripts/check-links.js --self-test`
- `npm run check:quality` in `website`
- `npm run check:all` in `website`
- `node docs/assets/validate-playground-figures.mjs`
- `powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/14-docs-site-education/validate-docs-site.ps1`
- `node tests/conformance/track12_20_evidence_check.mjs`
- `python notebooks\validate_notebooks.py`
- Generated `website/build/index.html` exists.
- Generated `website/build/docs-index.json`, `website/build/sitemap.xml`, and `website/build/robots.txt` exist.
- Generated source-backed HTML pages exist for manifest navigation Markdown targets, with at least 20 pages recorded in `website/build/docs-index.json`.
- The site home mentions the current repository docs tree.
- The site home includes tutorial/example, notebook/figure, quickstart, release, compatibility, citation, and quality-gate concepts.
- The docs link manifest covers implemented crate, binding, docs, and examples entry points.
- The docs link manifest drives navigation sections and quality expectations without external dependencies.
- Markdown fragment links resolve to rendered heading anchors for same-file and cross-file source links.
- The notebook tutorial surface includes runnable Python scheduler and reproducible benchmark/scenario notebooks with local figures only.
- Playground figures include committed SVG assets, non-empty alt text, and explicit source notes.
- Docs source files remain under `docs/` and `website/`.

The aggregate website validation flow is `npm run check:all`; the track-local validator (`validate-docs-site.ps1`) checks the same script and manifest contract surfaces without editing website scripts or manifests.

## CI commands

```bash
cd website
npm ci
npm run check:links
npm run build
npm run check:quality
npm run check:all
test -f build/index.html
test -f build/docs-index.json
test -f build/sitemap.xml
test -f build/robots.txt
rg -n "docs/README|docs/adr|docs/community|docs/release|docs/research|docs/benchmarks|docs/trustworthy-simulation|docs/design|docs/interoperability|bindings/python|bindings/r|bindings/julia|bindings/typescript|bindings/csharp|bindings/go|Tutorials and Examples|Jupyter notebooks and figures|check:quality" src/index.md
cd ..
node website/scripts/check-links.js --self-test
node docs/assets/validate-playground-figures.mjs
powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/14-docs-site-education/validate-docs-site.ps1
node tests/conformance/track12_20_evidence_check.mjs
python notebooks\validate_notebooks.py
```

## Local evidence

- 2026-05-06: `powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/14-docs-site-education/validate-docs-site.ps1`
- 2026-05-06: `npm --prefix website ci`
- 2026-05-06: `npm --prefix website run check:links`
- 2026-05-06: `npm --prefix website run build`
- 2026-05-06: `npm --prefix website run check:quality`
- 2026-05-06: `npm --prefix website run check:all`
- 2026-05-06: `node -e "const fs=require('fs'); const p=JSON.parse(fs.readFileSync('website/build/docs-index.json','utf8')); console.log(JSON.stringify({entries:p.entries.length, generatedPages:p.generatedPages.length, sample:p.entries.slice(0,3)}, null, 2));"` reported 23 entries and 23 generated pages.
- 2026-05-06: `node docs/assets/validate-playground-figures.mjs`
- 2026-05-06: `Test-Path -LiteralPath 'website/build/index.html'`
- 2026-05-06: `node tests\conformance\track12_20_evidence_check.mjs`
- 2026-05-06: `python notebooks\validate_notebooks.py`
- 2026-05-07: `npm --prefix website ci` passed.
- 2026-05-07: `npm --prefix website run check:all` passed; build rendered 105 doc pages, wrote 95 search-index entries, and indexed 23 crates / 457 public API items.
- 2026-05-07: `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\14-docs-site-education\validate-docs-site.ps1` passed with 100 required paths, 3 site sources, 6 navigation sections, and a 20-page minimum gate.
- 2026-05-07: `node docs\assets\validate-playground-figures.mjs` passed for 5 playground figure assets.
- 2026-05-07: `node tests\conformance\track12_20_evidence_check.mjs` passed.
- 2026-05-07: `python notebooks\validate_notebooks.py` passed for 2 notebooks after updating `notebooks/python_scheduler_tutorial.ipynb` to match the current Python scheduler stats contract.
- 2026-05-07: build output checks passed for `website/build/index.html`, `website/build/docs-index.json`, `website/build/sitemap.xml`, and `website/build/robots.txt`.
- 2026-05-07: `website/build/docs-index.json` reported 79 entries and 79 generated manifest-backed pages.
- 2026-05-08: `node website/scripts/check-links.js --self-test` passed for same-file and cross-file Markdown fragment anchor validation.
- 2026-05-08: `npm --prefix website ci` passed with 0 vulnerabilities.
- 2026-05-08: `node website/scripts/check-links.js --self-test` passed for Markdown fragment anchor validation and dependency-directory exclusion.
- 2026-05-08: `npm --prefix website run check:links` passed with 100 required paths, 3 markdown sources, and 6 navigation sections.
- 2026-05-08: `npm --prefix website run check:all` passed after the status/handoff update; build rendered 110 doc pages, wrote 100 search-index entries, indexed 23 crates / 459 public API items, and passed the docs quality gate.
- 2026-05-08: `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\14-docs-site-education\validate-docs-site.ps1` passed with 100 required paths, 3 site sources, 6 navigation sections, and a 20-page minimum gate.
- 2026-05-08: `node docs\assets\validate-playground-figures.mjs` passed for 5 playground figure assets and docs references.
- 2026-05-08: `node tests\conformance\track12_20_evidence_check.mjs` passed for Tracks 12-20 evidence coverage.
- 2026-05-08: `python notebooks\validate_notebooks.py` passed for 2 notebooks.
- 2026-05-08: `just docs-build` blocked locally because `just` is not installed on PATH. The underlying `npm --prefix website run check:all` docs build/link/quality gate passed.
- 2026-05-08: `pwsh -NoProfile -File scripts\validate_conductor_phase_gates.ps1` remained blocked by unrelated Track 19 handoff evidence outside Track 14 ownership: missing `commit SHA`, `pushed ref`, and `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` markers.
## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, phase-closeout ledger evidence, cleaned commit/push evidence, and blocker recording. At actual closeout, run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.
