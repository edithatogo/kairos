# Julia Getting Started

## Prerequisites
- Julia >= 1.10 installed
- The KairoECS repository cloned

## Quickstart

### 1. Activate and install
```julia
using Pkg
Pkg.activate("bindings/julia")
Pkg.instantiate()
```

### 2. Verify the package surface
```julia
using KairoECS
KairoECS.version_string()
KairoECS.self_check()
```

### 3. Run tests
```julia
using Pkg
Pkg.test("KairoECS")
```

## Package structure

| File | Purpose |
|---|---|
| `src/KairoECS.jl` | Module source (version, self-check, Arrow field definitions) |
| `test/runtests.jl` | Test suite |
| `Project.toml` | Package manifest |

## Dependencies

- `Arrow.jl` — Arrow IPC file reading for telemetry
- `JSON3.jl` — JSON parsing for conformance input/output

## Next steps

- Read the [Julia binding README](../../bindings/julia/README.md)
- Explore [Arrow schema reference](../arrow/schema-reference.md)
