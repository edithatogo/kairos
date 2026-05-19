# 45 Astro/Starlight Docs Platform and Polyglot Experience - spec.md

## Mission

Keep the active Astro/Starlight documentation platform release-ready, versioned,
polyglot-aware, and validated as a first-class product surface.

Track 45 owns the docs-platform hardening slice that follows Track 41. Track 41
proved learning coverage and workflow parity; Track 45 makes the Starlight stack
itself a gated surface with explicit plugin expectations and release evidence.

## Primary subagent

```text
docs-platform-agent + docs-agent + ci-agent
```

## Dependencies

```text
Tracks 14, 17, 27, 41, and 44.
```

## Owned paths

```text
.github/workflows/docs-quality.yml
docs/developer-experience/docs-platform.md
website/
scripts/validation/validate-docs-platform-sota.mjs
conductor/tracks/45-docs-platform-starlight-sota/*
```

## Acceptance criteria

- Astro and Starlight are documented as the active docs shell.
- `starlight-versions` provides the current/release archive contract.
- `kairoecs-starlight-polyglot` remains wired and validates all supported binding language entry points.
- SOTA documentation helpers are explicit: link validation, llms.txt output, icons, generated search output, and archive route proof.
- Deferred plugin candidates such as TypeDoc, OpenAPI, and hosted DocSearch are recorded with activation conditions.
- CI runs the docs-platform SOTA validator alongside the existing docs workflow and learning coverage gates.

## Release implications

Track 45 gates public claims that the documentation platform is Astro/Starlight-based, versioned, and polyglot-ready. It does not gate package publication by itself, but Track 44 may consume its validator as docs-health evidence.

## Blocked paths

Do not enable generated API-reference plugins, OpenAPI plugins, or hosted search without source-of-truth API artifacts and a privacy/cost decision. Do not claim a docs release line is archived unless the Starlight version route and generated output are present.
