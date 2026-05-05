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

### Track 30 toolchain and version support gates

**toolchain-matrix-current**: `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1` plus `.github/workflows/toolchain-check.yml` live setup lanes - the support matrix must name Rust, Python, R, Julia, TypeScript/Wasm, C#, and Go, must include min/latest/deprecation/OS-arch coverage columns, and every `CI-covered` selector must install and report the declared major/minor version.

**version-drop-policy-check**: `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1` - the matrix must retain the version-drop policy, proposed-drops table, release-note/README notice requirement, 2-release-cycle-or-6-month notice period, and documented removal criteria before any supported version or CI lane is removed.

### Track 31 performance regression gates

**threshold-definition-exists**: `python benches/regression/compare.py` - every active benchmark in `benches/benchmark-smoke.json` and `conformance/fixtures/manifest.json` must have exactly one row in `conductor/performance-thresholds.md`; orphaned threshold rows fail the gate.

**benchmark-regression-check**: `python benches/regression/compare.py --base <base-results.json> --current <current-results.json>` - supplied benchmark result pairs must report benchmark name, base mean, current mean, percent change, threshold, gate class, and pass/fail status. Until Track 12 publishes stable native benchmark artifacts, `.github/workflows/bench-regression.yml` runs threshold coverage plus benchmark target compilation rather than full base-vs-PR native timing.

### Tracks 32-35 accelerator, PDES, and distributed gates

**gpu-parity-check**: `pwsh -NoProfile -File conductor/tracks/32-gpu-compute-acceleration/validate-track32.ps1 -SkipCargoTest` plus `cargo check --manifest-path crates/kairo-ecs-gpu/Cargo.toml --features wgpu-backend,cuda-backend --tests` - validates the GPU crate contract, feature isolation, explicit unavailable-backend errors, and scaffold parity harness compilation. Real hardware CPU-vs-GPU parity remains blocked until backend dependencies and GPU runners are introduced.

**gpu-benchmark-threshold**: `rg -n "not yet available|not publish speedup" docs/gpu-compute/benchmark-results.md` - blocks unsupported GPU speedup claims until Track 12 benchmark outputs and hardware runner evidence exist.

**browser-webgpu-smoke**: `npm test --prefix website/webgpu-demo` and `npm run validate:wgsl --prefix website/webgpu-demo` - validates the static browser demo, fallback state, and GPU-free WGSL subset. Real browser WebGPU device initialization remains blocked until Wasm/browser dependency wiring and browser-runner proof exist.

**wasm-gpu-parity**: `cargo check --manifest-path crates/kairo-ecs-webgpu/Cargo.toml --features webgpu --tests` - validates the WebGPU crate's CPU fallback/parity metadata and explicit not-configured dispatch behavior. Real Wasm/WebGPU parity remains blocked until Track 09 and WebGPU runtime bindings are available.

**pdes-sequential-parity**: `cargo check --manifest-path crates/kairo-ecs-pdes/Cargo.toml --features pdes --tests` - validates that PDES parity fixtures compile. Runtime parity execution is blocked in this Windows shell by linker resolution and production parallel scheduling remains future work.

**gvt-progression-check**: `cargo check --manifest-path crates/kairo-ecs-pdes/Cargo.toml --features pdes --tests` - validates that GVT progression evidence and deadlock-smoke fixture code compile. Production GVT stress execution remains a beta/RC gate.

**mpi-smoke**: `cargo check --manifest-path crates/kairo-ecs-mpi/Cargo.toml --features mpi --tests` - validates the MPI transport protocol emulator, rank/tag checks, migration envelope, and telemetry envelope. Real `rsmpi` transport remains future work.

**grpc-smoke**: `cargo check --manifest-path crates/kairo-ecs-grpc/Cargo.toml --features grpc --tests` - validates the gRPC protocol emulator, peer/config checks, migration envelope, telemetry envelope, and heartbeat classification. Real `tonic` service wiring remains future work.

**entity-migration-check**: `cargo check --manifest-path crates/kairo-ecs-mpi/Cargo.toml --features mpi --tests` and `cargo check --manifest-path crates/kairo-ecs-grpc/Cargo.toml --features grpc --tests` - validates the dependency-free migration envelope contracts for both distributed transport scaffolds.

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
test -f .github/CODEOWNERS
test -f .github/dependabot.yml || test -f renovate.json
test -f .github/workflows/actions-security.yml
test -f .github/workflows/workflow-security.yml
test -f .github/workflows/secret-scan.yml
# .github/workflows/scorecard.yml must exist and run on main
# .github/workflows/dependency-review.yml must exist and fail on high severity
# .github/workflows/sbom-attestations.yml must exist for published release SBOMs
# .github/workflows/release-attestations.yml must exist for release artifact attestations
# SBOM and provenance artifacts must be generated for release builds
# package and binding CI must fail on missing manifests once a track is In Progress
# any waiver or exception must be recorded in conductor/release-engineering.md before release
# `.github/CODEOWNERS` must exist and cover the maintained paths
# `.github/workflows/secret-scan.yml` must exist and fail on findings
# `.github/workflows/toolchain-check.yml` must exist when Track 30 is In Progress
# `.github/workflows/bench-regression.yml` must exist when Track 31 is In Progress
```

Machine-checkable release-trust references:

```bash
test -f .github/workflows/scorecard.yml
test -f .github/workflows/dependency-review.yml
test -f .github/workflows/sbom-attestations.yml
test -f .github/workflows/release-attestations.yml
rg -n "fail-on-severity:\s*high" .github/workflows/dependency-review.yml
rg -n "attestations:\s*write|actions/attest|sbom.spdx.json|SHA256SUMS" .github/workflows/sbom-attestations.yml .github/workflows/release-attestations.yml
rg -n "OpenSSF and supply-chain readiness|scorecard.yml|dependency-review.yml|sbom-attestations.yml|release-attestations.yml|allowed-failure|exception|waiver" conductor/delivery-readiness-checklist.md conductor/tracks/20-openssf-supply-chain-institutional-trust/supply-chain-plan.md
```

Release artifact tree checks, required for RC and 1.0:

```bash
test -f dist/RELEASE.txt
test -f dist/SHA256SUMS
test -f dist/sbom.spdx.json
```

Exception review is intentionally human-gated. Any exception must be recorded with the failing control, affected stage, compensating control, approvers, expiry, and issue or ADR reference before release signoff.

### API compatibility

```bash
pwsh -NoProfile -File docs/design/validate-compatibility-pack.ps1
pwsh -NoProfile -File docs/design/validate-compatibility-pack.ps1 -ReleaseGate
# surface inventory reviewed for Rust crates, C ABI, Arrow schemas, host APIs, and conformance fixtures:
# docs/design/protected-surface-inventory.json
# breaking-change definition present in conductor/contracts/versioning-compatibility.md
# ADR required for protected-surface changes
# migration notes required for breaking changes
# package matrix and catalog must match the live binding/package roots
# compatibility notes must name the affected protected root before beta or later
```

### Red-team / release block review

```bash
# reviews/red-team-report.md is current for the planned release stage
# every critical finding has an owner and follow-up path
# unresolved critical findings block beta, RC, and 1.0
# Track 28 must reference the current package and compatibility claims
# Track 29 must reference the current wave policy and dependency closure claims
# Track 30 must reference the live toolchain matrix and version-drop policy
# Track 31 must reference the current benchmark thresholds and regression workflow
# wave-progression-check and dependency-closure-check gate files must exist
# benchmark-regression-check and threshold-definition-exists must be named for Track 31
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
| Wave manager and execution gatekeeper guidance (Track 29) | yes | yes | yes | yes |
| Toolchain and version support matrix guidance (Track 30) | yes | yes | yes | yes |
| Performance regression guard guidance (Track 31) | partial | yes | yes | yes |
| Benchmark harness public | partial | yes | yes | yes |
| OpenSSF Scorecard workflow | scaffold | active | active | active |
| SBOM attached | optional | dry-run | yes | yes |
| Provenance/attestation | optional | dry-run | yes where supported | yes where supported |
| Red-team review | yes | yes | yes | yes |
| Compatibility promise page | yes | yes | yes | yes |
| Reproducibility docs | partial | yes | yes | yes |
