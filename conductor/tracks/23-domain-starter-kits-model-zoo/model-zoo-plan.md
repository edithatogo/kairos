# Domain Starter Kits & Model Zoo Plan

## Principle

Domain examples should be templates and extension packages, not hard-coded into the kernel.

## Starter kits

| Starter kit | First examples |
|---|---|
| `kairo-ecs-health` | emergency department flow, bed flow, ambulance dispatch |
| `kairo-ecs-logistics` | warehouse pick/pack, supply chain disruption |
| `kairo-ecs-manufacturing` | factory bottleneck, rework loop, maintenance schedule |
| `kairo-ecs-transport` | airport security, bus headways, evacuation |
| `kairo-ecs-epidemics` | SEIR ABM, contact network, intervention scenarios |
| `kairo-ecs-rl` | queue control Gymnasium environment, dispatch policy learning |

## Model maturity labels

- `toy`: educational only
- `reference`: demonstrates a known pattern
- `validated`: has documented verification/validation inputs
- `benchmark`: used for performance/conformance
- `domain-preview`: useful but not authoritative
