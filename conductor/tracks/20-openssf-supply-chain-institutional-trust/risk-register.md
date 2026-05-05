# Risk Register: Track 20 OpenSSF, Supply Chain Trust & Institutional Readiness

| Risk | L | I | Sev | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| The track stays descriptive and never blocks a release | 3 | 4 | 12 | Every trust claim must map to a checklist row or gate command | security-agent | Release proceeds with unchecked trust claim |
| The readiness docs drift away from actual CI behavior | 3 | 4 | 12 | Update checklist and gates together in the same change | ci-agent | CI gate passes but readiness doc contradicts result |
| A missing policy file is treated as advisory instead of blocking | 3 | 4 | 12 | Make `SECURITY.md`, `CODEOWNERS`, and dependency policy release inputs | release-agent | Release candidate lacks required policy file |
| A temporary tooling gap is mistaken for a policy waiver | 3 | 3 | 9 | Separate allowed-failure lanes from approved exceptions | security-agent | Allowed-failure lane result used to justify exception |
| Another worker is asked to author the trust policy | 2 | 3 | 6 | Keep ownership in this track and hand off only dependent checks | track subagent | Trust policy PR authored by non-track member |
