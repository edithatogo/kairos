# Risk Register — 00 Project Foundation, Governance & Naming

| Risk | L | I | Sev | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| Naming conflict on registries (crates.io/PyPI/npm/NuGet) | 3 | 5 | 15 | Comprehensive naming due-diligence across all target registries before first release | foundation-agent | Any unresolved conflict blocks Spec Approved→In Progress |
| Trademark/common-law conflict | 2 | 5 | 10 | Trademark clearance search and common-law availability assessment before name lock-in | foundation-agent | Any competing mark or cease-and-desist received |
| Governance model not ratified by contributors | 3 | 4 | 12 | Formal ratification process with contributor voting; ADR 0001-record-architecture-decisions.md sets precedent | foundation-agent | Release blocked until governance ratified |
| ADR process abandonment | 3 | 3 | 9 | Gate significant changes through ADR; link from spec acceptance criteria | foundation-agent | Two consecutive changes without ADR |
| Missing license file compliance | 2 | 4 | 8 | Include LICENSE.md at repo root with explicit SPDX identifier; validate in CI gate | foundation-agent | CI license gate fails |
