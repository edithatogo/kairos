---
title: Julia
description: Julia 1.10+ ccall bindings and Tables.jl/Arrow.jl zero-copy integration.
---

The Julia package (`bindings/julia/KairoECS.jl`) wraps `libkairo_ecs_ffi` via zero-overhead `ccall` declarations and integrates directly with `Arrow.jl` and `Tables.jl`.

## Julia Surface Overview

| Symbol | Purpose |
| --- | --- |
| `KairoSimulation(seed::UInt64)` | Construct a managed simulation instance with finalizer cleanup |
| `step!(sim::KairoSimulation, n::Integer)` | Advance simulation by `n` steps deterministically |
| `snapshot(sim::KairoSimulation)` | Capture state digest and serialized checkpoint |
| `export_arrow_batch(sim::KairoSimulation)` | Zero-copy export of SoA columns via `FFI_ArrowArray` |

## Example

```julia
using KairoECS

sim = KairoSimulation(UInt64(42))
step!(sim, 250)
digest = snapshot(sim)
println("Simulation time: ", sim.time, " digest: ", digest)
```

Canonical sources:

- `bindings/julia/README.md`
- `docs/tutorials/julia-getting-started.md`
