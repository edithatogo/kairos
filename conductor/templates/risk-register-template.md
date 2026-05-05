# Risk Register — <!-- __TRACK_ID__ --> <!-- __TRACK_NAME__ -->

## Severity scoring

| Severity band | Score range | Action |
|---|---|---|
| Low | 1-6 | Accept, note in handoff.md |
| Medium | 7-14 | Accept with mitigation plan; owner must acknowledge |
| High | 15-20 | Mitigation required before track status can advance to In Review |
| Critical / Release-Blocker | 20+ | Track blocked; release-agent and governance-agent must review; ADR required |

**Escalation rule**: Any Medium risk whose mitigation deadline passes without resolution must be escalated to High. Track owner must file an escalation issue and notify governance-agent within 48 hours of deadline expiry.

## Active risks

| # | Risk | Likelihood (1-5) | Impact (1-5) | Severity (L x I) | Mitigation | Owner | Trigger / Escalation |
|---|---|---|---|---|---|---|---|
| 1 | <!-- Describe the risk --> | <!-- 1-5 --> | <!-- 1-5 --> | <!-- L x I --> | <!-- Concrete mitigation steps --> | <!-- Agent or role --> | <!-- Condition that triggers escalation --> |
| 2 | <!-- ... --> | | | | | | |

<!-- Add rows as needed. Copy the empty row template below:

| <N> | <risk description> | <1-5> | <1-5> | <L*I> | <mitigation> | <owner> | <trigger> |

-->

## Resolved / historical

| # | Risk | Resolution | Resolved date | Resolved by |
|---|---|---|---|---|
| <!-- 1 --> | <!-- risk description --> | <!-- how it was resolved --> | <!-- YYYY-MM-DD --> | <!-- agent or role --> |

## Instructions

1. **Likelihood**: 1 = near-certain it will NOT happen, 5 = near-certain it WILL happen within the track timeline.
2. **Impact**: 1 = cosmetic or trivial, 5 = blocks release or breaks compatibility promises.
3. **Severity**: Multiply Likelihood x Impact. Score 1-25.
4. **Mitigation**: Must be a concrete, verifiable action. Avoid vague mitigations like "be careful."
5. **Owner**: Must be a named subagent from `conductor/subagents.yaml` or a specific role (e.g. "governance-agent").
6. **Trigger / Escalation**: Define the observable condition that triggers escalation (e.g. "CI benchmark regression >5%", "ADR not submitted by beta freeze").
7. Update this register whenever a risk materializes, is mitigated, or its likelihood/impact changes.
8. Mark resolved risks in the historical table; do not delete rows from the active table.
