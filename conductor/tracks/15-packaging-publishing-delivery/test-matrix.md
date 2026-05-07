# Test Matrix — 15 Packaging, Publishing & Delivery

## Required checks

- Package matrix coverage for Rust, Python, R, Julia, TypeScript, C#, and Go.
- Registry plan coverage for each ecosystem's first target and fallback.
- Dry-run coverage for every ecosystem that supports packaging locally.
- Docs coverage for any change to package naming, registry order, or release policy.
- No-production-publish check: the track must not introduce live publish commands.
- No-publish-manifest check: the first local sequence must not add publish or publication manifest files.
- Aggregate Track 12-20 evidence check: the package manifest remains dry-run only and wired into conformance CI.

## Track-specific commands

```bash
rg -n "Rust|Python|R|Julia|TypeScript|C#|Go" conductor/package-matrix.md conductor/package-catalog.md conductor/release-engineering.md
rg -n "dry-run|draft only|preview|reservation|fallback" conductor/package-matrix.md conductor/release-engineering.md
python packaging/scripts/build_release_manifest.py --check
python packaging/scripts/build_release_manifest.py --version 0.0.0-r2-dry-run
powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/15-packaging-publishing-delivery/validate-packaging-dry-run.ps1
node tests/conformance/track12_20_evidence_check.mjs
```

## R2 dry-run matrix

| Ecosystem | Manifest inventory | Registry target | Dry-run evidence |
|---|---|---|---|
| Rust crates | `Cargo.toml` and `crates/*/Cargo.toml` listed in `packaging/release-package-manifest.json` | crates.io | `cargo package --allow-dirty --workspace`; `cargo publish --dry-run --workspace` |
| Python binding | `bindings/python/pyproject.toml` | TestPyPI | `python -m build`; `twine check dist/*` |
| R binding | `bindings/r/DESCRIPTION` | R-universe first | `R CMD build .`; `R CMD check --no-manual --as-cran *.tar.gz` |
| Julia binding | `bindings/julia/Project.toml` | Julia dev registry first | `julia --project=. -e "using Pkg; Pkg.instantiate(); Pkg.test()"` |
| TypeScript/Wasm binding | `bindings/typescript/package.json` | npm | `npm ci`; `npm run typecheck`; `npm test`; `npm pack --dry-run` |
| C# binding | `bindings/csharp/src/Kairo.ECS/Kairo.ECS.csproj` and `bindings/csharp/Kairo.ECS.sln` | NuGet | `dotnet restore`; `dotnet test`; `dotnet pack` |
| Go binding | `bindings/go/go.mod` | Go module proxy | `go test ./...`; `go vet ./...` |

The manifest/checksum builder is the local validation gate for this slice. It
does not execute registry commands; it verifies that the package inventory,
registry modes, and checksum evidence can be generated before publishing is
enabled.

## Focused offline validator

`validate-packaging-dry-run.ps1` verifies the seven ecosystem surfaces, dry-run
release stage, disabled production publishing flag, fallback entries, manifest
paths, expected release evidence output paths, the ordered local dry-run
sequence, and the absence of publish manifest files.

## First local sequence

1. Validate inventory without writes:
   `python packaging/scripts/build_release_manifest.py --check`
2. Generate local release evidence only:
   `python packaging/scripts/build_release_manifest.py --version 0.0.0-r2-dry-run`
3. Re-run the offline gate:
   `powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/15-packaging-publishing-delivery/validate-packaging-dry-run.ps1`

This sequence is the first local registry/package dry-run. It does not contact
registries, use credentials, upload artifacts, publish packages, or create
publish manifests.

## Registry checks to land later

- `cargo publish --dry-run`
- `python -m build`
- `twine check`
- `npm pack`
- `dotnet pack`
- `R CMD build`
- `julia --project -e "using Pkg; Pkg.test()"`
- `go test ./...`
## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, phase-closeout ledger evidence, cleaned commit/push evidence, and blocker recording. At actual closeout, run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.