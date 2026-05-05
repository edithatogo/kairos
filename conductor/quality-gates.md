# KairoECS Quality Gates

## Core Rust gates

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo nextest run --workspace --all-features
cargo test --doc --workspace
cargo llvm-cov --workspace --all-features --fail-under-lines 80
cargo deny check
cargo audit
cargo semver-checks check-release
```

## Heavy/nightly gates

```bash
cargo miri test -p kairo-ecs-core
cargo miri test -p kairo-ecs-state
cargo fuzz run ffi_boundary -- -max_total_time=60
cargo bench --workspace
```

## Binding gates

| Binding | Minimum gates |
|---|---|
| Python 3.10-3.14 | `pytest`, `ruff`, type checks, wheel build, Arrow roundtrip, package root exists |
| R | `R CMD check`, `testthat`, `lintr`, pkgdown build, Arrow roundtrip, package root exists |
| Julia | `Pkg.test`, `Aqua.jl`, formatting, Documenter build, Arrow roundtrip, package root exists |
| TypeScript | `tsc --noEmit`, `vitest`, Wasm browser/Node smoke, TypeDoc, package root exists |
| C# .NET 10 | `dotnet test`, analyzers, NuGet pack, DocFX, Arrow roundtrip, solution exists |
| Go | `go test`, `go vet`, `gofmt`, `staticcheck`, cgo smoke, Arrow roundtrip, module root exists |

## Gate definitions

### Tracks 03-05 specific gates

**des-fixture**: `cargo test -p kairo-ecs-des --test des_resource_queue_v1` — the DES resource queue fixture must exist and pass.

**abm-fixture**: `cargo test -p kairo-ecs-abm --test abm_behavior_update_v1` — the ABM behavior update fixture must exist and pass.

**arrow-schema-versioning**: `cargo test -p kairo-ecs-arrow --test schema_compatibility` — Arrow schemas must include a version field and pass roundtrip with the previous major version.

**optional-feature-check**: `cargo check -p kairo-ecs-core --no-default-features` — the core crate must compile without viz or optional features enabled.

**headless-core-check**: `cargo test --workspace --exclude kairo-ecs-viz` — all non-viz crates must build and test without the viz crate.

## Simulation-specific tests

1. Deterministic event ordering.
2. Cancellation correctness.
3. Zero-delay event guardrails.
4. Reproducible RNG streams per entity.
5. 1,000,000 entity creation and memory sanity benchmark.
6. DES resource queue fixture.
7. ABM behavior update fixture.
8. Hybrid DES/ABM fixture.
9. Arrow event log schema compatibility.
10. FFI lifecycle and double-free prevention.
11. Panic containment at FFI boundary.
12. Cross-language conformance output parity.

## Release gates

No release unless:

```text
all required CI green
all package dry-runs green
docs site green
changelog updated
compatibility notes updated
SBOM/checksums/provenance generated
security scan complete
release checklist signed off
```

### Secret scanning

```bash
# gitleaks detect must be run in CI for all PRs to main
# .github/workflows/secret-scan.yml must exist and fail on findings
```

## Governance gates

### OpenSSF and supply-chain

```bash
markdownlint "**/*.md"
check-links conductor/delivery-readiness-checklist.md
test -f SECURITY.md
test -f CODEOWNERS
# .github/workflows/scorecard.yml must exist and run on main
# .github/workflows/dependency-review.yml must exist and fail on high severity
# .github/workflows/sbom-attestations.yml must exist for published release SBOMs
# .github/workflows/release-attestations.yml must exist for release artifact attestations
# SBOM and provenance artifacts must be generated for release builds
# package and binding CI must fail on missing manifests once a track is In Progress
# any waiver or exception must be recorded in conductor/release-engineering.md before release
# `.github/CODEOWNERS` must exist and cover the maintained paths
# `.github/workflows/secret-scan.yml` must exist and fail on findings
```

### API compatibility

```bash
# surface inventory reviewed for Rust crates and binding package roots:
# crates/kairo-ecs-types, crates/kairo-ecs-core, crates/kairo-ecs-state, crates/kairo-ecs-rng
# bindings/python, bindings/r, bindings/julia, bindings/typescript, bindings/csharp, bindings/go
# breaking-change definition present in conductor/contracts/versioning-compatibility.md
# ADR required for protected-surface changes
# migration notes required for breaking changes
# package matrix and catalog must match the live binding/package roots
# compatibility notes must name the affected package root before beta or later
```

### Red-team / release block review

```bash
# reviews/red-team-report.md is current for the planned release stage
# every critical finding has an owner and follow-up path
# unresolved critical findings block beta, RC, and 1.0
# Track 28 must reference the current package and compatibility claims
```

## Community/SOTA gates

| Gate | Alpha | Beta | RC | 1.0 |
|---|---:|---:|---:|---:|
| CITATION.cff present | yes | yes | yes | yes |
| Comparative benchmarks and reproducibility guidance (Track 18) | yes | yes | yes | yes |
| Research software, citation, and archival guidance (Track 19) | yes | yes | yes | yes |
| OpenSSF, supply-chain trust, and institutional-readiness guidance (Track 20) | yes | yes | yes | yes |
| Community adoption, education, and ecosystem guidance (Track 17) | yes | yes | yes | yes |
| Verification, validation, and uncertainty guidance (Track 21) | yes | yes | yes | yes |
| Scenario runner and replay guidance (Track 22) | yes | yes | yes | yes |
| Model zoo examples | 3 | 6 | 8 | 10+ |
| Starter-kit and model-zoo guidance (Track 23) | yes | yes | yes | yes |
| Playground and demo guidance (Track 24) | yes | yes | yes | yes |
| API design review and compatibility governance guidance (Track 25) | yes | yes | yes | yes |
| Interoperability standards review guidance (Track 26) | yes | yes | yes | yes |
| Benchmark harness public | partial | yes | yes | yes |
| OpenSSF Scorecard workflow | scaffold | active | active | active |
| SBOM attached | optional | dry-run | yes | yes |
| Provenance/attestation | optional | dry-run | yes where supported | yes where supported |
| Red-team review | yes | yes | yes | yes |
| Compatibility promise page | yes | yes | yes | yes |
| Reproducibility docs | partial | yes | yes | yes |
