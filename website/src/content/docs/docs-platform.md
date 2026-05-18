---
title: Docs Platform
description: Astro Starlight documentation platform setup and plugin policy.
---

The documentation site is implemented with Astro and Starlight.

## Active stack

- Astro site build and dev server.
- `@astrojs/starlight` for docs routing, search, navigation, and page chrome.
- `starlight-versions` for versioned documentation navigation.
- `starlight-links-validator` for Starlight-aware link validation.
- `starlight-llms-txt` for `llms.txt`, `llms-full.txt`, and focused LLM
  ingestion surfaces.
- `starlight-plugin-icons` for richer sidebar/code-block icon affordances.
- `kairoecs-starlight-polyglot`, a local plugin that marks the language binding
  surface and keeps the polyglot scope visible to downstream automation.

## Versioning

The current documentation line is `R2 Preview`. The archived `R1 Archive`
content exists under the `r1/` route so version navigation is exercised in the
same build as the current docs.

## Source of truth

The Starlight shell does not replace the repository-level docs tree. It provides
the public site structure while the canonical technical content continues to live
under `docs/`, `bindings/`, `examples/`, and `conductor/`.
