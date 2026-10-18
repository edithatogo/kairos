---
title: R
description: R 4.4+ package bindings via extendr and C ABI with Arrow data.frame integration.
---

The R package (`bindings/r/`) exposes KairoECS to statistical and epidemiological workflows in R 4.4+ via `extendr` and the stable `kairo-ecs-ffi` C ABI.

## R Package Capabilities

| Function | Purpose | Return Type |
| --- | --- | --- |
| `kairo_sim_create(seed)` | Initialize a deterministic KairoECS simulation handle | External pointer (`SEXP`) |
| `kairo_sim_step(handle, steps)` | Advance simulation by `steps` discrete events or ticks | Integer (`SimTime`) |
| `kairo_sim_snapshot(handle)` | Capture a deterministic replay snapshot | Raw vector / snapshot ID |
| `kairo_sim_export_arrow(handle)` | Export entity component columns via Arrow C Data Interface | `arrow::RecordBatch` / `data.frame` |

## Example

```r
library(kairoecs)

sim <- kairo_sim_create(seed = 42L)
current_time <- kairo_sim_step(sim, steps = 100L)
df <- kairo_sim_export_arrow(sim)
head(df)
```

Canonical sources:

- `bindings/r/README.md`
- `docs/tutorials/r-getting-started.md`
