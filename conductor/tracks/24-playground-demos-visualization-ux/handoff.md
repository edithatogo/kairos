# Handoff: Track 24 Playground, Demos & Visualization UX

## Summary

Captured the demo and visualization requirements so the docs surface can stay honest about what the repo currently shows.
The Track 24 docs now point at a concrete playground page and concrete screenshot targets instead of generic demo language.

Worker 1 added the first concrete local playground slice: a static headless visualization snapshot page backed by a fixture tied to `examples/viz/headless-snapshot/src/main.rs`.

## Files changed

`website/playground/index.html`, `website/playground/main.js`, `website/playground/style.css`, `website/playground/headless-snapshot.json`, `website/scripts/smoke-playground.mjs`, `docs/playground/headless-snapshot.md`, `docs/community/playground.md`, `conductor/tracks/24-playground-demos-visualization-ux/plan.md`, `conductor/tracks/24-playground-demos-visualization-ux/test-matrix.md`, `conductor/tracks/24-playground-demos-visualization-ux/risk-register.md`, `conductor/tracks/24-playground-demos-visualization-ux/handoff.md`

## Contracts consumed

`website/`, `conductor/delivery-readiness-checklist.md`, `conductor/workflow.md`

## Release gates affected

Playground and visualization checks now sit on the docs and preview gate path through the docs home page and the community playground page.

## Local validation commands

- `node website/scripts/smoke-playground.mjs` — pass, `Playground smoke passed for examples/viz/headless-snapshot with 2 entities at tick 12.`
- `cargo check --manifest-path examples/viz/headless-snapshot/Cargo.toml` — pass, example type-checks.
- `cargo run --manifest-path examples/viz/headless-snapshot/Cargo.toml` — fail on local Windows linker configuration. Git `usr/bin/link.exe` fails with `couldn't create signal pipe, Win32 error 5`; after removing that PATH entry, MSVC `link.exe` is not found.
- `npm --prefix website run check:links` — pass, `Checked 25 required paths and 2 markdown sources.`
- `npm --prefix website run build` — pass, built `website/build/index.html`.

## Risks and unresolved questions

The main risk is presenting a demo as a product surface before the underlying assets or interactions exist.
The current screenshot paths are targets, so a later worker still needs to check in actual images before the page can claim visual completeness.

The implemented slice is intentionally fixture-backed. It should not be described as live Wasm execution until the TypeScript/Wasm track hands over that contract.

Local executable verification remains blocked until the Windows MSVC linker is available on PATH. Use `cargo check --manifest-path examples/viz/headless-snapshot/Cargo.toml` as the current non-linking Rust gate.
