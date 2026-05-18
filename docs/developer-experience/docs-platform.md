# Docs Platform

KairoECS now uses Astro and Starlight as the active documentation shell under
`website/`.

The roadmap target is the Astro/Starlight site. The previous custom Node
static-site scaffold has been retired; the remaining custom Node scripts are
validation helpers around the Starlight build, link manifest, and quality
checks.

## Active site

- `npm --prefix website run build` runs `astro build`.
- `npm --prefix website run dev` runs `astro dev`.
- `npm --prefix website run check:links` still validates the repository docs
  tree and `website/docs-link-manifest.json`.
- `npm --prefix website run check:quality` verifies the Starlight build output,
  configured plugin stack, versioned route, and polyglot content entry points.
- `npm --prefix website run check:all` wraps link validation, Starlight build,
  and quality validation.

## Plugin stack

The Starlight configuration uses:

- `starlight-versions` for the `R2 Preview` / `R1 Archive` version switcher.
- `starlight-links-validator` for Starlight-aware internal link validation.
- `starlight-llms-txt` for `llms.txt`, `llms-full.txt`, and
  `llms-small.txt`.
- `starlight-plugin-icons` for richer Starlight icon affordances.
- `kairoecs-starlight-polyglot`, the local polyglot plugin that marks the
  supported binding languages in the generated site metadata.

## Source of truth

The Starlight content collection provides the public website shell and primary
entry points. The canonical detailed documentation remains in the repository
trees:

- `docs/`
- `bindings/`
- `examples/`
- `conductor/`

The website links back to those sources where the detailed page is still owned
outside the Starlight content collection.

## Versioning

The current docs line is `R2 Preview`. The committed `r1/` content route and
`website/src/content/versions/r1.json` archive config exercise the versioning
plugin in the normal build.

## Local validation

```powershell
npm --prefix website run check:all
node scripts/dx/validate-docs-workflow.mjs
```
