# Risk Register — 14 Documentation Site & Education

| Risk | L | I | Sev | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| Docs become stale as implementation changes | 4 | 3 | 12 | Add link-check and content-grep CI; flag stale tutorial references before release | docs-agent | Link-check CI fails |
| Missing API docs for new features | 4 | 3 | 12 | Require docs update in the same PR as new public API | docs-agent | Public API merges without doc update |
| Tutorial drift: examples no longer compile | 3 | 4 | 12 | Run tutorial code in CI as smoke tests; fail build on non-compiling examples | docs-agent | Tutorial smoke test fails |
| Search/discoverability poor | 2 | 3 | 6 | Generate sitemap and `docs-index.json` from the link manifest; test search for top 10 user queries before beta | docs-agent | Top-10 query test fails |
| Accessibility non-compliance | 2 | 4 | 8 | Run automated aXe/WCAG checks on docs site in CI; target WCAG 2.1 AA | docs-agent | WCAG 2.1 AA automated check fails |
| Docs site build or preview pipeline breaks | 3 | 3 | 9 | Test docs-build and docs-dev in CI per PR; pin website dependency versions | docs-agent | Docs site build fails |
