# Risk Register: Track 28 Red Team & Devil's Advocate Review

| Risk | Likelihood | Impact | Mitigation | Owner |
|---|---:|---:|---|---|
| Findings are written but never affect release planning | Medium | High | Tie the report to a release checklist row and blocker decision | redteam-agent |
| The report becomes stale before the next release stage | Medium | High | Require a freshness date and re-run cadence | release-agent |
| A claim is left in public docs after the supporting artifact disappears | Medium | High | Keep the claim-versus-capability ledger current | docs-agent |
| Red-team findings are assigned to the wrong worker | Medium | Medium | Include an explicit owner field for every finding | track subagent |
| Severity is inflated or understated without evidence | Medium | Medium | Use a fixed blocker rubric with examples | redteam-agent |
