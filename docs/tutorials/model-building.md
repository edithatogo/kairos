# Model-Building Tutorial

This tutorial gives a conservative path for turning a domain question into a
KairoECS model using the checked-in example inventory.

## 1. Choose the model shape

Start with the [community model zoo](../community/model-zoo.md) and pick the
closest paradigm:

| Paradigm | Start with | Use when |
|---|---|---|
| DES | [M/M/1 queue](../../examples/des/mm1_queue/README.md) | Events, queues, resources, and timestamps drive the model. |
| DES resource flow | [Factory bottleneck](../../examples/des/factory_bottleneck/README.md) | Throughput, wait time, WIP, and bottleneck behavior matter. |
| ABM | [Flocking](../../examples/abm/flocking/README.md) | Entity behavior and local interaction rules drive the model. |
| Hybrid | [Emergency department flow](../../examples/hybrid/emergency_department_flow/README.md) | Domain flow mixes events, entities, resources, and policy choices. |

Use the maturity label to set expectations before writing code. A
`domain-preview` example is useful for framing, not for production claims.

## 2. Write the model card

Before implementation, write a short model card next to the example or in your
own working branch:

- domain question;
- entities and resources;
- events and event ordering assumptions;
- input parameters;
- outputs and acceptance checks;
- random seeds or deterministic replay policy;
- known exclusions;
- maturity label.

Cross-check the evidence wording against:

- [Replay and seeds](../trustworthy-simulation/replay-and-seeds.md)
- [Verification, validation, and uncertainty](../trustworthy-simulation/verification-validation-uncertainty.md)
- [Scenario evidence](../trustworthy-simulation/scenario-evidence.md)

## 3. Map events and entities

For a first DES model:

1. Name the event kinds.
2. Decide the tick scale.
3. Define the initial scheduled events.
4. Define what each dispatched event schedules next.
5. Record any cancellation rules.
6. Decide which outputs come from event logs, snapshots, or summary metrics.

For an ABM or hybrid model, add the behavior-update cadence and state variables
for each entity type.

## 4. Choose the first implementation surface

| Surface | Use when | Current boundary |
|---|---|---|
| Rust | You want to inspect the canonical scheduler and workspace crates. | Source-backed API review and compile checks. |
| Python | You want a small local learning loop. | Python-native facade, not native wheels. |
| TypeScript/Wasm | You want browser-shaped data and event-log rows. | TypeScript facade; native Wasm loading is `not-configured` until a loader exists. |

## 5. Validate the tutorial path

Run the tutorial validator after editing tutorial or example links:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File docs/tutorials/validate-tutorials.ps1
```

Then run the language-specific checks for the surface you used.

## Claim boundary

This page is a model-building guide. It does not claim that examples include
finished notebooks, generated figures, registry packages, or production-ready
domain validation. Those artifacts should be added and gated by the owning
documentation, packaging, and validation tracks.
