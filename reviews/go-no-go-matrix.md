# Go / No-Go Matrix

| Area | Go condition | No-go condition |
|---|---|---|
| Naming | Registry names verified or fallbacks selected | `kairoecs` assumed available everywhere |
| Core | Deterministic trace fixtures pass | Scheduler ordering ambiguous or f64-based |
| FFI | Handles/frees/finalizers fuzz/smoke tested | Panics or references cross boundary |
| Python | 3.10-3.14 smoke tests pass for released wheels/source | Unsupported versions implied by docs |
| C# | .NET 10 passes; .NET 11 clearly preview until GA | .NET 11 represented as production stable before GA |
| Arrow | Schema versioned and copy semantics documented | Universal zero-copy claim made |
| Benchmarks | Public harness and raw data exist | Benchmark graph without reproduction path |
| Docs | Docs site builds, links pass, maturity labels visible | Docs claim future features as current |
| Security | SBOM/checksums/provenance attached where feasible | Native artifacts released with no integrity metadata |
| Governance | Maintainer/release/security ownership assigned | No one owns release-blocking issues |
| Red-team freshness | `reviews/red-team-report.md` freshness date is within 14 days or re-run for the target beta/RC/1.0 gate | Stale red-team report treated as current release evidence |
| Claim ledger | Every blocker/warning has evidence, verdict, owner, and stage impact | Unsupported claim has no owner or follow-up path |
| Release artifacts | `dist/release-artifact-manifest.json`, `dist/SHA256SUMS`, SBOM, and provenance exist before RC/1.0 artifact claims | Release notes claim artifacts/checksums/SBOM/provenance before generation |
