# KairoECS Model Zoo

This directory is the inventory bridge between the public docs and the example directories.

## Start here

- `../../docs/community/model-zoo.md`
- `../../docs/model-zoo/inventory.md`
- `../../docs/starter-kits/README.md`
- `model-zoo.yaml`
- `../starter-kits/starter-kits.yaml`

## Maturity labels

| Label | Meaning |
|---|---|
| `toy` | Small teaching artifact; no broad compatibility promise. |
| `reference` | Recommended example shape for a paradigm or domain slice. |
| `validated` | Has checked validation fixtures or result artifacts. |
| `benchmark` | Used for benchmark or regression documentation. |
| `domain-preview` | Domain walkthrough with bounded, non-production claims. |

Do not raise a label unless the README names the command or artifact that proves
the stronger claim.

## Entry points

The YAML inventory points at the concrete example readmes:

- `../des/mm1_queue/README.md`
- `../des/factory_bottleneck/README.md`
- `../abm/flocking/README.md`
- `../hybrid/emergency_department_flow/README.md`

The inventory and the docs page must agree on the maturity label and the directory path.

Each README must include:

- `Maturity:`
- `## Tutorial path`
- `## Expected outputs`
- `## Validation commands`

## Tutorial assets

- Notebook: `notebooks/model-zoo-tour.ipynb`
- Figure: `figures/model-zoo-flow.svg`

The notebook is intentionally documentation-first: it teaches users how to read
the catalog, maturity labels, and output contracts without requiring a native
runtime backend. Add executable cells only when the corresponding example has a
checked scenario and expected-output artifact.

## Starter-kit bridge

The current concrete starter-kit slice is `../starter-kits/manufacturing/README.md`, which links the `factory_bottleneck` model-zoo entry to `../des/factory_bottleneck`.

Run `pwsh -NoProfile -File validate-inventory.ps1` from this directory, or `pwsh -NoProfile -File examples/model-zoo/validate-inventory.ps1` from the repo root, before publishing starter-kit or model-zoo updates.

## Community adoption map

| Model id | Best first audience | Follow-up docs |
|---|---|---|
| `mm1_queue` | Users learning deterministic DES traces. | `../../docs/trustworthy-simulation/replay-and-seeds.md` |
| `factory_bottleneck` | Users comparing resource and queue behavior. | `../../docs/benchmarks/benchmark-policy.md` |
| `flocking` | Users learning ABM behavior contracts. | `../../docs/community/playground.md` |
| `emergency_department_flow` | Users evaluating hybrid domain previews. | `../../docs/trustworthy-simulation/verification-validation-uncertainty.md` |

## Update checklist

- Add the example directory and README first.
- Add the inventory record to `model-zoo.yaml`.
- Add or link a tutorial notebook and figure.
- Add or update starter-kit records in `../starter-kits/starter-kits.yaml` when a kit points at the model.
- Add or update the public row in `../../docs/community/model-zoo.md`.
- Keep maturity labels conservative until a gate proves the stronger claim.
