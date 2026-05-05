# Manufacturing Bottleneck Starter Kit

Maturity: `domain-preview`

This starter kit shows how a manufacturing user should start from a concrete KairoECS example and adapt it into a bottleneck-analysis model.

## Linked model-zoo example

- Model-zoo id: `factory_bottleneck`
- Example path: `../../des/factory_bottleneck`
- Example README: `../../des/factory_bottleneck/README.md`

## Dependency list

- Rust workspace checkout for local example validation.
- KairoECS DES resource and queue APIs from Track 03.
- Optional telemetry export from the Arrow track when throughput and wait-time outputs are promoted beyond tutorial-only status.

## Tutorial path

1. Open the linked model README: `../../des/factory_bottleneck/README.md`.
2. Review the model-zoo walkthrough notebook:
   `../../model-zoo/notebooks/model-zoo-tour.ipynb`.
3. Use the shared figure in `../../model-zoo/figures/model-zoo-flow.svg` to
   explain the documentation flow to new contributors.
4. Keep this starter kit at `domain-preview` until scenario fixtures and output
   artifacts are checked in.

## Expected outputs

| Output | Meaning | Current evidence boundary |
|---|---|---|
| `throughput` | Completed jobs per scenario window. | Named by the linked model-zoo entry; no checked release artifact. |
| `wait_time` | Waiting time at the constrained station. | Tutorial metric only until a scenario fixture exists. |
| `work_in_process` | Jobs queued or running across stations. | Tutorial metric only until telemetry is checked in. |

## Validation commands

Run from the repository root:

```powershell
pwsh -NoProfile -File examples/model-zoo/validate-inventory.ps1
```

The validator proves the starter-kit record, linked model-zoo id, example path,
notebook, figure, maturity label, dependency list, expected outputs, and
validation-command section are present.

## Starter-kit inventory contract

This README is listed in `../starter-kits.yaml`. The inventory validator checks that the kit path, this README, and every linked example path exists before the kit is treated as public-discoverable.

## Adaptation checklist

1. Start from `../../des/factory_bottleneck/README.md`.
2. Replace tutorial assumptions with station, cycle-time, buffer, and shift-calendar inputs.
3. Record the scenario file and expected outputs next to the example before raising the maturity label.
4. Keep the model-zoo entry and starter-kit manifest in the same change.
