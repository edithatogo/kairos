# Docs Platform

KairoECS now uses Astro and Starlight as the active documentation shell under
`website/`.

The Astro/Starlight site is the current docs platform. The previous custom Node
static-site scaffold has been retired; the remaining custom Node scripts are
validation helpers around the Starlight build, link manifest, SOTA plugin stack,
and quality checks.

The live site parity boundary is now the Starlight output plus the repository
docs, bindings, examples, and conductor source trees linked from that shell.

## Active site

- `npm --prefix website run build` runs `astro build`.
- `npm --prefix website run postbuild` generates legacy compatibility routes,
  `docs-index.json`, `robots.txt`, and `sitemap.xml` after the Starlight build.
- `npm --prefix website run dev` runs `astro dev`.
- `npm --prefix website run check:links` still validates the repository docs
  tree and `website/docs-link-manifest.json`.
- `npm --prefix website run check:quality` verifies the Starlight build output,
  configured plugin stack, versioned route, and polyglot content entry points.
- `npm --prefix website run check:sota` verifies the versioning, polyglot,
  llms.txt, icon, generated-search, and archive-route contract.
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

Deferred plugins are explicit: `starlight-typedoc` waits until generated
TypeScript API reference is source-of-truth, `starlight-openapi` waits until an
OpenAPI contract exists, and hosted Algolia DocSearch waits until Pagefind is
insufficient for the public docs scale.

## Source of truth

The Starlight content collection provides the public website shell and primary
entry points. The canonical detailed documentation remains in the repository
trees:

- `docs/`
- `bindings/`
- `examples/`
- `conductor/`

The website links back to those sources where the detailed page is still owned
outside the Starlight content collection. The post-build compatibility layer
also writes legacy `/kairos/docs/*.html`, `/kairos/bindings/*.html`, and related
manifest-backed pages that redirect readers to the canonical source documents.

## Versioning

The current docs line is `R2 Preview`. The committed `r1/` content route and
`website/src/content/versions/r1.json` archive config exercise the versioning
plugin in the normal build.

## GitHub Pages Base

The project is published as a GitHub Pages project site under `/kairos/`.
`website/astro.config.mjs`, `website/docs-link-manifest.json`, and the
post-build compatibility generator all use that project-site base. Redirecting
traffic from the user-site root `/` is outside this repository's Pages build
scope; compatibility routes are generated under the project base.

## Local validation

```powershell
npm --prefix website run check:all
npm --prefix website run check:sota
node scripts/dx/validate-docs-workflow.mjs
```
