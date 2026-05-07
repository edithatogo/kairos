# Docs Workflow

The repository docs site lives in `website/` and renders `website/src/index.md` into `website/build/index.html`. Contributor-facing commands are exposed through `justfile` so the same commands work from the repo root.

## Commands

- `just dev-validate` checks the toolchain versions that a fresh contributor needs before building.
- `just docs-bootstrap` installs the docs site dependencies with `npm --prefix website ci`.
- `just docs-build` installs dependencies and runs `npm --prefix website run build`.
- `just docs-dev` installs dependencies and starts the local preview with `npm --prefix website start`.
- `just docs-smoke` runs the local smoke validator for the docs command surface, link manifest, build output, and preview server.
- `just check-docs` is the same validator used as the focused docs quality gate.

## Expected Output

- `just dev-validate` prints the installed toolchain versions and confirms the expected commands are on `PATH`.
- `just docs-build` must produce `website/build/index.html`.
- `just docs-dev` must serve the generated site at `http://localhost:3000` unless `PORT` is set.
- `just docs-smoke` starts the preview on `http://127.0.0.1:41727/` by default and verifies the rendered page includes the docs title and contributor commands.

## Layout Contract

- `website/package.json` owns the docs npm scripts.
- `website/scripts/build.js` owns the static HTML generation.
- `website/scripts/dev.js` owns the local preview server.
- `website/docs-link-manifest.json` lists required repo paths and Markdown sources checked by `npm --prefix website run check:links`.
- `scripts/dx/validate-docs-workflow.mjs` owns the repo-root smoke validation used by `just docs-smoke` and `just check-docs`.
