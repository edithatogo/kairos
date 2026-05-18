---
title: KairoECS Documentation
description: Deterministic simulation engine documentation for core runtime, evidence, and polyglot bindings.
---

KairoECS is a Rust-first simulation engine for deterministic event scheduling,
ECS-style state, DES, ABM, Arrow telemetry, and polyglot bindings.

This Starlight site is now the active documentation shell for the repository. It
keeps the existing repository documentation tree as the source of truth while
surfacing the highest-signal entry points for contributors, release managers,
and binding users.

## Start here

- [Architecture](architecture.md)
- [Docs platform](docs-platform.md)
- [Conductor status](evidence/conductor-status.md)
- [Rust quickstart](polyglot/rust.md)
- [Python quickstart](polyglot/python.md)
- [PDES and distributed evidence](evidence/pdes-distributed.md)

## Source documentation

The full source documentation remains in the repository-level `docs/`,
`bindings/`, `examples/`, and `conductor/` trees. The Starlight shell links back
to those canonical files where a page is still maintained outside the website
content collection.

## Local workflow

```powershell
npm --prefix website run check:all
npm --prefix website run build
npm --prefix website run dev
just docs-build
just docs-dev
```
