# Risk Register: Track 29 Wave Manager & Execution Gatekeeper

| Risk | L | I | Sev | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| Wave policy is too rigid and blocks legitimate parallel work | 3 | 4 | 12 | Document exception override path with ADR requirement | wave-manager-agent | Two consecutive overrides for the same dependency pair |
| A track dependency is misconfigured in tracks.yaml | 3 | 4 | 12 | Validate dependency graph on every gate run; flag cycles immediately | foundation-agent | Cycle detected in dependency graph |
| A track is marked "Done" but its outputs are incomplete | 3 | 4 | 12 | Each track owns its "Done" criteria; wave gate checks status only, not output quality | wave-manager-agent | Downstream track blocked by incomplete upstream output |
| The critical-path heatmap is stale | 2 | 3 | 6 | Regenerate heatmap on every track status change | wave-manager-agent | Heatmap last-generated timestamp exceeds track-status age by >1hr |
| Transitive dependency closure is expensive to compute for 35+ tracks | 2 | 2 | 4 | Compute once per gate run; cache between status changes | wave-manager-agent | Gate run exceeds 60s wall-clock time |
| Maintainers override wave gates too frequently, eroding policy trust | 2 | 4 | 8 | Require ADR for each override; track override count in wave-policy.md | governance-agent | >3 overrides in a single release cycle |
