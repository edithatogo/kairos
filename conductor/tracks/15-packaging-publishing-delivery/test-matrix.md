# Test Matrix — 15 Packaging, Publishing & Delivery

## Required checks

- Package matrix coverage for Rust, Python, R, Julia, TypeScript, C#, and Go.
- Registry plan coverage for each ecosystem's first target and fallback.
- Dry-run coverage for every ecosystem that supports packaging locally.
- Docs coverage for any change to package naming, registry order, or release policy.
- Release-delivery gate coverage for the workflow step that runs before artifact upload.
- Required release evidence is fail-closed: a valid SPDX 2.3 SBOM, provenance covering all release-manifest SHA-256 subjects, and matching entries in `SUPPLY-CHAIN-SHA256SUMS` must exist before artifact upload.
- Archive retention coverage: the seven package jobs upload their actual native archive or explicitly labelled source archive; the aggregator validates archive members, matches each archive to its command/toolchain receipt, writes and verifies SHA-256 sums, and uploads the combined archive tree for 90 days.
- No-production-publish check: the track must not introduce live publish commands.
- No-publish-manifest check: the offline sequence must not generate publication manifests. The existing Track42 configuration is permitted only with reviewed dry-run/approval defaults; unexpected manifests remain rejected.
- Aggregate Track 12-20 evidence check: the package manifest remains dry-run only and wired into conformance CI.

## Track-specific commands

```bash
rg -n "Rust|Python|R|Julia|TypeScript|C#|Go" conductor/package-matrix.md conductor/package-catalog.md conductor/release-engineering.md
rg -n "dry-run|draft only|preview|reservation|fallback" conductor/package-matrix.md conductor/release-engineering.md
python packaging/scripts/build_release_manifest.py --check
python packaging/scripts/build_release_manifest.py --version 0.0.0-r2-dry-run
python packaging/scripts/build_release_manifest.py --verify-existing
powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/15-packaging-publishing-delivery/validate-packaging-dry-run.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/validate_track15_release_delivery.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tests/test_track15_release_evidence_gate.ps1
node tests/conformance/track12_20_evidence_check.mjs
```

## R2 dry-run matrix

| Ecosystem | Manifest inventory | Registry target | Dry-run evidence |
|---|---|---|---|
| Rust crates | `Cargo.toml` and `crates/*/Cargo.toml` listed in `packaging/release-package-manifest.json` | crates.io | `cargo package --allow-dirty -p kairo-ecs-types`; broader workspace package remains blocked by path dependency versions |
| Python binding | `bindings/python/pyproject.toml` | TestPyPI | hashed build-tool install; `python -m build --no-isolation`; `twine check` |
| R binding | `bindings/r/DESCRIPTION` | R-universe first | `R CMD build .`; `R CMD check --no-manual --no-tests *.tar.gz` |
| Julia binding | `bindings/julia/Project.toml` | Julia dev registry first | commit-bound source archive; `Pkg.test()` remains in required binding CI |
| TypeScript/Wasm binding | `bindings/typescript/package.json` | npm | `npm ci`; `npm run build`; `npm pack --ignore-scripts` |
| C# binding | `bindings/csharp/src/Kairo.ECS/Kairo.ECS.csproj` and `bindings/csharp/Kairo.ECS.sln` | NuGet | locked .NET 10 restore; `dotnet pack` |
| Go binding | `bindings/go/go.mod` | Go module proxy | commit-bound source archive; `go vet ./...` and `go test ./...` remain in required binding CI |

The manifest/checksum builder is the local validation gate for this slice. It
does not execute registry commands; it verifies that the package inventory,
registry modes, and checksum evidence can be generated before publishing is
enabled.

## Focused offline validator

`validate-packaging-dry-run.ps1` verifies the seven ecosystem surfaces, dry-run
release stage, disabled production publishing flag, fallback entries, manifest
paths, expected release evidence output paths, the ordered local dry-run
sequence, and the absence of publish manifest files.

`scripts/validate_track15_release_delivery.ps1` reuses the packaging dry-run
validator and fails if the release tree lacks a populated SPDX 2.3 SBOM,
provenance whose subjects cover the generated release manifest, or a checksum
list matching those evidence files. The release workflow runs this gate after
manifest verification and before `Upload artifacts`. The regression test
asserts failure for missing evidence, checksum drift, and incomplete provenance
subjects, plus success for a complete matching evidence set.

## First local sequence

1. Validate inventory without writes:
   `python packaging/scripts/build_release_manifest.py --check`
2. Generate local release evidence only:
   `python packaging/scripts/build_release_manifest.py --version 0.0.0-r2-dry-run`
3. Verify the generated evidence still matches the inventory and checksums:
   `python packaging/scripts/build_release_manifest.py --verify-existing`
4. Re-run the offline gate:
   `powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/15-packaging-publishing-delivery/validate-packaging-dry-run.ps1`

This sequence is the first local registry/package dry-run. It does not contact
registries, use credentials, upload artifacts, publish packages, or create
publish manifests.

## Registry checks to land later

- `cargo publish --dry-run`
- Registry-native dry-runs remain gated on naming reservations, registry readiness, release set selection, and publication review. Package archive generation and checksum retention do not satisfy registry acceptance.
## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, phase-closeout ledger evidence, cleaned commit/push evidence, and blocker recording. At actual closeout, run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.

## Release preparation policy regression

`python3 conductor/tracks/15-packaging-publishing-delivery/test-dry-run-policy.py` covers existing gated Track42 configuration, rejection of enabled production defaults, and rejection of unexpected generated publication manifests. This validates offline policy structure, not public release readiness or SBOM/provenance availability.
