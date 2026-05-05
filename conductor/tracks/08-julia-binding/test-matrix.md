# Test Matrix — 08 Julia Binding

## Required tests

- `julia --project -e 'using Pkg; Pkg.test()'` for Julia package coverage.
- `julia --project -e 'using Pkg; Pkg.instantiate()'` to verify environment resolution.
- `julia --project -e 'include("tests/conformance.jl")'` or equivalent when Track 12 fixtures are consumed.
- `julia --project -e 'using Pkg; Pkg.precompile()'` to catch package-load regressions early.
- `julia --project -e 'using Pkg; Pkg.build()'` only when package metadata is present.

## Future-surface controls

- Do not add General registry publishing, package server release automation, or credentials here.
- Do not expand into Python, R, TypeScript, C#, or Go surfaces.
- Do not widen beyond the Julia package and its conformance adapters.
- Stop at local package validation until Track 12 owns fixture parity and Track 15 owns dry-run release planning.

## CI command

```bash
julia --project -e 'using Pkg; Pkg.test()' && julia --project -e 'using Pkg; Pkg.instantiate()'
```

