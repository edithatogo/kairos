# Risk Register — 13 CI/CD, Code Quality & Supply Chain

| Risk | L | I | Sev | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| Workflow brittleness from matrix expansion | 3 | 3 | 9 | Pin workflow versions; test matrix changes on a branch before merging to main | ci-agent | Workflow matrix change breaks without branch test |
| Secret exposure in CI logs or artifacts | 2 | 5 | 10 | Use GitHub secrets + OpenID Connect; audit workflow logs for accidental env exposure | ci-agent | Any secret detected in CI log or artifact |
| Cross-platform failure (Windows/macOS/Linux) | 3 | 4 | 12 | Add platform smoke matrix for core crates; flag platform-specific failures as release blockers | ci-agent | Any required platform fails smoke test |
| Build cache poisoning | 2 | 4 | 8 | Checksum all cached artifacts; invalidate on workflow changes | ci-agent | Cache hash mismatch in CI |
| Supply-chain attack on CI dependencies (actions, runners) | 2 | 5 | 10 | Pin action SHAs; use GitHub's attestation and artifact verification | ci-agent | Unapproved action SHA detected |
| Benchmark regression undetected | 3 | 3 | 9 | Add comparison gate that fails on >20% regression from baseline | ci-agent | No benchmark failure on >20% measured regression |
| cargo-deny or cargo-audit misconfiguration | 3 | 4 | 12 | Test deny/audit config changes on a branch; require policy file review | ci-agent | cargo-deny or cargo-audit CI lane fails |
