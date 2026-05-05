# Risk Register: Track 25 API Design Review & Compatibility Governance

| Risk | Likelihood | Impact | Mitigation | Owner |
|---|---:|---:|---|---|
| Compatibility policy is vague and cannot block a release | Medium | High | Make release-stage rules explicit in the spec and checklist | api-governance-agent |
| A protected surface is missing from the inventory | Medium | High | Treat missing inventory as an unreviewed release risk | release-agent |
| Breaking changes slip through without an ADR | Medium | High | Require ADRs for protected-surface changes before beta | contracts-agent |
| Migration notes exist but do not match the actual API delta | Medium | High | Tie notes to named surfaces and review the diff against them | docs-agent |
| Another worker is asked to negotiate compatibility policy | Low | Medium | Keep policy ownership in this track and hand off only implementation facts | track subagent |
