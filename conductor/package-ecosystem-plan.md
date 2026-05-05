# KairoECS Package Ecosystem Plan

This file expands the package/tooling plan beyond the earlier registry matrix. Treat this as a living dependency inventory; every package adopted into the repo should have an owner, purpose, license review, and replacement strategy.

## Rust workspace crates

| Crate | Purpose | Key dependencies/tools to evaluate | Notes |
|---|---|---|---|
| `kairo-ecs-core` | SimTime, event queue, scheduler, run loop | `thiserror`, `serde`, `tracing`, `proptest`, `criterion`, `loom`, `insta` | No FFI deps; no `unsafe` without ADR |
| `kairo-ecs-state` | Entity/component storage | `slotmap` or generational IDs, `hashbrown`, `indexmap`, `smallvec`, `bitvec`, `serde` | Benchmark custom storage before committing |
| `kairo-ecs-rng` | Deterministic streams | `rand`, `rand_chacha`, `rand_pcg`, `getrandom` | Per-agent stream policy required |
| `kairo-ecs-ffi` | Stable C ABI | `cbindgen`, `libc`, `safer-ffi` or manual ABI, `trybuild` | Panic containment mandatory |
| `kairo-ecs-uniffi` | UniFFI facade | `uniffi`, `uniffi_bindgen` | Use only where bindings are mature enough |
| `kairo-ecs-diplomat` | Diplomat facade | `diplomat`, `diplomat-tool` | Evaluate for TS/C++/other targets |
| `kairo-ecs-arrow` | Telemetry | `arrow-array`, `arrow-schema`, `arrow-ipc`, `parquet` | Version schemas explicitly |
| `kairo-ecs-experiment` | Scenario runner | `clap`, `toml`, `rayon`, `indicatif`, `serde_json` | Avoid hidden global state |
| `kairo-ecs-viz` | Optional visualization | `wgpu`, `bevy` or lightweight renderer, `wasm-bindgen` | Optional; never required by headless core |
| `kairo-ecs-cli` | CLI/dev harness | `clap`, `tracing-subscriber`, `miette`, `serde` | Useful for tests and demos |

## Rust quality and release tooling

| Tool | Purpose |
|---|---|
| `cargo-nextest` | faster, reliable test runner |
| `cargo-llvm-cov` | coverage |
| `cargo-deny` | license/advisory/ban checks |
| `cargo-audit` | RustSec advisory checks |
| `cargo-semver-checks` | Rust API semver checks |
| `cargo-fuzz` | libFuzzer fuzzing |
| `cargo-miri` | UB checks for safe/unsafe assumptions |
| `cargo-mutants` | mutation testing for critical scheduler logic |
| `cargo-machete` | unused dependency detection |
| `cargo-hakari` | workspace dependency unification if needed |
| `release-plz` or `cargo-smart-release` | Rust release automation candidate |

## Python package plan: 3.10-3.14

| Area | Packages/tools |
|---|---|
| Build | `maturin`, `cibuildwheel`, `build`, `twine` |
| Test | `pytest`, `hypothesis`, `pytest-benchmark`, `pytest-xdist` |
| Quality | `ruff`, `pyright` or `mypy`, `pre-commit` |
| Data | `pyarrow`, `numpy`, `pandas`, `polars` |
| Docs | `mkdocs-material` or `sphinx`, `pdoc` |
| Release | PyPI Trusted Publishing, TestPyPI dry runs |

## R package plan

| Area | Packages/tools |
|---|---|
| Build/check | `devtools`, `rcmdcheck`, `usethis` |
| Docs | `roxygen2`, `pkgdown` |
| Test | `testthat`, `covr` |
| Data | `arrow`, `data.table`, `vctrs`, `tibble` |
| Native bridge | `.Call` over C ABI; evaluate `rextendr` only if it does not compromise universal ABI |
| Release | R-universe first; CRAN only after API/ABI maturity |

## Julia package plan

| Area | Packages/tools |
|---|---|
| Build/binary | `Artifacts`, `JLLWrappers`, `BinaryBuilder` when mature |
| Wrapper generation | `Clang.jl` for C headers where helpful |
| Test | `Test`, `Aqua.jl`, `JET.jl` where useful |
| Quality | `JuliaFormatter.jl` |
| Data | `Arrow.jl`, `DataFrames.jl`, `Tables.jl` |
| Docs | `Documenter.jl` |
| Release | Registrator.jl + TagBot after package stabilizes |

## TypeScript/Wasm package plan

| Area | Packages/tools |
|---|---|
| Build | `wasm-bindgen`, `wasm-pack`, `vite`, `tsup` |
| Test | `vitest`, `playwright` for browser smoke tests |
| Quality | `typescript`, `eslint`, `prettier` |
| Docs | `typedoc`, website examples |
| Data | Arrow JS packages, binary IPC examples |
| Release | npm provenance and `npm audit signatures` verification |

## C# package plan: .NET 10-11

| Area | Packages/tools |
|---|---|
| Target frameworks | `net10.0`; `net11.0` preview/allowed-failure until GA |
| Native interop | `SafeHandle`, `LibraryImport`, `NativeLibrary`, source-generated P/Invoke |
| Test | `xUnit` or `NUnit`, `Microsoft.NET.Test.Sdk` |
| Benchmark | `BenchmarkDotNet` |
| Coverage | `coverlet.collector` |
| Quality | .NET analyzers, `dotnet format` |
| Docs | XML docs, DocFX |
| Release | `dotnet pack`, `dotnet nuget push`, NuGet API key or future trusted publishing if available |

## Go package plan

| Area | Packages/tools |
|---|---|
| Interop | cgo wrappers over stable C ABI |
| Test | `testing`, `testify` if useful |
| Benchmark | `go test -bench`, `benchstat` |
| Quality | `gofmt`, `go vet`, `staticcheck`, `golangci-lint` |
| Release | semantic Git tags, `goreleaser` only if distributing binaries |

## Docs/site plan

| Area | Tools |
|---|---|
| Public site | VitePress or Docusaurus |
| Diagrams | Mermaid |
| API docs | docs.rs, pdoc/Sphinx, pkgdown, Documenter.jl, TypeDoc, DocFX, pkg.go.dev |
| Link checks | lychee or markdown-link-check |
| Spelling | codespell or typos |

## Security/supply-chain plan

| Area | Tools |
|---|---|
| GitHub Actions lint | `actionlint`, `zizmor` |
| Dependency updates | Dependabot or Renovate |
| Vulnerability scan | CodeQL, OSV Scanner, cargo-audit |
| License policy | cargo-deny plus manual review for non-Rust ecosystems |
| SBOM | Syft |
| Signing/provenance | Sigstore/cosign, SLSA/in-toto provenance where practical |
| Scorecard | OpenSSF Scorecard GitHub Action |

## Additional packages/tools to evaluate before implementation freeze

### Rust and native library hardening

| Tool/package | Purpose | Adoption rule |
|---|---|---|
| `cargo-vet` | Dependency review/audit trail | Add before beta if dependency graph grows beyond core crates. |
| `cargo-public-api` | Rust public API diffing | Pair with `cargo-semver-checks`. |
| `abi-dumper` / `abi-compliance-checker` | Native ABI diffing on Linux | Use for C ABI release candidates. |
| `cargo-c` | Build/install C ABI libraries and pkg-config metadata | Evaluate for native release packaging. |
| `bindgen` | Generate Rust bindings to C only if needed | Avoid in public API unless justified. |
| `sccache` | CI compilation caching | Use in Rust-heavy matrix. |
| `cargo-zigbuild` | Cross-compilation helper | Evaluate for native artifacts; do not make release-critical until proven. |
| `cross` | Containerized cross builds | Useful for CI smoke tests. |
| `taplo` | TOML formatting/linting | Scenario manifests and Cargo hygiene. |
| `typos` or `codespell` | Spelling checks | Docs and examples. |
| `lychee` | Link checks | Docs release gate. |
| `markdownlint-cli2` | Markdown quality | Docs release gate. |

### Python 3.10-3.14 additions

| Tool/package | Purpose |
|---|---|
| `check-wheel-contents` | Validate wheel contents before upload. |
| `twine check` | Validate distributions before TestPyPI/PyPI. |
| `pytest-cov` | Coverage for Python wrapper code. |
| `abi3audit` | Audit ABI3 wheels if using PyO3 stable ABI strategy. |
| `auditwheel` / `delocate` | Linux/macOS wheel repair for native libs. |
| `nox` | Local multi-version test sessions. |
| `uv` | Fast local/CI Python environment management. |

### R additions

| Tool/package | Purpose |
|---|---|
| `pak` | Fast dependency install in CI. |
| `rhub` | CRAN-like multi-platform checks. |
| `revdepcheck` | Reverse dependency checks once users depend on KairoECS. |
| `goodpractice` | Optional package quality diagnostics. |
| `styler` | Formatting. |

### Julia additions

| Tool/package | Purpose |
|---|---|
| `PkgTemplates.jl` | Julia package scaffolding. |
| `CompatHelper.jl` | Dependency compatibility PRs. |
| `TagBot.jl` | Release tagging. |
| `LocalRegistry.jl` | Internal/dev registry before General. |
| `Scratch.jl` | Temporary data if examples need it. |

### TypeScript/Wasm additions

| Tool/package | Purpose |
|---|---|
| `pnpm` | Workspace/package manager. |
| `changesets` | npm package versioning/changelog. |
| `publint` | npm package correctness. |
| `arethetypeswrong` | Type package validation. |
| `knip` | Unused dependency/export detection. |
| `wasm-opt` | Wasm size/performance optimization. |

### C# .NET 10-11 additions

| Tool/package | Purpose |
|---|---|
| `Microsoft.SourceLink.GitHub` | SourceLink support for NuGet packages. |
| `Nerdbank.GitVersioning` or `MinVer` | Git-derived package versions. |
| `Microsoft.CodeAnalysis.NetAnalyzers` | Analyzer quality gates. |
| `System.CommandLine` | Only for CLI samples/tools, not binding core. |
| `NativeLibrary` + runtime assets | Native library loading and RID-specific packaging. |
| `NuGet/login` | OIDC-based NuGet Trusted Publishing when available for the account. |

### Go additions

| Tool/package | Purpose |
|---|---|
| `govulncheck` | Go vulnerability checks. |
| `go-licenses` | License report. |
| `go tool covdata` / `go test -cover` | Coverage. |
| `goreleaser` | Only if publishing CLI binaries or native artifacts. |

### Docs, governance, and community additions

| Tool/package | Purpose |
|---|---|
| `mermaid-cli` | Render/validate Mermaid diagrams in CI. |
| `Vale` | Prose style checks. |
| `all-contributors` | Recognize community contributions. |
| `OpenSSF Scorecard` | Repository security health checks. |
| `cffconvert` | Validate `CITATION.cff`. |
| `codemetapy` or schema validation | Validate CodeMeta where feasible. |
| `pre-commit` | Local multi-language hygiene. |

## Dependency acceptance rule

A subagent may propose a package, but adoption requires one of these dispositions:

```text
adopt-now        # required for MVP or release safety
adopt-later      # useful but stage-gated
spike-only       # evaluate, do not commit broadly
reject-for-now   # risk/cost exceeds value
```

This prevents the package ecosystem from becoming dependency sprawl.
