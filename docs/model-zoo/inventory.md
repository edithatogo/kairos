# Model Zoo Inventory

The model-zoo inventory is the source used by docs and starter kits to link
users to real example paths, tutorial assets, maturity labels, and expected
outputs.

## Documentation system

The current model-zoo documentation system has four synchronized surfaces:

| Surface | Role |
|---|---|
| `examples/model-zoo/model-zoo.yaml` | Machine-readable source for model ids, maturity labels, paths, expected outputs, notebooks, and figures. |
| `examples/*/README.md` | Tutorial entry point for each concrete example path. |
| `docs/model-zoo/inventory.md` | Human-readable inventory and validation contract. |
| `docs/starter-kits/README.md` | Domain grouping layer for users starting from a problem domain. |

Every public model-zoo example should be understandable from its README without
running code, and every stronger runtime claim should have a validation command
or artifact named beside it.

## Local validation

Run:

```powershell
pwsh -NoProfile -File examples/model-zoo/validate-inventory.ps1
```

The validator checks:

- every model-zoo `path` exists;
- every model-zoo `docs` path exists;
- every model-zoo `tutorial` notebook exists when listed;
- every model-zoo `figure` exists when listed;
- every model-zoo README includes a maturity label, expected outputs, and validation commands;
- every starter-kit `kit_path` and `docs` path exists;
- starter-kit READMEs include a maturity label and dependency list;
- starter-kit `model_zoo_ids` resolve to model-zoo entries;
- starter-kit `example_paths` exist.

## Tutorial assets

| Asset | Purpose |
|---|---|
| `examples/model-zoo/notebooks/model-zoo-tour.ipynb` | Jupyter walkthrough for reading maturity labels, outputs, and starter-kit links without requiring a runtime backend. |
| `examples/model-zoo/figures/model-zoo-flow.svg` | Reusable figure showing the inventory-to-example-to-output documentation path. |

## Current model-zoo entries

| Model-zoo id | Maturity | Expected outputs | Tutorial asset |
|---|---|---|---|
| `mm1_queue` | `benchmark` | `event_trace`, `queue_length`, `utilization` | `examples/model-zoo/notebooks/model-zoo-tour.ipynb` |
| `factory_bottleneck` | `reference` | `throughput`, `wait_time`, `work_in_process` | `examples/model-zoo/notebooks/model-zoo-tour.ipynb` |
| `flocking` | `reference` | `agent_positions`, `cohesion`, `separation` | `examples/model-zoo/notebooks/model-zoo-tour.ipynb` |
| `emergency_department_flow` | `domain-preview` | `length_of_stay`, `wait_time`, `resource_utilization` | `examples/model-zoo/notebooks/model-zoo-tour.ipynb` |

## Current concrete starter-kit link

| Starter kit | Model-zoo id | Real example path |
|---|---|---|
| `manufacturing-bottleneck` | `factory_bottleneck` | `examples/des/factory_bottleneck` |
