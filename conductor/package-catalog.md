# KairoECS Package Catalog

This catalog records likely dependencies, tools, package surfaces, and release tooling so subagents do not make ad-hoc choices.

## Principles

1. Keep hot-path core crates small and auditable.
2. Keep FFI unsafe code isolated in bridge crates.
3. Prefer zero-copy or batch APIs over object-per-event APIs.
4. Prefer ecosystem-native quality tools for each binding.
5. Pin minimum runtime versions and test across the declared support matrix.
6. Every dependency added by a subagent must be justified in an ADR if it affects core, FFI, telemetry schema, or public APIs.

## Current checked-in package surfaces

| Ecosystem | Current manifest | Current package name | Notes |
|---|---|---|---|
| Rust | `Cargo.toml` + `crates/kairo-ecs-types`, `crates/kairo-ecs-core`, `crates/kairo-ecs-state`, `crates/kairo-ecs-rng` | `kairo-ecs-types`, `kairo-ecs-core`, `kairo-ecs-state`, `kairo-ecs-rng` | The workspace is real; the root meta crate and bridge crates remain future track outputs. |
| Python | `bindings/python/pyproject.toml` | `kairo-ecs` / `kairo_ecs` | setuptools-backed package skeleton with `pytest` support and import smoke tests. |
| R | `bindings/r/DESCRIPTION` | `kairoECS` | CRAN-style package skeleton with `testthat`, `NAMESPACE`, and package smoke tests. |
| Julia | `bindings/julia/Project.toml` | `KairoECS` | Package skeleton with `Arrow` and `JSON3` dependencies declared. |
| TypeScript | `bindings/typescript/package.json` | `@kairo-ecs/typescript` | Node package skeleton with local typecheck/test scripts. |
| C# | `bindings/csharp/src/Kairo.ECS/Kairo.ECS.csproj` | `Kairo.ECS` | Package skeleton for `net10.0`. |
| Go | `bindings/go/go.mod` | `github.com/edithatogo/kairos/bindings/go` | Minimal module skeleton with local tests. |

## Rust workspace crates

| Crate | Purpose | Dependency posture |
|---|---|---|
| `kairo-ecs-types` | Shared identifiers, time, errors, DTOs | Minimal, no host-language dependencies |
| `kairo-ecs-core` | Scheduler, virtual time, event queue, run loop | Minimal, deterministic, no FFI |
| `kairo-ecs-state` | Entity/component storage | Minimal SoA/sparse-set design; avoid game-engine dependency in core |
| `kairo-ecs-rng` | Deterministic RNG streams and seed manifests | `rand`, `rand_chacha` or PCG-compatible generator, `rand_distr` |
| `kairo-ecs-des` | Resources, queues, trajectories | Depends on core/types/ecs |
| `kairo-ecs-abm` | Agent behavior semantics | Depends on core/types/ecs/rng |
| `kairo-ecs-arrow` | Arrow telemetry, IPC, Parquet optional | `arrow-*`, `parquet` optional |
| `kairo-ecs-ffi` | Stable C ABI, handles, panic containment | `libc`, `cbindgen`, custom ownership rules |
| `kairo-ecs-uniffi` | UniFFI facade where viable | `uniffi`, generated bindings |
| `kairo-ecs-diplomat` | Diplomat facade where viable | `diplomat` tooling |
| `kairo-ecs-experiment` | Scenario sweeps, replications, manifests | `serde`, `toml`, `rayon` optional, Arrow output |
| `kairo-ecs-viz` | Optional WGPU/Bevy visualization | optional; never a dependency of core |
| `kairo-ecs-cli` | Developer CLI and examples | `clap`, `tracing-subscriber`, `serde_json` acceptable |

The checked-in workspace currently contains `kairo-ecs-types`, `kairo-ecs-core`, `kairo-ecs-state`, and `kairo-ecs-rng`. The root meta crate and the FFI/Arrow/viz/experiment/conformance crates are still planned and should stay out of release promises until they exist. Release promises should only mention the checked-in Rust workspace crates plus any binding package skeletons that already exist under `bindings/`.

## Rust dependency candidates

| Category | Candidate packages/tools | Notes |
|---|---|---|
| Errors | `thiserror` | Library errors only. Avoid `anyhow` in public library APIs. |
| Serialization | `serde`, `serde_json`, `toml` | Core DTOs and manifests; avoid in hot loops if not needed. |
| Collections | `smallvec`, `hashbrown`, `indexmap`, `bitvec` | Use only with measured benefit. |
| IDs/storage | `slotmap`, `generational-arena`, `thunderdome` | Consider custom generational handles if external crates constrain FFI. |
| RNG | `rand`, `rand_chacha`, `rand_distr`, PCG-compatible crate | Determinism matters more than cryptographic strength. |
| Concurrency | `rayon`, `crossbeam` | Experiment runner and future PDES; keep scheduler deterministic. |
| Tracing | `tracing` | Zero-cost when disabled; subscriber only in CLI/tests. |
| Testing | `proptest`, `insta`, `pretty_assertions` | Property and snapshot tests. |
| Benching | `criterion`, `iai-callgrind` optional | Stable benchmark fixtures. |
| Memory/safety | `static_assertions`, `bytemuck`, `zerocopy` | Only when layout invariants are explicit and tested. |
| C ABI | `libc`, `cbindgen`, `cargo-c` | Stable universal backstop. |
| Generated bindings | `uniffi`, `diplomat`, `wasm-bindgen` | Use where viable; do not rely on one generator for every target. |
| Native packaging | `cargo-dist`, `cross`, `cargo-zigbuild` | Evaluate before committing. |
| Version checks | `cargo-semver-checks`, header diffing | Include ABI and API checks. |

## Security/quality tooling

| Tool | Purpose |
|---|---|
| `cargo-nextest` | Fast Rust test runner |
| `cargo-hack` | Feature matrix testing |
| `cargo-deny` | License/advisory/bans policy |
| `cargo-audit` | RustSec advisories |
| `cargo-vet` | Supply-chain review for Rust dependencies |
| `cargo-llvm-cov` | Coverage |
| `cargo-fuzz` | FFI and parser fuzzing |
| `cargo-miri` | Undefined behavior checks where feasible |
| `kani` or `loom` | Optional formal/concurrency checks |
| `cargo-about` | License reports |
| `osv-scanner` | Multi-ecosystem vulnerability scanning |
| `gitleaks` or `trufflehog` | Secret scanning |
| `zizmor` | GitHub Actions security linting |
| `syft` | SBOM generation |
| `grype` | SBOM vulnerability review |

## Python package plan

Target: CPython 3.10, 3.11, 3.12, 3.13, 3.14.

| Area | Tools/packages | Notes |
|---|---|---|
| Build | `maturin`, `cibuildwheel`, `auditwheel`, `delocate` | If PyO3 path is used; otherwise native loader wrapper still needs wheels. |
| Packaging | `pyproject.toml`, `uv`, `hatchling` or `maturin` | Do not mix backends without reason. |
| API tests | `pytest`, `hypothesis` | Hypothesis for conformance/property tests. |
| Type checking | `pyright` or `mypy` | Generate or maintain `.pyi` stubs. |
| Lint/format | `ruff` | One tool for lint and format. |
| Data | `pyarrow`, `numpy`, `pandas`, `polars` optional | `pyarrow` required for telemetry examples. |
| Docs | `pdoc` or `Sphinx`, notebooks | Link from main docs site. |
| Free-threaded checks | Python 3.14 free-threaded lane where runner support exists | Do not assume GIL-based safety. |

## R package plan

| Area | Tools/packages | Notes |
|---|---|---|
| Native binding | `.Call`/C ABI wrapper, `cpp11` optional | Prefer stable C ABI and external pointers. |
| Docs | `roxygen2`, `pkgdown` | Public API docs. |
| Tests | `testthat` | Include conformance fixtures. |
| Quality | `lintr`, `styler`, `R CMD check` | CRAN-style checks even before CRAN. |
| Data | `arrow`, `data.table`, `tibble` optional | Arrow examples first. |
| Release path | R-universe first, CRAN later | CRAN waits for mature native artifact strategy. |

## Julia package plan

| Area | Tools/packages | Notes |
|---|---|---|
| Native loading | `Libdl`, `Artifacts`, JLL package later | Start with Artifacts; JLL after stable ABI. |
| Data | `Arrow.jl`, `Tables.jl`, `DataFrames.jl` | Telemetry integration. |
| Tests | `Test`, `Aqua.jl`, `JET.jl` optional | Conformance fixtures. |
| Docs | `Documenter.jl` | Link from docs site. |
| Formatting | `JuliaFormatter.jl` | CI gate. |
| Release path | GitHub/dev registry, then General Registry | Register only once API is credible. |

## TypeScript/Wasm package plan

| Area | Tools/packages | Notes |
|---|---|---|
| Wasm | `wasm-bindgen`, `wasm-pack`, `wasm-opt` | Browser and Node targets. |
| Build | `pnpm`, `tsup`/`vite`, `rollup` optional | Keep package simple. |
| Tests | `vitest`, `playwright` | Node + browser lanes. |
| Types | `typescript`, `.d.ts`, `typedoc` | Strong typed public API. |
| Lint | `eslint`, `prettier` | Standard hygiene. |
| Data | `apache-arrow` JS package | Arrow event log viewer. |
| Release | npm with provenance where possible | Consider scoped fallback. |

Current package surface: `bindings/typescript/package.json` declares `@kairo-ecs/typescript` and a plain TypeScript test/typecheck loop. Keep Wasm generation as a later enhancement unless the ABI and artifact story are ready.

## C# package plan

Target: `net10.0`.

| Area | Tools/packages | Notes |
|---|---|---|
| Binding | `DllImport`/source-generated P/Invoke, `SafeHandle`, `NativeLibrary` | Safe ownership wrapper is mandatory. |
| Tests | `xUnit` or `NUnit`, `Microsoft.NET.Test.Sdk` | Run conformance fixtures. |
| Benchmarks | `BenchmarkDotNet` | Binding overhead and throughput. |
| Coverage | `coverlet.collector` | CI coverage reports. |
| Docs | XML docs, DocFX | Link from main docs site. |
| Quality | `dotnet format`, analyzers, nullable enabled | Treat warnings as errors for library. |
| Packaging | NuGet with runtime-specific native assets | Include `runtimes/<rid>/native/` libraries. |
| Versioning | `MinVer`, `Nerdbank.GitVersioning`, or release tooling | One consistent release source. |

Note: .NET 10 is the primary stable lane. .NET 11 is a preview/future lane until final GA; it is required for coverage but should not force stable NuGet promises before SDK/runtime production readiness.

## Go package plan

| Area | Tools/packages | Notes |
|---|---|---|
| Binding | cgo over C ABI | Explicit `Close()` and ownership rules. |
| Tests | `go test`, conformance fixtures | Include race detector where feasible, noting cgo caveats. |
| Lint | `gofmt`, `go vet`, `staticcheck`, `golangci-lint` | CI gate. |
| Benchmarks | `go test -bench`, `benchstat` | Binding overhead. |
| Docs | pkg.go.dev | Semantic tags and module docs. |
| Release | semantic Git tags, GoReleaser optional for examples/CLI | Native library distribution strategy must be explicit. |

## Docs/site package plan

| Area | Tools/packages | Notes |
|---|---|---|
| Site | Docusaurus or VitePress | Choose one. Docusaurus is strong for versioned docs; VitePress is simpler. |
| Diagrams | Mermaid, Mermaid CLI | Required for architecture/track/release docs. |
| API docs | rustdoc, pdoc/Sphinx, pkgdown, Documenter.jl, TypeDoc, DocFX, pkg.go.dev | Link from central docs. |
| Link checking | `lychee` | CI gate. |
| Markdown | `markdownlint-cli2` | CI gate. |
| Notebooks | Jupyter, Quarto optional | Examples and tutorials. |

## Model zoo inventory

| Surface | Purpose | Notes |
|---|---|---|
| `docs/community/model-zoo.md` | Public discovery page for example models | Must agree with maturity labels and entry paths. |
| `examples/model-zoo/model-zoo.yaml` | Inventory source of truth | Tracks the concrete example readmes and release labels. |
| `examples/model-zoo/README.md` | Local inventory bridge | Points users from docs to runnable example directories. |
| `examples/des/*`, `examples/abm/*`, `examples/hybrid/*`, `examples/rl/*` | Example entry points | Each directory should have a runnable README and a maturity label. |

## Release/publishing tooling

| Area | Tools/packages | Notes |
|---|---|---|
| Rust release | `release-plz` or `cargo-release` | Choose one; avoid duplicate automation. |
| Multi-artifact release | `cargo-dist` optional | Evaluate for native libs and installers. |
| GitHub releases | `softprops/action-gh-release` or GitHub CLI | Draft releases until gates pass. |
| Python wheels | `maturin`, `cibuildwheel` | TestPyPI before PyPI. |

## Actionable release starters

The following are the first non-destructive steps each ecosystem should support before any production publish is attempted:

| Ecosystem | Starter work | Dry-run or local check | Notes |
|---|---|---|---|
| Rust | Keep the workspace crates checked in and gate any new root/bridge crate on naming approval | `cargo check --workspace`, `cargo test --workspace`, `cargo package --allow-dirty --manifest-path crates/kairo-ecs-core/Cargo.toml` | Keep `unsafe` isolated and verify crate naming early. |
| Python | Package the checked-in `bindings/python/pyproject.toml` surface | `python -m build`, `twine check dist/*`, `pytest`, `python -c "import kairo_ecs; print(kairo_ecs.self_check())"` | Prefer a single backend path and keep Python 3.10-3.14 visible. |
| R | Package the checked-in `bindings/r` surface | `R CMD build .`, `R CMD check --no-manual .`, `Rscript -e "testthat::test_dir('tests', reporter = 'summary')"` | Start with GitHub/R-universe style distribution and a CRAN-style package check. |
| Julia | Package the checked-in `bindings/julia` surface | `julia --project -e "using Pkg; Pkg.test()"`, `julia --project -e "using Pkg; Pkg.build()"` | Keep General Registry and JLL as later-stage work. |
| TypeScript | Package the checked-in `bindings/typescript` surface | `npm ci`, `npm run typecheck`, `npm test`, `npm pack` | Keep scoped package naming explicit and treat `dist/` as the publish tree. |
| C# | Package the checked-in `bindings/csharp` surface | `dotnet test bindings/csharp/tests/Kairo.ECS.Tests/Kairo.ECS.Tests.csproj`, `dotnet pack bindings/csharp/src/Kairo.ECS/Kairo.ECS.csproj` | Keep .NET 11 as preview until the runtime lane is confirmed. |
| Go | Package the checked-in `bindings/go` surface | `go test ./...`, `go vet ./...`, `gofmt -w .` | Keep the module path explicit before any tag plan. |

## Registry notes by ecosystem

- Rust: reserve `kairo-ecs` family names before the first crates.io publish.
- Python: treat TestPyPI as the first external check.
- R: use GitHub or R-universe first; delay CRAN until the package check, native loading, and artifact strategy are stable.
- Julia: keep GitHub/dev registry first and defer JLL until ABI is stable.
- TypeScript: keep npm package names scoped and build a draft `npm pack` flow first.
- C#: keep NuGet prerelease packaging separate from stable promises.
- Go: use semantic version tags only after the module path and native library plan are fixed.

## Release artifact expectations

Every ecosystem should be able to point to a concrete release artifact set before a public write:

| Ecosystem | Expected release artifacts |
|---|---|
| Rust | source tarball, checksums, SBOM, provenance/attestations, changelog, GitHub Release draft |
| Python | sdist, wheel(s), metadata check output, checksums, SBOM where applicable |
| R | package source tarball, `R CMD check` output, package docs, checksums, SBOM where applicable |
| Julia | package source tree, artifact metadata, docs build output, checksums |
| TypeScript | `dist/` bundle, `npm pack` tarball, typecheck/test output, checksums |
| C# | NuGet package, runtime asset layout, pack/test output, checksums |
| Go | module source tree, tag/release notes, test/vet output, checksums |
| npm | `npm publish`, provenance if available | Scoped fallback if name collision. |
| NuGet | `dotnet pack`, `dotnet nuget push` | Pre-release package until stable. |
| SBOM | `syft`, `cargo-about` | Attach to release. |
| Signing | GitHub artifact attestations, Sigstore/cosign | Use keyless where suitable. |
| Changelog | `release-please`, `git-cliff`, or release-plz changelog | Choose a single source of release truth. |

## Dependency approval tiers

| Tier | Criteria | Review |
|---|---|---|
| Tier 0 | Core hot path or FFI unsafe boundary | ADR + benchmark + security review |
| Tier 1 | Public API or telemetry schema | ADR + compatibility review |
| Tier 2 | Binding-only package | Binding owner + conformance review |
| Tier 3 | Docs/examples/dev-only | Lightweight review |
