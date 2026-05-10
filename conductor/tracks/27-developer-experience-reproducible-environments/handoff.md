# Handoff: Track 27 Developer Experience & Reproducible Environments

Last updated: 2026-05-10

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
- `scoop install just julia` installed `just` 1.50.0 and Julia 1.12.2 on 2026-05-09.
- `just --version` returned `just 1.50.0`.
- `just --list` passed and listed the expected repo recipes, including `docs-smoke`, `toolchain-docs`, `dev-setup`, `dev-validate`, `validate-conductor`, and `validate-tracks21-27`.
- `npm --prefix website ci` passed: audited 1 package, found 0 vulnerabilities.
- `npm --prefix website run check:links` passed: checked 25 required paths and 2 Markdown sources.
- `npm --prefix website run build` passed: built `website/build/index.html`.
- `$env:PORT='41727'; node website\scripts\dev.js` started the docs dev server at `http://localhost:41727`; `Invoke-WebRequest -UseBasicParsing -Uri http://127.0.0.1:41727/` returned HTTP 200.
- `node scripts/dx/validate-docs-workflow.mjs` passed: link check, build check, built HTML assertions, and local preview smoke at `http://127.0.0.1:41727/`.
- `pwsh -NoProfile -File scripts/bootstrap.ps1 -CheckOnly` passed and reported `just` absence as a warning.
- `node scripts/dx/validate-toolchain-docs.mjs` passed after checking `.devcontainer/devcontainer.json`, `devbox.json`, `mise.toml`, `justfile`, and bootstrap script references.
- 2026-05-09 rerun initially exposed drift in `scripts/bootstrap.sh`: the validator required the literal Unix `for tool in just` install guard while the script only installed `just` through the versioned `spec` loop.
- `scripts/bootstrap.sh` now has an explicit `for tool in just; do cargo install "$tool" --locked || true; done` guard before the version-pinned cargo tool loop.
- `node scripts/dx/validate-toolchain-docs.mjs` passed after the Unix bootstrap guard fix.
- `node scripts/dx/validate-docs-workflow.mjs` passed after rerun; an earlier parallel run hit a transient `127.0.0.1:41727` port collision while another docs smoke was active.
- `just toolchain-docs` passed outside the sandbox on 2026-05-09. The sandboxed run failed in Git Bash before invoking Node with `couldn't create signal pipe, Win32 error 5`.
- `just docs-smoke` passed outside the sandbox on 2026-05-09. The sandboxed run failed in Git Bash before invoking Node with `CreateFileMapping ... Win32 error 5`.
- `just docs-build` passed outside the sandbox on 2026-05-09: `npm --prefix website ci` found 0 vulnerabilities and the docs build rendered 110 pages, 100 search-index entries, 23 crates, and 459 public API items.
- `just validate-conductor` passed outside the sandbox on 2026-05-09, including `scripts/validate_conductor_setup.ps1`, phase gates, git closeout, and the Rust workspace test suite invoked by the setup validator.
- 2026-05-10 follow-up changed `just dev-setup` to call `scripts/bootstrap.ps1 -SkipPython -SkipNpm`, so Windows setup no longer runs raw `cargo install` from Git Bash.
- `scripts/bootstrap.ps1` now prefers `rustup run stable-x86_64-pc-windows-gnu cargo install` for optional Rust tools when that toolchain is installed, avoiding Git's `usr\bin\link.exe` shadowing the MSVC linker.
- `just dev-setup` passed outside the sandbox on 2026-05-10 after installing or confirming `cargo-nextest`, `cargo-vet`, `cargo-deny`, `cargo-audit`, and `cargo-llvm-cov`.
- `just dev-validate` passed outside the sandbox on 2026-05-10, confirming Rust/Cargo, `cargo nextest`, `cargo vet`, Python, Node, Go, Julia, R, .NET, and `just` are visible.
- `node scripts/validation/validate-track21-27-evidence-boundaries.mjs` passed.
- `node scripts/validation/validate-tracks21-27.mjs` passed with Track 27 docs workflow smoke green.
- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` passed after the shared ledger settled.
- `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` passed in non-strict mode.

## Risks and unresolved questions

The site preview is intentionally simple and does not yet serve a richer docs framework.

## Contracts changed

The root `justfile` command contract now includes `just dev-setup` alongside the docs bootstrap, build, dev, smoke, and check-docs recipes.

## Tests added

The current validation evidence is `npm --prefix website ci`, `npm --prefix website run check:links`, `npm --prefix website run build`, a local preview HTTP 200 check, `node scripts/dx/validate-docs-workflow.mjs`, `node scripts/dx/validate-toolchain-docs.mjs`, `just toolchain-docs`, `just docs-smoke`, `just docs-build`, `just validate-conductor`, `just docs-bootstrap`, `just validate-tracks21-27`, `just dev-setup`, and `just dev-validate`.

## Known risks

`just` is now available through the Scoop shim. Direct recipe execution works outside the sandbox; inside the sandbox, Git Bash can fail before invoking the recipe body with Win32 access-denied errors.

## Follow-up issues

No direct Track 27 bootstrap/doc recipe remains blocked by missing local tools. Keep running direct `just` recipes outside the sandbox on this host when Git Bash needs process primitives that the sandbox denies.

## Integration notes

Use the underlying npm, node, PowerShell, and bootstrap commands as the current fallback gates until `just` availability is present in this local shell.
## Phase closeout evidence

`$conductor-review` found no blocking Track 27 defects after the Unix bootstrap, docs-workflow, and toolchain-docs validation hardening. Accepted fixes: add the Track 27 toolchain-docs validator, wire `just toolchain-docs`, require the recipe from the docs-workflow validator, and align `scripts/bootstrap.sh` with the documented `just dev-validate` bootstrap path. The 2026-05-09 follow-up fixed the remaining Unix bootstrap drift by making the `just` install guard explicit enough for `validate-toolchain-docs.mjs`, installed the missing Scoop `just` shim, and proved `just toolchain-docs` plus `just docs-smoke` outside the sandbox. The 2026-05-10 follow-up moved `just dev-setup` through the Windows bootstrap script and proved the GNU cargo install path plus `just dev-validate`.

Validation commands run:

- `pwsh -NoProfile -File scripts/bootstrap.ps1 -CheckOnly`
- `node scripts/dx/validate-toolchain-docs.mjs`
- `node scripts/dx/validate-docs-workflow.mjs`
- `node scripts/validation/validate-track21-27-evidence-boundaries.mjs`
- `node scripts/validation/validate-tracks21-27.mjs`
- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`
- `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1`

Cleanup state: dirty shared worktree with Track 27 edits plus pre-existing unrelated registry/status edits from other workers. Commit SHA: blocked, no Track 27 commit created in this shared dirty worktree. Pushed ref: blocked, no push performed. Strict `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`: not run because the shared worktree is not clean. Next-phase decision: Track 27 is In Review with direct `just` recipe execution now available outside the sandbox and local developer tooling validated. Do not advance to Done until a cleaned commit/push exists and strict git closeout can run cleanly.
