# Risk Register: Track 30 Toolchain & Version Support Matrix

| Risk | Likelihood | Impact | Mitigation | Owner |
|---|---:|---:|---|---|
| A binding track ships with an undocumented minimum version | Medium | High | Require every binding to declare min version in the matrix before reaching beta | toolchain-agent |
| CI runner drops a version without updating the matrix | Medium | High | `toolchain-check.yml` fails on mismatch; gate blocks release | toolchain-agent |
| A language ecosystem releases a new major version faster than the update cycle | Medium | Medium | Set a maximum lag policy (e.g., support new major within 1 release cycle) | toolchain-agent |
| Version-drop policy is ignored by binding tracks | Medium | High | Gate `version-drop-policy-check` blocks release; require ADR for exceptions | release-agent |
| OS/arch support matrix is incomplete | Medium | Medium | Mark unverified cells as "best-effort"; gate only requires CI-covered cells to be verified | toolchain-agent |
| Toolchain matrix diverges from CI runner reality | Medium | High | `toolchain-check.yml` runs on every PR that modifies the matrix or package manifests | ci-agent |
| Multiple binding tracks declare conflicting minimum versions for the same language | Low | Medium | Toolchain-agent owns the single source of truth; binding tracks read from the matrix | toolchain-agent |
