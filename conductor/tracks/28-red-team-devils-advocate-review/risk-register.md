# Risk Register: Track 28 Red Team & Devil's Advocate Review

| Risk | L | I | Sev | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| Findings are written but never affect release planning | 3 | 4 | 12 | Tie the report to a release checklist row and blocker decision | redteam-agent | Release proceeds with unresolved red-team finding |
| The report becomes stale before the next release stage | 3 | 4 | 12 | Require a freshness date and re-run cadence | release-agent | Report age exceeds re-run cadence without refresh |
| A claim is left in public docs after the supporting artifact disappears | 3 | 4 | 12 | Keep the claim-versus-capability ledger current | docs-agent | Claim-capability ledger mismatch at release |
| Red-team findings are assigned to the wrong worker | 3 | 3 | 9 | Include an explicit owner field for every finding | track subagent | Finding unowned for >1 sprint |
| Severity is inflated or understated without evidence | 3 | 3 | 9 | Use a fixed blocker rubric with examples | redteam-agent | Severity assigned without rubric cross-reference |
