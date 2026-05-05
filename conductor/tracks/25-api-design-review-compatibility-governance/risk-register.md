# Risk Register: Track 25 API Design Review & Compatibility Governance

| Risk | L | I | Sev | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| Compatibility policy is vague and cannot block a release | 3 | 4 | 12 | Make release-stage rules explicit in the spec and checklist | api-governance-agent | Release proceeds with ambiguous policy interpretation |
| A protected surface is missing from the inventory | 2 | 4 | 8 | `docs/design/protected-surface-inventory.json` is validated by `docs/design/validate-compatibility-pack.ps1`; missing roots are release holds | release-agent | Protected surface changes without inventory entry or validator fails |
| Breaking changes slip through without an ADR | 2 | 4 | 8 | `conductor/contracts/versioning-compatibility.md` makes ADRs mandatory for protected-surface semantic changes, root renames, splits, merges, removals, or promise changes | contracts-agent | Protected-surface change merges without ADR |
| Migration notes exist but do not match the actual API delta | 3 | 4 | 12 | Tie notes to named surfaces and review the diff against them | docs-agent | Migration note diff-check fails against actual API delta |
| Another worker is asked to negotiate compatibility policy | 2 | 3 | 6 | Keep policy ownership in this track and hand off only implementation facts | track subagent | Compatibility policy decision made outside track |
| Release compatibility note drifts from the protected-surface inventory | 3 | 4 | 12 | `docs/design/validate-compatibility-pack.ps1 -ReleaseGate` checks every inventory root appears in `docs/release/compatibility.md` before beta, RC, or 1.0 signoff | release-agent | Current release-gate failure: `docs/release/compatibility.md` omits `include/kairo_ecs.h` and `schemas/arrow/event_log_v1.schema.json` |
