# Risk Register: Track 28 Red Team & Devil's Advocate Review

Severity scale: Likelihood 1-5 x Impact 1-5. Low 1-4, Medium 5-9, High 10-16, Critical 17-25.

| Risk | Likelihood | Impact | Severity | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| Findings are written but never affect release planning | 3 | 4 | 12 | Tie the report to a release checklist row and blocker decision | redteam-agent | Release proceeds with unresolved red-team finding |
| The report becomes stale before the next release stage | 3 | 4 | 12 | Require a freshness date and re-run cadence | release-agent | Report age exceeds re-run cadence without refresh |
| A claim is left in public docs after the supporting artifact disappears | 3 | 4 | 12 | Keep the claim-versus-capability ledger current | docs-agent | Claim-capability ledger mismatch at release |
| Red-team findings are assigned to the wrong worker | 3 | 3 | 9 | Include an explicit owner field for every finding | track subagent | Finding unowned for >1 sprint |
| Severity is inflated or understated without evidence | 3 | 3 | 9 | Use a fixed blocker rubric with examples | redteam-agent | Severity assigned without rubric cross-reference |
| Release artifact evidence is claimed before artifacts exist | 3 | 5 | 15 | Treat missing `dist/release-artifact-manifest.json`, `dist/SHA256SUMS`, or SBOM/provenance evidence as an RC/1.0 blocker | release-agent | Release notes claim artifacts, checksums, SBOM, or provenance before files are generated |
| Planned conformance fixtures are mistaken for ready coverage | 3 | 4 | 12 | Ledger limits claims to ready fixture IDs and flags DES/ABM/hybrid/Arrow/FFI fixture IDs as planned until Track 12 updates the manifest | conformance-agent | Release claim relies on a planned fixture ID |
| Owner/freshness validation is skipped | 3 | 4 | 12 | Require freshness date, owner, stage impact, and focused validation command results in the report/test matrix | redteam-agent | Red-team report is older than 14 days or blocker/warning row has no owner |

## Current blocker readout

| Finding | Class | Owner | Stage impact | Evidence |
|---|---|---|---|---|
| Release artifact manifest absent in focused check | Blocker for RC/1.0 artifact claims | release-agent | Blocks RC/1.0 claims about attached release artifacts, checksums, SBOM, or provenance | `Test-Path -LiteralPath 'dist/release-artifact-manifest.json'` returned `False` on 2026-05-06 |
| DES/ABM/hybrid/Arrow/FFI conformance fixtures are still planned | Warning for alpha; blocker for beta+ claims outside ready fixtures | conformance-agent | Blocks beta+ claims that these surfaces are conformance-proven | `conformance/fixtures/manifest.json` shows these IDs as `planned` |
| Six binding roots are not production-ready merely because roots exist | Warning for alpha; blocker for RC/1.0 stable binding claims | binding owners, release-agent | Blocks stable multi-binding release language until per-binding checks and package dry-runs are green | `packaging/release-package-manifest.json` is dry-run only and `production_publish_enabled` is `false` |
