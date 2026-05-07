# Test Matrix — 14 Documentation Site & Education

## Required tests

- `npm ci` in `website`
- `npm run build` in `website`
- `npm run check:links` in `website`
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
