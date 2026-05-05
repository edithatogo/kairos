# Risk Register Template

## Severity Scoring

Each risk is scored by multiplying **Likelihood (L)** by **Impact (I)**:

**Severity = L × I**

### Likelihood Scale (1-5)

| Value | Label | Description |
|-------|-------|-------------|
| 1 | Rare | Would require exceptional circumstances; never seen before |
| 2 | Unlikely | Could happen but not expected under normal operation |
| 3 | Possible | Might occur at some point; has been observed in similar projects |
| 4 | Likely | Will probably occur; has happened before or conditions favor it |
| 5 | Almost Certain | Expected to occur; multiple paths lead to this outcome |

### Impact Scale (1-5)

| Value | Label | Description |
|-------|-------|-------------|
| 1 | Negligible | No noticeable effect on project outcomes |
| 2 | Minor | Minor inconvenience; workaround exists; no schedule impact |
| 3 | Moderate | Noticeable delay or quality reduction; partial workaround |
| 4 | Major | Significant delay, cost, or quality impact; may affect release |
| 5 | Critical | Project blocker; release cannot proceed; existential threat |

### Severity Thresholds

| Range | Label | Action |
|-------|-------|--------|
| 1-4 | Low | Monitor; accept or document |
| 5-8 | Medium | Active mitigation required; track in sprint |
| 9-15 | High | Escalate to track owner; mitigation must be in place before next phase |
| 16-25 | Critical / Release-Blocker | Immediate escalation; release cannot proceed until resolved or accepted by governance |

## Table Format

```markdown
# Risk Register — {Track Number} {Track Title}

| Risk | L | I | Sev | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| {Risk description} | {1-5} | {1-5} | {L×I} | {What will be done to reduce likelihood or impact} | {responsible-agent} | {Condition that triggers escalation} |
```

### Column Definitions

- **Risk**: A concise description of the risk (what could go wrong)
- **L**: Likelihood score (1-5 per the Likelihood Scale above)
- **I**: Impact score (1-5 per the Impact Scale above)
- **Sev**: Severity = L × I (auto-calculated)
- **Mitigation**: Action taken or planned to reduce likelihood or impact. Must be testable or verifiable.
- **Owner**: The agent or role accountable for managing this risk. Use `{domain}-agent` naming convention (e.g., `core-agent`, `ci-agent`, `release-agent`).
- **Escalation trigger**: A specific, observable condition that—if met—requires immediate escalation to the track owner or release governance. Must be falsifiable (it must be possible to determine whether the condition has been met).

## Escalation Triggers

Escalation triggers are specific, measurable conditions. Good triggers are:

- **Observable**: Someone can definitively determine if the condition has been met
- **Actionable**: Meeting the condition implies a concrete next step (raise issue, block release, notify owner)
- **Time-bound where applicable**: "if X persists for >N days" rather than "if X happens"

### Examples of Good Escalation Triggers

- "Any conformance fixture fails against canonical output"
- "Test PyPI dry-run fails"
- "CRAN submission pending >2 weeks after release"
- ">20% regression on any tracked benchmark"
- "Two consecutive changes made without an ADR"

### Examples of Poor Escalation Triggers

- "If things go wrong" (not observable)
- "Quality degrades" (not measurable)
- "Team is unhappy" (not falsifiable)

## Guidelines

1. Each track should have **4-9 risks** covering the most impactful failure modes.
2. Every Severity ≥ 9 risk should have an escalation trigger that is actionable before a release.
3. Every Severity ≥ 16 risk should have an escalation trigger that **blocks the release**.
4. Review and update the risk register at each phase transition (Spec Approved → In Progress → Review → Done).
5. When a risk materializes, add a dated note or link to the incident/issue.
