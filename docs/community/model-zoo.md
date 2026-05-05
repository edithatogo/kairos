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
| Emergency department flow | Hybrid | domain-preview | `examples/hybrid/emergency_department_flow/README.md` |

## Examples map

| User goal | Recommended start | Why |
|---|---|---|
| Learn deterministic queues | M/M/1 queue | Small DES model with benchmark-oriented outputs. |
| Inspect a resource bottleneck | Factory bottleneck | Reference DES model with throughput, wait-time, and WIP outputs. |
| Learn agent behavior | Flocking behavior | Reference ABM model with position and cohesion outputs. |
| Explore mixed patient-flow concepts | Emergency department flow | Hybrid domain-preview model with length-of-stay and utilization outputs. |

## How to use it

1. Open `examples/model-zoo/README.md`.
2. Pick a model by paradigm and maturity.
3. Open the example README at the listed entry point.
4. Use the example README to locate the runnable command or local smoke check.

## Claim boundary

The model zoo is a discovery and inventory surface. It does not promise that every example is production-ready, supported, or stable. Maturity labels must match the example README and the release gate language.

## Inventory update rule

When adding or removing a community-facing example, update all three surfaces in the same PR:

- `examples/model-zoo/model-zoo.yaml`
- `examples/model-zoo/README.md`
- `docs/community/model-zoo.md`

The Track 17 `onboarding-docs` gate fails if a model appears in the docs table but not in the YAML inventory, or if a YAML path does not have a README.
