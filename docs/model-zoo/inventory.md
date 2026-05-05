# Model Zoo Inventory

The model-zoo inventory is the source used by docs and starter kits to link users to real example paths.

## Local validation

Run:

```powershell
pwsh -NoProfile -File examples/model-zoo/validate-inventory.ps1
```

The validator checks:

- every model-zoo `path` exists;
- every model-zoo `docs` path exists;
- every starter-kit `kit_path` and `docs` path exists;
- starter-kit READMEs include a maturity label and dependency list;
- starter-kit `model_zoo_ids` resolve to model-zoo entries;
- starter-kit `example_paths` exist.

## Current concrete starter-kit link

| Starter kit | Model-zoo id | Real example path |
|---|---|---|
| `manufacturing-bottleneck` | `factory_bottleneck` | `examples/des/factory_bottleneck` |
