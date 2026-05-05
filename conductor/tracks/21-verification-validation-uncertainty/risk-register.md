# Risk Register: Track 21 Verification, Validation & Uncertainty

| Risk | L | I | Sev | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| A docs claim overstates what replay actually proves | 3 | 4 | 12 | Require the page to name the scenario, seed, trace, and comparison basis | docs-agent | Claim published without scenario/seed/trace reference |
| Validation is confused with verification | 3 | 4 | 12 | Keep the term definitions and evidence boundary on the page | track subagent | Page conflates validation and verification terms |
| Scenario fixtures drift away from replay commands | 3 | 4 | 12 | Tie claims to committed fixtures and update them together | fixture-owner | Fixture change without corresponding doc update |
| Uncertainty is described only narratively | 3 | 3 | 9 | Require a replications or sensitivity artifact and a short interpretation note | analysis-owner | Claim published without quantitative uncertainty artifact |
| The page becomes disconnected from release review | 2 | 4 | 8 | Keep the handoff aligned with track tests and release-facing claims | release-agent | Release review references stale V&V page |
