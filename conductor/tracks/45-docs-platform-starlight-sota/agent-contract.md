# 45 Astro/Starlight Docs Platform and Polyglot Experience - agent-contract.md

## Owning agents

```text
docs-platform-agent + docs-agent + ci-agent
```

## Must produce

- A documented Astro/Starlight active-platform contract.
- Versioning evidence through `starlight-versions`.
- Polyglot docs evidence through `kairoecs-starlight-polyglot`.
- SOTA helper evidence for link checks, llms.txt, icons, generated search, and archived release routes.
- CI wiring for the docs-platform SOTA validator.

## May change

- `website/` documentation-platform code, content, config, and package metadata.
- Docs-platform narrative under `docs/developer-experience/`.
- Conductor Track 45 artifacts and registry/status surfaces.
- Docs-quality workflow steps that validate the same owned surface.

## Must not change without handoff

- Package publication workflows.
- Binding API contracts.
- Hosted search, analytics, or third-party docs services that require credentials, account setup, privacy review, or cost approval.
- Generated API-reference plugins unless the generated source artifact is authoritative.

## Coordination notes

Track 45 consumes Track 41 learning-coverage evidence and Track 44 health-gate expectations. It should stay focused on docs-platform publication quality rather than reopening all docs content scope.
