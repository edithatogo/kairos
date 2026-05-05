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
