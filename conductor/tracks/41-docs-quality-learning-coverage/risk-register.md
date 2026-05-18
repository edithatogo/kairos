# 41 Documentation Platform, Quality Gates & Learning Coverage -- risk-register.md

| Risk | Likelihood | Impact | Mitigation | Owner | Evidence | Trigger |
|---|---:|---:|---|---|---|---|
| Strict CI changes turn intended skip cases into noisy failures | 3 | 4 | Keep fork/no-secret and environment-gated exceptions explicit in the workflow and docs. | ci-agent | Trusted runs stay strict; forked PRs are documented exceptions. | Jobs begin failing only on missing secrets or unavailable hardware. |
| Astro/Starlight migration drops a current docs route or anchor | 3 | 5 | Preserve the current docs tree entry points while migrating and verify link coverage after every slice. | docs-agent | `docs-build` and link checks remain green during migration. | A previously reachable docs page or fragment vanishes. |
| Notebook coverage becomes cargo-cult repetition instead of useful learning artifacts | 2 | 4 | Require a coverage matrix that names the artifact type and the rationale for each language. | community-agent | Matrix distinguishes tutorial, example, notebook, or explicit exclusion. | A notebook is added only to satisfy a checklist. |
| Binding-owned examples drift out of sync with docs inventory | 3 | 4 | Treat bindings as blocked paths unless a handoff explicitly opens them. | docs-agent + binding owners | Coverage matrix and handoff notes stay aligned. | A docs page points to a missing or stale example. |
| Validation commands are added but not actually wired into CI | 2 | 4 | Keep the test matrix paired with workflow changes and require strict closeout evidence. | ci-agent | Workflow YAML and track test matrix match. | The repo documents a gate but CI does not run it. |
