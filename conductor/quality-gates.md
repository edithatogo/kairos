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

### Central required-gate catalogue

Every gate listed in `conductor/tracks.yaml` must appear here as a bold gate ID. Track agents may add stricter track-local commands, but central status cannot move a track to `In Review`, `Done`, beta, RC, or 1.0 unless each required gate has either passed evidence or an explicit waiver with owner, expiry, and follow-up.

**metadata-check**: validates project metadata, license files, maintainer/governance files, and Conductor setup references.

**naming-due-diligence**: validates naming due-diligence and release-stage naming evidence before public package claims.

**cargo-fmt**: `cargo fmt --all --check`.

**cargo-clippy**: `cargo clippy --workspace --all-targets --all-features -- -D warnings`.

**cargo-test**: `cargo test --workspace` or the narrower track-approved workspace test command when full workspace testing is blocked with a documented reason.

**deterministic-ordering-fixture**: validates the deterministic ordering conformance fixture and any scheduler code that consumes it.

**ffi-lifecycle-tests**: validates FFI handle lifecycle, allocation/free, and double-free guard behavior for the checked-in FFI slice.

**panic-boundary-tests**: validates panic containment and error reporting across FFI boundary code.

**header-diff**: validates that generated or maintained C headers match the checked-in public header contract.

**arrow-roundtrip**: validates Arrow schema/event-log roundtrip behavior for the relevant crate or binding surface.

**pytest**: runs the Python package test suite when `bindings/python` is in scope.

**ruff**: runs Python lint/format checks for `bindings/python`.

**wheel-build**: builds or dry-runs the Python wheel surface without publishing.

**r-cmd-check**: runs the R package check or the repository's local static equivalent when R tooling is unavailable and the handoff records the limitation.

**julia-pkg-test**: runs the Julia package tests or the repository's local static equivalent when Julia tooling is unavailable and the handoff records the limitation.

**tsc**: runs TypeScript type checking without emitting artifacts.

**vitest**: runs the TypeScript test suite.

**browser-smoke**: validates browser/demo behavior through the checked-in smoke harness for the relevant website or Wasm surface.

**dotnet-test-net10**: runs .NET 10 tests for the C# binding surface.

**dotnet-build-net10**: runs the .NET 10 build for the C# binding surface without publishing artifacts.

**dotnet-test-net11-preview**: validates the .NET 11 preview lane only as experimental until GA support is documented.

**nuget-pack**: validates NuGet package packing without publishing.

**go-test**: runs Go package tests for the Go binding surface.

**go-vet**: runs `go vet` for the Go binding surface.

**cgo-smoke**: validates the Go/native bridge status boundary without claiming stable native FFI unless artifacts exist.

**fixture-schema-check**: validates conformance fixture JSON shape and manifest coverage.

**benchmark-smoke**: validates benchmark metadata or target compilation without claiming stable comparative timings unless result artifacts exist.

**workflow-presence**: validates required GitHub workflow files exist and do not silently skip concrete R2 surfaces.

**cargo-metadata**: validates Rust workspace metadata is readable and includes checked-in crates.

**dependency-policy**: validates dependency policy files and workflow references.

**docs-build**: builds the docs site or runs the repository's documented static docs build.

**link-check-plan**: validates docs link-check coverage through `website/docs-link-manifest.json` and local link checks.

**package-dry-run**: validates package inventory/dry-run output without publishing.

**checksums**: validates checksum manifest generation for release artifacts when artifacts exist.

**release-checklist**: validates release checklist coverage and stage-specific blockers.

**compatibility-policy**: validates release compatibility policy and protected-surface references.

**changelog-check**: validates changelog presence and release-note readiness.

**onboarding-docs**: validates contributor/community onboarding docs and entry points.

**benchmark-metadata**: validates benchmark plan, fixture IDs, seeds, and measurement metadata.

**raw-results-policy**: validates that raw benchmark-result requirements are documented before performance claims.

**citation-metadata**: validates citation metadata files and no-fake-DOI boundaries.

**archival-plan**: validates archive/release metadata and the publication handoff path.

**scorecard**: validates OpenSSF Scorecard workflow presence and staged evidence requirements.

**sbom-plan**: validates SBOM/provenance workflow presence and release-stage artifact requirements.

**vulnerability-policy**: validates vulnerability reporting, dependency review, and exception handling.

**replay-fixture**: validates replay fixture linkage and documented reproducibility limits.

**seed-manifest**: validates seed/replay manifest coverage for uncertainty and validation claims.

**scenario-manifest**: validates scenario manifest shape and concrete fixture references.

**resumability-plan**: validates scenario resume/replay handling and output-shape documentation.

**example-maturity-labels**: validates starter-kit/model-zoo maturity labels and concrete example paths.

**non-core-dependency-check**: validates optional/demo dependencies do not leak into core workspace requirements.

**api-review-template**: validates API review intake and protected-surface review templates.

**compatibility-matrix**: validates compatibility matrix coverage for current Rust, C ABI, Arrow, host API, and binding roots.

**standards-mapping**: validates interoperability mapping rows and status labels.

**adr-recommendations**: validates ADR recommendations for interoperability or protected-surface decisions.

**bootstrap-smoke**: validates local bootstrap/developer workflow commands for the current R2 setup.

**toolchain-docs**: validates toolchain documentation and environment setup references.

**no-critical-release-blockers**: validates red-team blocker ledger state and release-stage owner/follow-up coverage.

**wave-progression-check**: `pwsh -NoProfile -File conductor/tracks/29-wave-manager-execution-gatekeeper/validate-wave-gates.ps1` and `conductor/gates/wave-progression-check.yml` must derive wave eligibility from `conductor/tracks.yaml`.

**dependency-closure-check**: `pwsh -NoProfile -File conductor/tracks/29-wave-manager-execution-gatekeeper/validate-wave-gates.ps1` and `conductor/gates/dependency-closure-check.yml` must block review/done claims when dependencies are not sufficiently mature or explicitly waived.

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

### Track 35 distributed simulation gates

**distributed-state-parity**: `pwsh -NoProfile -File conductor/tracks/35-distributed-simulation-mpi-grpc/validate-track35.ps1` - validates the dependency-free MPI/gRPC protocol emulators and track docs boundary while keeping real two-node parity as future work.

**entity-migration-integrity**: `pwsh -NoProfile -File conductor/tracks/35-distributed-simulation-mpi-grpc/validate-track35.ps1` - validates the entity-migration protocol scaffold, envelope checks, and placeholder transport boundaries; real byte-level runtime migration remains future work.

**grpc-fault-tolerance**: `pwsh -NoProfile -File conductor/tracks/35-distributed-simulation-mpi-grpc/validate-track35.ps1` - validates the gRPC failure-classification scaffold and keeps the non-leader worker failure story explicit until real transport wiring exists.

**distributed-telemetry-merge**: `pwsh -NoProfile -File conductor/tracks/35-distributed-simulation-mpi-grpc/validate-track35.ps1` - validates the distributed telemetry protocol scaffold and keeps Arrow batch merge claims limited to the documented transport emulator.

### Tracks 36-40 streaming, ML, FMI, cloud/HPC, and debugging gates

**kafka-smoke**: `cargo test -p kairo-ecs-streaming --features kafka` - validates the dependency-free Kafka feature surface and adapter type alias exposed by the Track 36 R2 scaffold. It does not prove a live Kafka broker connection until a broker-backed integration harness is added.

**arrow-flight-smoke**: `cargo test -p kairo-ecs-streaming --features arrow-flight` - validates the Arrow Flight feature surface and event-log stream contract exposed by the Track 36 R2 scaffold. It does not prove an Arrow Flight server/client runtime.

**realtime-wallclock-check**: `cargo test -p kairo-ecs-streaming --no-default-features` - validates event-log message shape, wall-clock pacing contracts, and no-adapter operation for the Track 36 R2 scaffold. It does not claim production latency or broker throughput.

**onnx-inference-smoke**: `cargo test -p kairo-ecs-ml --features onnx` and `cargo check --manifest-path examples/ml-surrogate/de-surrogate/Cargo.toml --features onnx` - validates model metadata, tensor shape checks, deterministic passthrough inference, and the ONNX feature alias in the Track 37 R2 scaffold. It does not prove ONNX Runtime execution or model accuracy.

**gymnasium-env-smoke**: `cargo test -p kairo-ecs-ml --features gymnasium` plus the local `python/kairo_gym` tests when Python test dependencies are available - validates Rust-side action/space contracts and the Python environment contract scaffold. It does not prove RL training-loop performance.

**fmi-import-smoke**: `cargo check --manifest-path crates/kairo-ecs-fmi/Cargo.toml --features fmi2 --tests` and `cargo check --manifest-path examples/fmi-co-simulation/basic-import/Cargo.toml` - validates unpacked FMU layout and FMI 2 import contract compilation for Track 38 R2. It does not prove shared-library FMU execution.

**fmi-export-smoke**: `cargo check --manifest-path crates/kairo-ecs-fmi/Cargo.toml --all-features --tests` - validates modelDescription generation and unpacked export-layout checks for Track 38 R2. It does not prove full `.fmu` archive packaging or third-party tool roundtrip.

**aas-validate**: `cargo check --manifest-path crates/kairo-ecs-fmi/Cargo.toml --features aas --tests` - validates dependency-free AAS descriptor/submodel structural checks for Track 38 R2. It does not prove AASX Package Explorer schema conformance.

**docker-build**: `python cloud/validate_cloud_hpc.py` plus `docker build` when Docker is available - validates Track 39 Dockerfile and offline cloud/HPC manifest scaffolds. The Python validator is the portable R2 gate; live container execution remains a later gate.

**kubernetes-smoke**: `python cloud/validate_cloud_hpc.py` - validates the Kubernetes CRD/sample/operator manifest shape for Track 39 R2. It does not prove a live cluster, controller deployment, or pod lifecycle.

**spot-checkpoint-test**: `python cloud/validate_cloud_hpc.py` - validates checkpoint/resume script and telemetry-output scaffold contracts for Track 39 R2. It does not prove cloud spot/preemptible interruption handling.

**trace-record-replay**: `cargo test -p kairo-ecs-debug` and `node website/time-travel-demo/validate-demo.mjs` - validates trace-line encoding, fixture loading, reconstruction, and replay-oriented offline demo checks for Track 40 R2. It does not prove Arrow IPC trace storage or live record/replay integration.

**fwd-back-parity**: `cargo test -p kairo-ecs-debug` - validates forward/backward stepping and tick seek behavior over the offline trace model for Track 40 R2. It does not prove large-trace performance.

**breakpoint-smoke**: `cargo test -p kairo-ecs-debug` - validates event-kind breakpoint matching and CLI command names for Track 40 R2. It does not prove interactive debugging of a running simulation.

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
| Client-side search | no | yes | yes | yes |
