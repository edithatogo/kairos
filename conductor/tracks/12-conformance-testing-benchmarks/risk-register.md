# Risk Register — 12 Conformance, Testing & Benchmarks

| Risk | L | I | Sev | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| Contract drift | 3 | 4 | 12 | Use `conductor/contracts` and conformance fixtures | conformance-agent | Conformance fixture fails on any subagent output |
| CI blind spot | 3 | 4 | 12 | Add track-specific workflow/test gate | ci-agent | A merge passes CI but breaks a downstream track |
| Public API churn | 4 | 3 | 12 | Gate public surface changes through ADR | conformance-agent | Public API changes without ADR >2 times |
| Package/version mismatch | 3 | 5 | 15 | Use publishing dry-runs before production | conformance-agent | Any dry-run reveals version mismatch |
| Fixture naming drift | 3 | 3 | 9 | Freeze fixture IDs in `conformance/fixtures/manifest.json` | conformance-agent | Fixture ID collision or rename detected |
| Benchmark naming drift | 3 | 3 | 9 | Keep scenario names stable in `benches/benchmark-plan.md` | conformance-agent | Benchmark name change not reflected in plan |
