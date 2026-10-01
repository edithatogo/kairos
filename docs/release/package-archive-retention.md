# Package archive retention

`Package Dry Runs` builds archive files without contacting or publishing to a
package registry. Each successful ecosystem job retains its output briefly for
diagnosis; the dependent verification job checks that every archive can be read,
rejects unsafe archive paths and links, writes `ARCHIVE-INDEX.json` and
`SHA256SUMS`, then retains the combined tree as
`kairos-actual-package-archives-<commit>` for 90 days.

The workflow currently builds:

| Ecosystem | Retained file | Archive kind |
| --- | --- | --- |
| Rust | `.crate` for `kairo-ecs-types` | crates.io package archive |
| Python | source distribution and pure Python wheel | Python distributions |
| R | `kairoECS` source tarball | R package source archive |
| TypeScript | `.tgz` | npm package archive |
| C# | `Kairo.ECS` `.nupkg` for .NET 10 | NuGet package archive |
| Julia | `KairoECS` source tarball | Git source archive; `Pkg.test()` remains in binding CI |
| Go | Go binding source tarball | Git source archive; `go vet` and `go test` remain in binding CI |

Julia and Go do not currently have a tagged registry package to archive, so their
retained files are explicitly source archives. The Rust artifact is the
standalone types crate; workspace packaging remains blocked by internal path
dependencies that lack registry version constraints. These outputs do not
represent a coordinated release set or authorize publication.

The repository can also build a local bundle from the workflow's package output
directories and validate its checksums:

```sh
python3 packaging/scripts/build_package_archive_bundle.py \
  --input dist/packages \
  --output dist/retained-package-archives \
  --source-commit <full-40-character-commit-sha>
```

The command requires one archive directory per ecosystem (`rust`, `python`,
`r`, `julia`, `typescript`, `nuget`, and `go`). It validates archive structure,
records byte counts and SHA-256 values, then verifies the resulting index and
checksum file against the retained bytes. The local 2026-10-01 rehearsal used
source commit `738f2206f7a59e338c169d771faae7b73cd6372b`; its separately
retained bundle is outside Git.

This is an archive-integrity and retention step. It does not assert full
workspace Rust package readiness, native platform builds, SBOM/provenance,
registry acceptance, or release approval.
