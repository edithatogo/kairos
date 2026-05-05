# Risk Register: Track 29 Wave Manager & Execution Gatekeeper

| Risk | Likelihood | Impact | Mitigation | Owner |
|---|---:|---:|---|---|
| Wave policy is too rigid and blocks legitimate parallel work | Medium | High | Document exception override path with ADR requirement | wave-manager-agent |
| A track dependency is misconfigured in tracks.yaml | Medium | High | Validate dependency graph on every gate run; flag cycles immediately | foundation-agent |
| A track is marked "Done" but its outputs are incomplete | Medium | High | Each track owns its "Done" criteria; wave gate checks status only, not output quality | wave-manager-agent |
| The critical-path heatmap is stale | Low | Medium | Regenerate heatmap on every track status change | wave-manager-agent |
| Transitive dependency closure is expensive to compute for 35+ tracks | Low | Low | Compute once per gate run; cache between status changes | wave-manager-agent |
| Maintainers override wave gates too frequently, eroding policy trust | Low | High | Require ADR for each override; track override count in wave-policy.md | governance-agent |
