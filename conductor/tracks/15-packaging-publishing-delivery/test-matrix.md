# Test Matrix — 15 Packaging, Publishing & Delivery

## Required checks

- Package matrix coverage for Rust, Python, R, Julia, TypeScript, C#, and Go.
- Registry plan coverage for each ecosystem's first target and fallback.
- Dry-run coverage for every ecosystem that supports packaging locally.
- Docs coverage for any change to package naming, registry order, or release policy.
- No-production-publish check: the track must not introduce live publish commands.

## Track-specific commands

```bash
rg -n "Rust|Python|R|Julia|TypeScript|C#|Go" conductor/package-matrix.md conductor/package-catalog.md conductor/release-engineering.md
rg -n "dry-run|draft only|preview|reservation|fallback" conductor/package-matrix.md conductor/release-engineering.md
```

## Registry checks to land later

- `cargo publish --dry-run`
- `python -m build`
- `twine check`
- `npm pack`
- `dotnet pack`
- `R CMD build`
- `julia --project -e "using Pkg; Pkg.test()"`
- `go test ./...`
