# Risk Register: Track 30 Toolchain & Version Support Matrix

| Risk | L | I | Sev | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| A binding track ships with an undocumented minimum version | 3 | 4 | 12 | Require every binding to declare min version in the matrix before reaching beta | toolchain-agent | Any binding enters beta without min-version declaration |
| CI runner drops a version without updating the matrix | 3 | 4 | 12 | `toolchain-check.yml` fails on mismatch; gate blocks release | toolchain-agent | Matrix contains a version not present on any CI runner |
| A language ecosystem releases a new major version faster than the update cycle | 3 | 3 | 9 | Set a maximum lag policy (e.g., support new major within 1 release cycle) | toolchain-agent | New major version unsupported 2+ release cycles after release |
| Version-drop policy is ignored by binding tracks | 3 | 4 | 12 | Gate `version-drop-policy-check` blocks release; require ADR for exceptions | release-agent | Version dropped without ADR or policy-compliant notice |
| OS/arch support matrix is incomplete | 3 | 3 | 9 | Mark unverified cells as "best-effort"; gate only requires CI-covered cells to be verified | toolchain-agent | Required-platform cell unverified at release |
| Toolchain matrix diverges from CI runner reality | 3 | 4 | 12 | `toolchain-check.yml` runs on every PR that modifies the matrix or package manifests | ci-agent | `toolchain-check.yml` fails |
| Multiple binding tracks declare conflicting minimum versions for the same language | 2 | 3 | 6 | Toolchain-agent owns the single source of truth; binding tracks read from the matrix | toolchain-agent | Two bindings declare different min versions for same language |
