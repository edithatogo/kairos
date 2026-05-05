# Risk Register — 16 Release Governance & Maintenance

| Risk | L | I | Sev | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| Compatibility promise broken by oversight | 3 | 5 | 15 | Enforce semver check in CI; require ADR before any public-API change | release-agent | Semver check fails on release candidate |
| Changelog/CHANGELOG.md stale | 4 | 3 | 12 | Require changelog entry as merge gate; validate in release CI | release-agent | PR merges without changelog entry |
| Deprecation policy not followed | 3 | 4 | 12 | Deprecation notice must appear in at least one published release before removal | release-agent | Removal without prior deprecation notice |
| Maintainer capacity insufficient for polyglot support | 3 | 4 | 12 | Define maintainer-per-ecosystem coverage; flag uncovered ecosystems before release | release-agent | Any ecosystem has zero assigned maintainers at release |
| Release checklist not ran before publish | 4 | 5 | 20 | Automate checklist verification in the release pipeline; block publish on failure | release-agent | Checklist gate fails on release candidate |
