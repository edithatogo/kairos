# Model Zoo

The model zoo is the discovery layer for KairoECS examples. It should help a new user move from the docs home page to a concrete, runnable example without guessing which directory matters.

## Source of truth

- `examples/model-zoo/model-zoo.yaml`
- `examples/model-zoo/README.md`

## Maturity labels

- `toy`
- `reference`
- `validated`
- `benchmark`
- `domain-preview`

## Current inventory

| Model | Paradigm | Maturity | Entry point |
|---|---|---|---|
| M/M/1 queue | DES | benchmark | `examples/des/mm1_queue/README.md` |
| Factory bottleneck | DES | reference | `examples/des/factory_bottleneck/README.md` |
| Flocking | ABM | reference | `examples/abm/flocking/README.md` |
| Schelling segregation | ABM | reference | `examples/abm/schelling/README.md` |
| Emergency department flow | Hybrid | domain-preview | `examples/hybrid/emergency_department_flow/README.md` |
| Supply chain disruption | Hybrid | reference | `examples/hybrid/supply_chain_disruption/README.md` |
| Queue control Gymnasium | RL | domain-preview | `examples/rl/queue_control_gymnasium/README.md` |

## How to use it

1. Open `examples/model-zoo/README.md`.
2. Pick a model by paradigm and maturity.
3. Open the example README at the listed entry point.
4. Use the example README to locate the runnable command or local smoke check.

## Claim boundary

The model zoo is a discovery and inventory surface. It does not promise that every example is production-ready, supported, or stable. Maturity labels must match the example README and the release gate language.
