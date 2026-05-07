# Risk Register — 15 Packaging, Publishing & Delivery

Severity scale: Likelihood 1-5 x Impact 1-5. Low 1-4, Medium 5-9, High 10-16, Critical 17-25.

| Risk | Likelihood | Impact | Severity | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| Contract drift | 3 | 4 | 12 | Use `conductor/contracts`, the package matrix, and conformance fixtures | packaging-agent | Conformance fixture fails on any package surface |
| Registry name collision | 3 | 5 | 15 | Check names early and record fallbacks for every ecosystem | packaging-agent | Name unavailable in any target registry |
| CI blind spot | 3 | 3 | 9 | Add track-specific doc/test gates and no-live-publish checks | ci-agent | Release-plan merge breaks downstream check |
| Public API churn | 4 | 3 | 12 | Gate public surface changes through ADR | packaging-agent | Public API changes without ADR |
| Package/version mismatch | 3 | 5 | 15 | Use pack and dry-run validation before any production publish | packaging-agent | Any dry-run reveals version mismatch |
| Toolchain drift | 3 | 3 | 9 | Pin minimum supported versions in the package catalog | packaging-agent | Pinned toolchain version goes EOL |
| Scope creep | 3 | 4 | 12 | Keep the track limited to metadata, release policy, and dry-runs | packaging-agent | Track merges implementation code |
