# Julia Style Guide

- Use `ccall`/`Libdl` over the stable C ABI.
- Use Artifacts first; consider JLL only after ABI maturity.
- Test with `Test`, `Aqua.jl`, and optional `JET.jl`.
- Use `Documenter.jl` for API docs.
- Use Arrow.jl, Tables.jl, and DataFrames.jl for telemetry examples.
