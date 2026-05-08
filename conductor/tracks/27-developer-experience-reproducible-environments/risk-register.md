# Risk Register: Track 27 Developer Experience & Reproducible Environments

Severity scale: Likelihood 1-5 x Impact 1-5. Low 1-4, Medium 5-9, High 10-16, Critical 17-25.

| Risk | Likelihood | Impact | Severity | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| Dev environment not reproducible across operating systems | 4 | 4 | 16 | Nix flake or devcontainer definition with locked inputs; CI validates fresh checkout builds on all platforms | devx-agent | Fresh checkout fails `cargo build` on any supported OS |
| Container/devcontainer drift from CI environment | 3 | 4 | 12 | Use same base image in devcontainer and CI; scheduled rebuild to detect drift | ci-agent | Devcontainer and CI diverge by >1 dependency version |
| Toolchain version conflicts in dev environment | 3 | 4 | 12 | Lock all toolchains in `rust-toolchain.toml` + devShell; pin wasm-pack, wasm-bindgen, and binding toolchains | devx-agent | Toolchain version mismatch blocks developer build |
| Onboarding time >30 minutes to first `cargo build` | 4 | 3 | 12 | One-command setup script; benchmark fresh-clone-to-build time in CI | devx-agent | Fresh-clone-to-build exceeds 30min benchmark |
| Dependency cache invalidation on every checkout | 3 | 3 | 9 | Document `sccache` and `cargo-chef` usage; verify cache hits in CI | ci-agent | Cache miss rate >20% on incremental builds |
| Docs bootstrap/build/dev commands drift from `website/` layout | 3 | 4 | 12 | `just docs-smoke` validates `justfile` recipes, `website/package.json` scripts, link manifest, static build output, and preview HTTP response | dx-agent | `just docs-smoke` or `just check-docs` fails on a clean checkout |
| Docs preview port conflicts on contributor machines | 2 | 3 | 6 | `docs-dev` uses the website default `PORT` override; smoke validator uses `DOCS_SMOKE_PORT` and defaults to 41727 to avoid the normal dev port | dx-agent | Preview cannot start after setting `PORT` or `DOCS_SMOKE_PORT` |
| Contributor cannot run `just` recipes because `just` is missing from `PATH` | 3 | 3 | 9 | Keep underlying npm/node commands documented, include `just --version` in `just dev-validate`, provide `scripts/bootstrap.ps1` to install or check the Windows path, and keep `scripts/bootstrap.sh` installing `just` for Unix-like paths. | dx-agent | `just --list`, `just docs-bootstrap`, or `just check-docs` is not recognized after running the relevant bootstrap path |
