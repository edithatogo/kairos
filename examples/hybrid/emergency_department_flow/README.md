# Emergency Department Flow

Maturity: `domain-preview`

This example documents the shape of a hybrid health-flow model. It is useful for
discussion and tutorial planning, but it remains a domain preview until
scenario fixtures and validation outputs are checked in.

## Tutorial path

1. Use this README to understand the claim boundary.
2. Open `../../model-zoo/notebooks/model-zoo-tour.ipynb` for the model-zoo
   walkthrough.
3. Use `../../model-zoo/figures/model-zoo-flow.svg` to show how a domain-preview
   example differs from a benchmark or reference entry.
4. Run the inventory validator before linking this preview from docs pages.

## Model assumptions

- Patient arrivals, service resources, and disposition paths are represented at
  a teaching level.
- DES flow and state transitions may be combined with higher-level domain state.
- The current docs do not claim clinical validation, operational readiness, or
  patient-level fidelity.

## Expected outputs

| Output | Meaning | Current evidence boundary |
|---|---|---|
| `length_of_stay` | Time from arrival to disposition. | Declared in inventory; no clinical validation artifact. |
| `wait_time` | Waiting time before service stages. | Expected tutorial output; no calibrated scenario yet. |
| `resource_utilization` | Utilization of modeled service resources. | Expected tutorial output; no production dashboard yet. |

## Validation commands

Run from the repository root:

```powershell
pwsh -NoProfile -File examples/model-zoo/validate-inventory.ps1
```

The command checks catalog wiring and tutorial assets. It does not execute or
validate a clinical simulation.

## Promotion checklist

- Add a synthetic scenario with no patient-identifiable data.
- Add expected output tables for length of stay, wait time, and utilization.
- Add a validation note that separates tutorial evidence from clinical evidence.
