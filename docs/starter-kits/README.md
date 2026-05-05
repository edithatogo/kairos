# Starter Kits

Starter kits group model-zoo examples into domain-oriented starting points. They are discovery artifacts first: a kit should point to real example paths and state its maturity before it promises runtime support.

## Inventory source

- `examples/starter-kits/starter-kits.yaml`
- `examples/model-zoo/model-zoo.yaml`

## Current starter kits

| Kit | Domain | Maturity | Starter README | Linked example |
|---|---|---|---|---|
| Manufacturing bottleneck starter kit | manufacturing | domain-preview | `examples/starter-kits/manufacturing/README.md` | `examples/des/factory_bottleneck` |

## Acceptance rule

A starter kit is discoverable only when:

- its `kit_path` exists;
- its README exists and includes a maturity label and dependency list;
- each `example_paths` entry exists;
- every `model_zoo_ids` entry is present in `examples/model-zoo/model-zoo.yaml`.

Run `pwsh -NoProfile -File examples/model-zoo/validate-inventory.ps1` before publishing starter-kit docs.
