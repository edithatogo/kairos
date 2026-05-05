# Starter Kits

Starter kits group model-zoo examples into domain-oriented starting points. They
are discovery artifacts first: a kit should point to real example paths, state
its maturity, name expected outputs, and provide validation commands before it
promises runtime support.

## Inventory source

- `examples/starter-kits/starter-kits.yaml`
- `examples/model-zoo/model-zoo.yaml`

## Current starter kits

| Kit | Domain | Maturity | Starter README | Linked example | Expected outputs |
|---|---|---|---|---|---|
| Manufacturing bottleneck starter kit | manufacturing | domain-preview | `examples/starter-kits/manufacturing/README.md` | `examples/des/factory_bottleneck` | `throughput`, `wait_time`, `work_in_process` |

## Tutorial assets

- Notebook: `examples/model-zoo/notebooks/model-zoo-tour.ipynb`
- Figure: `examples/model-zoo/figures/model-zoo-flow.svg`

These assets are shared by the starter-kit docs so the first tutorial path is
consistent with the model-zoo inventory.

## Acceptance rule

A starter kit is discoverable only when:

- its `kit_path` exists;
- its README exists and includes a maturity label and dependency list;
- its README names expected outputs and validation commands;
- each `example_paths` entry exists;
- every `model_zoo_ids` entry is present in `examples/model-zoo/model-zoo.yaml`.

Run `pwsh -NoProfile -File examples/model-zoo/validate-inventory.ps1` before publishing starter-kit docs.
