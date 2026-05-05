# M/M/1 Queue

Maturity: `benchmark`

This example is the smallest queueing model in the model zoo. Use it when you
want to learn the deterministic DES trace shape before moving to larger domain
examples.

## Tutorial path

1. Read the model contract in this README.
2. Inspect the model-zoo walkthrough notebook:
   `../../model-zoo/notebooks/model-zoo-tour.ipynb`.
3. Use the shared model-zoo flow figure to explain the path from inventory to
   expected outputs: `../../model-zoo/figures/model-zoo-flow.svg`.
4. Run the local inventory validation command below before linking this example
   from release notes or tutorials.

## Model assumptions

- Single arrival stream.
- Single server.
- First-in, first-out service discipline.
- Deterministic replay is more important than stochastic distribution fitting
  at the current maturity level.

## Expected outputs

| Output | Meaning | Current evidence boundary |
|---|---|---|
| `event_trace` | Ordered arrival, service-start, and departure events. | Declared in `examples/model-zoo/model-zoo.yaml`; local path validation only. |
| `queue_length` | Queue length over simulated time. | Expected analysis output; no checked-in Arrow trace yet. |
| `utilization` | Server busy fraction for the scenario window. | Expected analysis output; no release benchmark artifact yet. |

## Validation commands

Run from the repository root:

```powershell
pwsh -NoProfile -File examples/model-zoo/validate-inventory.ps1
```

The command proves the model-zoo entry, README path, tutorial notebook, and
shared figure exist. It does not execute a queue simulation.

## Promotion checklist

- Add a scenario file with seeded arrival and service parameters.
- Add a checked expected trace or conformance fixture.
- Add an executable analysis script or notebook cell that computes the expected
  output table.
