# KairoECS Model Zoo

This directory is the inventory bridge between the public docs and the example directories.

## Start here

- `../../docs/community/model-zoo.md`
- `model-zoo.yaml`

## Maturity labels

- `toy`
- `reference`
- `validated`
- `benchmark`
- `domain-preview`

## Entry points

The YAML inventory points at the concrete example readmes:

- `../des/mm1_queue/README.md`
- `../des/factory_bottleneck/README.md`
- `../abm/flocking/README.md`
- `../hybrid/emergency_department_flow/README.md`

The inventory and the docs page must agree on the maturity label and the directory path.

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
- Add or update the public row in `../../docs/community/model-zoo.md`.
- Keep maturity labels conservative until a gate proves the stronger claim.
