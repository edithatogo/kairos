# Risk Register: Track 25 API Design Review & Compatibility Governance

| Risk | L | I | Sev | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| Compatibility policy is vague and cannot block a release | 3 | 4 | 12 | Make release-stage rules explicit in the spec and checklist | api-governance-agent | Release proceeds with ambiguous policy interpretation |
| A protected surface is missing from the inventory | 3 | 4 | 12 | Treat missing inventory as an unreviewed release risk | release-agent | Protected surface changes without inventory entry |
| Breaking changes slip through without an ADR | 3 | 4 | 12 | Require ADRs for protected-surface changes before beta | contracts-agent | Protected-surface change merges without ADR |
| Migration notes exist but do not match the actual API delta | 3 | 4 | 12 | Tie notes to named surfaces and review the diff against them | docs-agent | Migration note diff-check fails against actual API delta |
| Another worker is asked to negotiate compatibility policy | 2 | 3 | 6 | Keep policy ownership in this track and hand off only implementation facts | track subagent | Compatibility policy decision made outside track |
