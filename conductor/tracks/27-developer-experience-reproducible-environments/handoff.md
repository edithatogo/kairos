# Handoff: Track 27 Developer Experience & Reproducible Environments

Last updated: 2026-05-07

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
- `just dev-setup` runs `rustup component add clippy rustfmt` plus the optional `cargo install cargo-nextest --locked` and `cargo install cargo-vet --locked` bootstrap steps.
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

## Contracts changed

The root `justfile` command contract now includes `just dev-setup` alongside the docs bootstrap, build, dev, smoke, and check-docs recipes.

## Tests added

The current validation evidence is `npm --prefix website ci`, `npm --prefix website run check:links`, `npm --prefix website run build`, a local preview HTTP 200 check, and `node scripts/dx/validate-docs-workflow.mjs`.

## Known risks

`just` is not on PATH in this shell, so the recipes are documented and validator-mapped but not directly executable here.

## Follow-up issues

Install or provision `just` in the developer environment/devcontainer path, then rerun `just dev-setup`, `just docs-bootstrap`, `just docs-build`, and `just docs-smoke` directly.

## Integration notes

Use the underlying npm and node commands as the current fallback gate until `just` availability is part of the reproducible environment.
## Phase closeout evidence

Pending for the next actual phase closeout. Before this track advances, record `$conductor-review` findings, accepted fixes, deferred or blocked fixes, validation commands, cleanup state, commit SHA or explicit push blocker, and next-phase decision here.