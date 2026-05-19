# KairoECS Release Engineering

## Release model

KairoECS has four compatibility surfaces:

```text
1. Rust crate API
2. C ABI
3. Host-language APIs
4. Arrow telemetry schemas
```

A release is only stable if all four surfaces have documented compatibility expectations.

The current checked-in package surfaces are the Rust crates in `crates/` that participate in the manifest-backed release inventory (`kairo-ecs-types`, `kairo-ecs-core`, `kairo-ecs-state`, `kairo-ecs-rng`, `kairo-ecs-bench`, `kairo-ecs-des`, `kairo-ecs-abm`, `kairo-ecs-arrow`, `kairo-ecs-ffi`, `kairo-ecs-uniffi`, `kairo-ecs-diplomat`, `kairo-ecs-cli`, `kairo-ecs-gpu`, `kairo-ecs-webgpu`, `kairo-ecs-pdes`, `kairo-ecs-mpi`, `kairo-ecs-grpc`, `kairo-ecs-streaming`, `kairo-ecs-ml`, `kairo-ecs-fmi`, `kairo-ecs-debug`, `kairo-ecs-viz`, `kairo-ecs-wasm`, `kairo-ecs-cs-bridge`), plus the Python package in `bindings/python/`, the R package in `bindings/r/`, the Julia package in `bindings/julia/`, the TypeScript package in `bindings/typescript/`, the C# package in `bindings/csharp/`, and the Go module in `bindings/go/`. Ignored local draft satellites such as `kairo-ecs-pyo3` are not release surfaces until tracked deliberately and assigned their own packaging gate.

## Early-stage policy

- This track plans release mechanics before production publishing.
- Draft releases and dry-runs are the default.
- Production registry writes wait until registry names, toolchains, and package metadata are verified.
- Any public API or schema change that affects downstream packages must be recorded in the packaging docs first.

## Offline dry-run order

Track 15 uses the manifest-backed local dry-run sequence in `packaging/release-package-manifest.json`:

1. Validate the workspace and binding package inventory.
2. Generate local release evidence in `dist/release-artifact-manifest.json` and `dist/SHA256SUMS`.
3. Validate the offline dry-run gate.

The Rust inventory in that manifest includes `kairo-ecs-cs-bridge` alongside the other manifest-backed workspace crates, while `kairo-ecs-pyo3` remains an ignored local draft satellite outside the release-manifest inventory until its own packaging gate is approved.

## Versioning

Use SemVer at the product level, with explicit schema/ABI versions:

```text
KairoECS package version: 0.4.0
C ABI version: kairo_ecs_ffi_abi_v1
Arrow event schema: kairo_ecs.event_log.v1
Conformance suite: kairoecs.conformance.v1
```

## Registry ladder

| Ecosystem | First step | Second step | Stable path |
|---|---|---|---|
| Rust | `cargo package` and `cargo publish --dry-run` | crates.io draft readiness | crates.io publish + GitHub Release |
| Python | Build wheel/sdist and validate metadata | TestPyPI upload dry-run | PyPI publish |
| R | Build package and check native loading | GitHub release or R-universe artifact build | CRAN later |
| Julia | Validate package metadata and artifacts | GitHub/dev registry prep | General Registry later |
| TypeScript | `npm pack` and type validation | npm publish dry-run | npm publish |
| C# | `dotnet pack` and local package inspection | NuGet prerelease validation | NuGet publish |
| Go | `github.com/edithatogo/kairos/bindings/go` module path and version plan | semantic tag rehearsal | semantic tag and module release |

## Production registry publication model

Production registry publication is owned by Track 42 for language/package registries and Track 43 for cloud/HPC registries. Track 15 remains the offline dry-run and artifact-inventory base; it does not authorize public writes by itself.

Publication workflows must use the strongest current registry mechanism available:

- PyPI/TestPyPI: trusted publishing from GitHub Actions with `id-token: write` and a protected environment.
- npm: trusted publishing or `npm publish --provenance` from GitHub Actions with OIDC/provenance enabled.
- NuGet: trusted publishing where available; token fallback must be environment-gated and short-lived.
- crates.io: trusted publishing when generally available for the crate owner; otherwise use the narrowest scoped token in a protected environment and keep dry-run as the default.
- R-universe/CRAN: source package checks and release-manager approval before publication.
- Julia registry: registry PR/dev-registry path first, General Registry only after API/native-artifact maturity.
- Go module proxy: semantic tags only after release-manager approval and compatibility gate signoff.
- OCI/container registries: digest-pinned image publication with SBOM, signature/attestation, and rollback record.

Every public write requires a code/repo health score of at least `9.5/10`, recorded by Track 44, plus release-manager approval in the target GitHub environment.

## Release train

```mermaid
flowchart LR
    Dev[main branch]
    RC[release/x.y branch]
    DryRun[Registry dry-runs]
    Conformance[Full conformance matrix]
    Docs[Docs version build]
    Security[SBOM + scans + checksums]
    GH[GitHub Release candidate]
    Registries[Publish registries]
    Announce[Release notes + docs]

    Dev --> RC
    RC --> DryRun
    RC --> Conformance
    RC --> Docs
    RC --> Security
    DryRun --> GH
    Conformance --> GH
    Docs --> GH
    Security --> GH
    GH --> Registries
    Registries --> Announce
```

## Required release artifacts

For RC and 1.0, the artifact tree must include `RELEASE.txt`, `SHA256SUMS`, `sbom.spdx.json`, and the ecosystem artifacts below.

```text
source tarball
checksums
SBOM
native libraries
C headers
Python wheels/sdist
R package artifact
Julia artifact metadata
npm package
NuGet package
Go tag
API docs
GitHub Pages docs version
changelog
migration guide if breaking changes
```

## Supply-chain gates

A release is not eligible for beta, RC, or 1.0 unless these checks are present and green:

- `.github/workflows/scorecard.yml`
- `.github/workflows/dependency-review.yml`
- `.github/workflows/sbom-attestations.yml`
- `.github/workflows/release-attestations.yml`
- `SECURITY.md`
- `CODEOWNERS`
- `conductor/quality-gates.md` supply-chain gate section

Waivers and exceptions are recorded in `conductor/quality-gates.md` and must state:

1. the exact missing or failing check
2. the release stage affected
3. the approver
4. the expiry or follow-up issue

Temporary toolchain gaps can be allowed-failure only for alpha. They do not create a standing policy waiver.

## Citation and archive requirements

Before any public release write, the release note must include:

- the exact release version
- the citation files used for the release: `CITATION.cff`, `codemeta.json`, `.zenodo.json`
- the archive status: draft, sandbox, or DOI-minted
- the DOI or draft Zenodo link if one exists
- the source archive location
- reproducibility instructions
- any author, version, repository-code, or license changes

The citation metadata path is:

1. Update `CITATION.cff`, `codemeta.json`, and `.zenodo.json`.
2. Record the archive note in the release note.
3. Use a Zenodo draft or sandbox deposition first.
4. Mint the DOI only after the archive record and release note are complete.
5. Keep the release note and archive metadata in sync before any public write.

## Actionable release checklist

1. Confirm package names and fallback names in `conductor/package-matrix.md`.
2. Confirm the first registry target for each ecosystem.
3. Record the dry-run command for each current package surface in the package catalog.
4. Keep GitHub Releases draft-only until all package checks pass.
5. Keep registry credentials out of the repo and use the least-privileged release mechanism available.
6. Add release notes, citation metadata references, and compatibility notes before any public package write.
7. Run Track 42 and Track 44 validators before any language registry write.
8. Run Track 43 and Track 44 validators before any cloud/HPC registry write or production-ready cloud/HPC claim.

## Pre-release policy

Use pre-release tags for binding surfaces before 1.0:

```text
0.4.0-alpha.1
0.4.0-beta.1
1.0.0-rc.1
```

## Maintenance windows

| Surface | Minimum support policy |
|---|---|
| Rust | stable Rust + stated MSRV |
| Python | 3.10-3.14 initially; remove EOL versions only in minor/major releases with notice |
| C# | .NET 10-11 target as requested; revise when runtime lifecycle requires |
| Arrow schemas | backward-compatible additive changes within same major schema version |
| FFI ABI | no breaking ABI changes without major version or side-by-side ABI |

## Release checklist

See `docs/release/release-checklist.md`.
The release checklist should remain compatible with the early-stage draft-only policy above.
