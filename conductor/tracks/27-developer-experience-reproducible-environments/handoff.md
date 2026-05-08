# Handoff: Track 27 Developer Experience & Reproducible Environments

Last updated: 2026-05-08

## Summary

Documented and validated the contributor workflow commands for bootstrapping, building, previewing the docs site, and checking the reproducible toolchain manifest surfaces.

## Files changed

`justfile`, `scripts/bootstrap.sh`, `scripts/bootstrap.ps1`, `scripts/dx/validate-docs-workflow.mjs`, `scripts/dx/validate-toolchain-docs.mjs`, `docs/developer-experience/docs-workflow.md`, `conductor/tracks/27-developer-experience-reproducible-environments/test-matrix.md`, `conductor/tracks/27-developer-experience-reproducible-environments/risk-register.md`, `conductor/tracks/27-developer-experience-reproducible-environments/handoff.md`

## Contracts consumed

`conductor/workflow.md`, `conductor/contracts/`, `docs/`, `website/`

## Release gates affected

Docs build, preview, bootstrap-smoke, and toolchain-docs commands are now explicit contributors to the developer-experience gate.

## Current command contract

- `just docs-bootstrap` runs `npm --prefix website ci`.
- `just docs-build` runs `npm --prefix website ci` and `npm --prefix website run build`.
- `just docs-dev` runs `npm --prefix website ci` and `npm --prefix website start`, which serves `http://localhost:3000` by default.
- `just dev-setup` runs `rustup component add clippy rustfmt` plus the optional `cargo install cargo-nextest --locked` and `cargo install cargo-vet --locked` bootstrap steps.
- `pwsh -NoProfile -File scripts/bootstrap.ps1 -CheckOnly` verifies the Windows/PowerShell bootstrap prerequisites without installing anything.
- `just docs-smoke` and `just check-docs` run `node scripts/dx/validate-docs-workflow.mjs`.
- `just toolchain-docs` runs `node scripts/dx/validate-toolchain-docs.mjs`.
- The smoke validator runs `npm --prefix website run check:links`, `npm --prefix website run build`, verifies `website/build/index.html`, then starts the preview on `http://127.0.0.1:41727/` unless `DOCS_SMOKE_PORT` is set.

## Validation evidence

- `just --list` failed in this shell: `The term 'just' is not recognized as a name of a cmdlet, function, script file, or executable program.`
- `npm --prefix website ci` passed: audited 1 package, found 0 vulnerabilities.
- `npm --prefix website run check:links` passed: checked 25 required paths and 2 Markdown sources.
- `npm --prefix website run build` passed: built `website/build/index.html`.
- `$env:PORT='41727'; node website\scripts\dev.js` started the docs dev server at `http://localhost:41727`; `Invoke-WebRequest -UseBasicParsing -Uri http://127.0.0.1:41727/` returned HTTP 200.
- `node scripts/dx/validate-docs-workflow.mjs` passed: link check, build check, built HTML assertions, and local preview smoke at `http://127.0.0.1:41727/`.
- `pwsh -NoProfile -File scripts/bootstrap.ps1 -CheckOnly` passed and reported `just` absence as a warning.
- `node scripts/dx/validate-toolchain-docs.mjs` passed after checking `.devcontainer/devcontainer.json`, `devbox.json`, `mise.toml`, `justfile`, and bootstrap script references.
- `node scripts/validation/validate-track21-27-evidence-boundaries.mjs` passed.
- `node scripts/validation/validate-tracks21-27.mjs` passed with Track 27 docs workflow smoke green.
- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` passed after the shared ledger settled.
- `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` passed in non-strict mode.

## Risks and unresolved questions

The site preview is intentionally simple and does not yet serve a richer docs framework.

## Contracts changed

The root `justfile` command contract now includes `just dev-setup` alongside the docs bootstrap, build, dev, smoke, and check-docs recipes.

## Tests added

The current validation evidence is `npm --prefix website ci`, `npm --prefix website run check:links`, `npm --prefix website run build`, a local preview HTTP 200 check, and `node scripts/dx/validate-docs-workflow.mjs`.

## Known risks

`just` is not on PATH in this shell, so the recipes are documented and validator-mapped but not directly executable here.

## Follow-up issues

Install or provision `just` in this local shell, then rerun `just dev-setup`, `just docs-bootstrap`, `just docs-build`, `just docs-smoke`, and `just toolchain-docs` directly. The Windows fallback is `pwsh -NoProfile -File scripts/bootstrap.ps1`; the Unix-like bootstrap path now installs `just` before docs dependency bootstrap.

## Integration notes

Use the underlying npm, node, PowerShell, and bootstrap commands as the current fallback gates until `just` availability is present in this local shell.
## Phase closeout evidence

`$conductor-review` found no blocking Track 27 defects after the Unix bootstrap, docs-workflow, and toolchain-docs validation hardening. Accepted fixes: add the Track 27 toolchain-docs validator, wire `just toolchain-docs`, require the recipe from the docs-workflow validator, and align `scripts/bootstrap.sh` with the documented `just dev-validate` bootstrap path.

Validation commands run:

- `pwsh -NoProfile -File scripts/bootstrap.ps1 -CheckOnly`
- `node scripts/dx/validate-toolchain-docs.mjs`
- `node scripts/dx/validate-docs-workflow.mjs`
- `node scripts/validation/validate-track21-27-evidence-boundaries.mjs`
- `node scripts/validation/validate-tracks21-27.mjs`
- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`
- `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1`

Cleanup state: dirty shared worktree with Track 27 edits plus pre-existing unrelated registry/status edits from other workers. Commit SHA: blocked, no Track 27 commit created in this shared dirty worktree. Pushed ref: blocked, no push performed. Strict `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`: not run because the shared worktree is not clean. Next-phase decision: Track 27 is In Review; do not advance to Done until `just` is available or explicitly waived for direct recipe execution, a cleaned commit/push exists, and strict git closeout can run cleanly.
