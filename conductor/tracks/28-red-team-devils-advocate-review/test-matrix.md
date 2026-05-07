# Test Matrix: Track 28 Red Team & Devil's Advocate Review

| Check | Alpha | Beta | RC | 1.0 |
|---|---:|---:|---:|---:|
| Track docs exist and render cleanly | yes | yes | yes | yes |
| `reviews/red-team-report.md` exists and is current | yes | yes | yes | yes |
| `conductor/delivery-readiness-checklist.md` includes the red-team rows | yes | yes | yes | yes |
| Every major claim has a capability match or an explicit warning | pending | yes | yes | yes |
| Every critical finding has an owner and follow-up path | yes | yes | yes | yes |
| Release-blocker threshold is explicit | no | yes | yes | yes |
| Claim downgrade or removal path is documented | no | yes | yes | yes |
| Re-run cadence before release is explicit | yes | yes | yes | yes |
| Blocked paths are not violated (file-scope diff check) | yes | yes | yes | yes |
| Mermaid diagrams in track docs render cleanly | yes | yes | yes | yes |

## Focused validation commands

Run these from the repository root before release planning uses Track 28 output.

| Check | Command | Expected result | Last result |
|---|---|---|---|
| Evidence paths used by the claim ledger exist | `Test-Path -LiteralPath 'benches/benchmark-plan.md'; Test-Path -LiteralPath 'conformance/fixtures/manifest.json'; Test-Path -LiteralPath 'docs/release/release-checklist.md'; Test-Path -LiteralPath 'docs/release/compatibility.md'; Test-Path -LiteralPath 'packaging/release-package-manifest.json'; Test-Path -LiteralPath 'dist/release-artifact-manifest.json'` | First five `True`; release artifact manifest may remain `False` before a dry-run artifact build | `True True True True True False` on 2026-05-06 |
| Ready conformance fixture IDs are visible | `Get-Content -LiteralPath 'conformance/fixtures/manifest.json'` | Ready IDs include `scheduler_ordering_v1`, `scheduler_cancellation_v1`, `rng_reproducibility_v1`, `zero_delay_guard_v1`, and `vvuq_scenario_replay_v1`; DES/ABM/hybrid/Arrow/FFI IDs remain planned unless updated by Track 12 | Pass on 2026-05-06 |
| Package manifest is dry-run only | `Get-Content -LiteralPath 'packaging/release-package-manifest.json'` | `release_stage` is `r2-dry-run` and `production_publish_enabled` is `false` | Pass on 2026-05-06 |
| Red-team rows are present in release checklist | `Select-String -LiteralPath 'conductor/delivery-readiness-checklist.md' -Pattern 'Claim-versus-capability ledger','Blocker rubric','Red-team validation commands'` | All three patterns found | Pass on 2026-05-06 |
| Owner/freshness language is present | `Select-String -LiteralPath 'reviews/red-team-report.md' -Pattern 'Freshness date','Owner','Stage impact','Blocker rubric'` | All four patterns found | Pass on 2026-05-06 |
| Machine-readable ledger parses and required owners are present | `$ledger = Get-Content -Raw -LiteralPath 'conductor/tracks/28-red-team-devils-advocate-review/claim-capability-ledger.json' | ConvertFrom-Json; if (-not $ledger.freshness_date) { throw 'missing freshness_date' }; $missing = @($ledger.entries | Where-Object { $_.class -in @('blocker','warning') -and [string]::IsNullOrWhiteSpace($_.owner) }); if ($missing.Count) { throw 'ledger entries missing owners' }; $ledger.entries.Count` | Returns `10` and throws no owner/freshness error | Pass on 2026-05-06 |
| Write scope stayed in Track 28-owned files | `git status --short -- conductor/tracks/28-red-team-devils-advocate-review reviews conductor/red-team-review.md conductor/devils-advocate-review.md conductor/delivery-readiness-checklist.md` | Only Track 28-owned files for this worker's diff; full repo still has unrelated in-flight edits by others | Pass on 2026-05-06 |

## Validation interpretation

- A missing `dist/release-artifact-manifest.json` is acceptable before dry-run artifact generation, but it blocks RC or 1.0 claims that release artifacts, checksums, SBOM, or provenance are attached.
- Planned conformance fixtures do not support release claims until their fixture rows become `ready` and the conformance workflow validates them.
- A red-team report older than 14 days is stale even if the file exists.
