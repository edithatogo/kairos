# 45 Astro/Starlight Docs Platform and Polyglot Experience - risk-register.md

| Risk | Likelihood | Impact | Severity | Mitigation | Owner | Escalation trigger |
|---|---:|---:|---:|---|---|---|
| Plugin drift breaks Starlight builds or generated routes | 3 | 4 | 12 | Keep `npm --prefix website run check:all` and `check:sota` in CI. | docs-platform-agent | Docs build fails after dependency update. |
| Versioned archive route exists in config but not generated output | 2 | 4 | 8 | Validate `website/build/r1/index.html` after every build. | ci-agent | Missing archive route in generated site. |
| Polyglot docs claim a supported binding without a page | 3 | 4 | 12 | Validator checks each supported language content page. | docs-agent | New binding added without docs page. |
| Enabling API plugins before source artifacts exist creates stale docs | 2 | 5 | 10 | Keep TypeDoc/OpenAPI deferred until API references are source-of-truth. | docs-agent | Plugin PR lacks generated-source validation. |
| Search or llms.txt generated artifacts silently disappear | 2 | 3 | 6 | Validator checks Pagefind and llms.txt outputs. | docs-platform-agent | Generated build output misses search or llms files. |

## Severity scale

Low 1-4, Medium 5-9, High 10-15, Critical 16-25.
