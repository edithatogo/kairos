# KairoECS Package and Registry Matrix

## Naming policy

The public project name is **KairoECS**. Public package names should use the distinctive `kairo-ecs` / `Kairo.ECS` / `KairoECS.jl` family rather than a bare `kairo` or `kairos` name.

Before the first public release, re-check every registry manually and reserve names where appropriate. Treat name availability as a release blocker, not an assumption.

## Preferred package names

| Ecosystem | Preferred name | Notes / fallbacks |
|---|---|---|
| Rust root crate | `kairo-ecs` | Thin meta crate that re-exports stable user-facing Rust APIs. |
| Rust internal crates | `kairo-ecs-types`, `kairo-ecs-core`, `kairo-ecs-state`, `kairo-ecs-rng`, `kairo-ecs-ffi`, `kairo-ecs-arrow`, `kairo-ecs-viz`, `kairo-ecs-experiment`, `kairo-ecs-conformance` | The checked-in workspace currently ships `kairo-ecs-types`, `kairo-ecs-core`, `kairo-ecs-state`, and `kairo-ecs-rng`; the root meta crate remains reserved. |
| Python distribution | `kairo-ecs` | Import as `kairo_ecs`. |
| Python import | `kairo_ecs` | Never require `import kairo` or `import kairoecs`. |
| R package | `kairoECS` | Use camel-case R package name if accepted by the release channel. |
| Julia package | `KairoECS.jl` | Module name `KairoECS`. |
| TypeScript / npm | `@kairo-ecs/typescript` | Current checked-in package surface. Future `@kairo-ecs/core`, `@kairo-ecs/arrow`, and `@kairo-ecs/viz` packages can split off later if needed. |
| C# / NuGet | `Kairo.ECS` | Future packages may use `Kairo.ECS.Native`, `Kairo.ECS.Arrow`, etc. |
| Go module | `github.com/edithatogo/kairos/bindings/go` | Current checked-in module path. Keep the local package name simple and explicit. |
| C ABI library | `libkairo_ecs` | Header: `kairo_ecs.h`; function prefix: `kairo_ecs_`. |
| CLI | `kairoecs` | CLI can be a compact command while packages remain explicit. |
| Docs domain | `kairo-ecs.dev` or `kairo-ecs.org` | Verify availability and trademark risk. |

## Current checked-in package surfaces

| Ecosystem | Manifest path | Declared package name | Current release posture |
|---|---|---|---|
| Rust | `Cargo.toml` + `crates/kairo-ecs-types`, `crates/kairo-ecs-core`, `crates/kairo-ecs-state`, `crates/kairo-ecs-rng` | `kairo-ecs-types`, `kairo-ecs-core`, `kairo-ecs-state`, `kairo-ecs-rng` | Workspace crates are checked in; the root meta crate and FFI/Arrow layers remain planned. |
| Python | `bindings/python/pyproject.toml` | `kairo-ecs` / `kairo_ecs` | Package skeleton is checked in and ready for wheel/sdist dry-runs. |
| R | `bindings/r/DESCRIPTION` | `kairoECS` | Package skeleton is checked in and ready for `R CMD build` / `R CMD check`. |
| Julia | `bindings/julia/Project.toml` | `KairoECS` | Package skeleton is checked in and ready for `Pkg.test`. |
| TypeScript | `bindings/typescript/package.json` | `@kairo-ecs/typescript` | Package skeleton is checked in and ready for `npm pack` and local Node tests. |
| C# | `bindings/csharp/src/Kairo.ECS/Kairo.ECS.csproj` | `Kairo.ECS` | Package skeleton is checked in and ready for `dotnet pack` and `dotnet test`. |
| Go | `bindings/go/go.mod` | `github.com/edithatogo/kairos/bindings/go` | Module skeleton is checked in and ready for `go test` and `go vet`. |

## Publishing stages

| Stage | Goal | Registries |
|---|---|---|
| 0.1 alpha | Rust core + C ABI preview | crates.io optional, GitHub Releases |
| 0.2 alpha | Python + Arrow telemetry preview | TestPyPI/PyPI, GitHub Releases |
| 0.3 alpha | TypeScript/Wasm + docs site | npm, GitHub Pages |
| 0.4 beta | R/Julia/C#/Go previews | R-universe, Julia dev registry, NuGet pre-release, Go tags |
| 0.5 beta | Full conformance suite required | all test registries |
| 1.0 | Stable ABI/schema/API baseline | production registries |

## Early release order

The first release wave should stay narrow and non-destructive:

1. Rust root crate plus C ABI preview.
2. Python preview against the Rust/C ABI layer.
3. TypeScript/Wasm preview if the C ABI and artifact layout are stable enough.
4. R and Julia preview packages using the shared artifact and conformance story.
5. C# preview package with native runtime assets.
6. Go preview module over the stable C ABI.

The governance/control wave follows the package wave:

1. Wave Manager & Execution Gatekeeper keeps release ordering honest.
2. Toolchain & Version Support Matrix keeps the supported-version floor explicit.
3. Performance Regression Guard keeps benchmark drift visible before release claims are made.

## Ecosystem action map

| Ecosystem | First artifact | First validation | First registry target |
|---|---|---|---|
| Rust | checked-in workspace crates (`kairo-ecs-types`, `kairo-ecs-core`, `kairo-ecs-state`, `kairo-ecs-rng`) | `cargo check --workspace`, `cargo test --workspace`, `cargo package --allow-dirty --manifest-path crates/kairo-ecs-core/Cargo.toml`, `cargo publish --dry-run` | crates.io draft readiness, then GitHub Releases |
| Python | `kairo-ecs` wheel/sdist | `python -m build`, `twine check dist/*`, `pytest`, `python -c "import kairo_ecs; print(kairo_ecs.self_check())"` | TestPyPI first, PyPI later |
| R | `kairoECS` package skeleton | `Rscript -e "testthat::test_dir('tests', reporter = 'summary')"`, `R CMD build .`, `R CMD check --no-manual .` | GitHub release or R-universe first |
| Julia | `KairoECS.jl` package skeleton | `julia --project -e "using Pkg; Pkg.test()"`, `julia --project -e "using Pkg; Pkg.build()"` | GitHub/dev registry first |
| TypeScript | `@kairo-ecs/typescript` package skeleton | `npm ci`, `npm run typecheck`, `npm test`, `npm pack` | npm first |
| C# | `Kairo.ECS` package skeleton | `dotnet test bindings/csharp/tests/Kairo.ECS.Tests/Kairo.ECS.Tests.csproj`, `dotnet pack bindings/csharp/src/Kairo.ECS/Kairo.ECS.csproj` | NuGet prerelease first |
| Go | module skeleton at the chosen module path | `go test ./...`, `go vet ./...`, `gofmt -w .` | semantic tag planning only |

## Package artifact checklist

Every package must include:

```text
README
LICENSE
version
changelog link
API docs link
examples
native library loading strategy
platform support table
minimum runtime version
conformance test status
security reporting link
```

Release validation should also prove:

- the checked-in package manifest exists for the ecosystem
- the dry-run command runs from the package root
- the package name matches the registry plan
- the release artifact tree is explicit and reproducible
- SBOM/provenance or equivalent attestation exists where the ecosystem supports it

## Platform artifacts

| Platform | Native artifacts |
|---|---|
| Linux x86_64 | `.so`, wheels, npm wasm/node artifacts, NuGet runtime asset |
| Linux aarch64 | `.so`, wheels where supported, NuGet runtime asset |
| macOS x86_64 | `.dylib`, wheels, NuGet runtime asset |
| macOS arm64 | `.dylib`, wheels, NuGet runtime asset |
| Windows x86_64 | `.dll`, wheels, NuGet runtime asset |
| Wasm | `.wasm`, `.js`, `.d.ts` |

## Registry-specific notes

### Python

Target Python 3.10-3.14. Prefer `abi3-py310` if feasible. Otherwise build per Python version.

### C#

Target `net10.0`.

### R

Start with R-universe or GitHub releases. CRAN should wait until native artifact strategy, reverse dependency checks, and `R CMD check --no-manual .` are mature.

### Julia

Start with GitHub + Artifacts. Move to General Registry and JLL packaging after ABI stability.

### Go

Release using semantic Git tags. Keep cgo instructions explicit.

## Registry readiness checklist

- Reserve names before the first production publish.
- Record the first registry target and the fallback target for each ecosystem.
- Keep dry-run commands in the release notes and CI plan.
- Record minimum runtime versions in the package catalog.
- Keep GitHub Releases draft-only until the release gates are satisfied.
- Record the release artifact tree for each ecosystem before a public write.
- Keep the control-track documents current before release claims are made.

## Detailed dependency/tooling inventory

See `conductor/package-ecosystem-plan.md` for crate-level, binding-level, documentation, security, and release tooling plans.

Important assumptions to verify before public release:

- Python support targets 3.10, 3.11, 3.12, 3.13, and 3.14.
- C# support targets .NET 10 as stable and .NET 11 as preview/allowed-failure until GA.
- Registry release workflows should prefer OIDC/provenance when supported and use scoped tokens only where no better mechanism exists.
