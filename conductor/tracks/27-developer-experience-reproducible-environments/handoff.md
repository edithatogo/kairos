# Handoff: Track 27 Developer Experience & Reproducible Environments

## Summary

Documented the contributor workflow commands for bootstrapping, building, and previewing the docs site from the repo layout that already exists.

## Files changed

`justfile`, `scripts/dx/validate-docs-workflow.mjs`, `docs/developer-experience/docs-workflow.md`, `conductor/tracks/27-developer-experience-reproducible-environments/test-matrix.md`, `conductor/tracks/27-developer-experience-reproducible-environments/risk-register.md`, `conductor/tracks/27-developer-experience-reproducible-environments/handoff.md`

## Contracts consumed

`conductor/workflow.md`, `conductor/contracts/`, `docs/`, `website/`

## Release gates affected

Docs build and preview commands are now explicit contributors to the developer-experience gate.

## Current command contract

- `just docs-bootstrap` runs `npm --prefix website ci`.
- `just docs-build` runs `npm --prefix website ci` and `npm --prefix website run build`.
- `just docs-dev` runs `npm --prefix website ci` and `npm --prefix website start`, which serves `http://localhost:3000` by default.
- `just docs-smoke` and `just check-docs` run `node scripts/dx/validate-docs-workflow.mjs`.
- The smoke validator runs `npm --prefix website run check:links`, `npm --prefix website run build`, verifies `website/build/index.html`, then starts the preview on `http://127.0.0.1:41727/` unless `DOCS_SMOKE_PORT` is set.

## Validation evidence

- `just --list` failed in this shell: `The term 'just' is not recognized as a name of a cmdlet, function, script file, or executable program.`
- `npm --prefix website ci` passed: audited 1 package, found 0 vulnerabilities.
- `npm --prefix website run check:links` passed: checked 25 required paths and 2 Markdown sources.
- `npm --prefix website run build` passed: built `website/build/index.html`.
- `$env:PORT='41727'; node website\scripts\dev.js` started the docs dev server at `http://localhost:41727`; `Invoke-WebRequest -UseBasicParsing -Uri http://127.0.0.1:41727/` returned HTTP 200.
- `node scripts/dx/validate-docs-workflow.mjs` passed: link check, build check, built HTML assertions, and local preview smoke at `http://127.0.0.1:41727/`.

## Risks and unresolved questions

The site preview is intentionally simple and does not yet serve a richer docs framework.
