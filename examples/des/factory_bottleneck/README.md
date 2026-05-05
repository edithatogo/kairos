# Factory Bottleneck

Maturity: `reference`

This example is the canonical manufacturing starter-kit model. It is suitable
for explaining a resource bottleneck and the documentation path from model-zoo
inventory to a domain starter kit.

## Tutorial path

1. Start here to understand the model scope.
2. Open `../../starter-kits/manufacturing/README.md` for the domain adaptation
   checklist.
3. Open `../../model-zoo/notebooks/model-zoo-tour.ipynb` for the catalog-level
   walkthrough.
4. Use `../../model-zoo/figures/model-zoo-flow.svg` in docs or slides when
   explaining how a user finds this example.

## Model assumptions

- Work enters a small station network.
- One constrained station dominates throughput.
- Buffers and work-in-process are tracked as discovery outputs.
- The README documents the intended tutorial contract; it does not claim a
  production manufacturing optimizer.

## Expected outputs

| Output | Meaning | Current evidence boundary |
|---|---|---|
| `throughput` | Completed jobs per scenario window. | Declared in `examples/model-zoo/model-zoo.yaml`; no release artifact yet. |
| `wait_time` | Waiting time at the constrained station. | Expected starter-kit metric; tutorial documentation only. |
| `work_in_process` | Jobs queued or being processed across stations. | Expected starter-kit metric; no checked-in telemetry file yet. |

## Validation commands

Run from the repository root:

```powershell
pwsh -NoProfile -File examples/model-zoo/validate-inventory.ps1
```

The command proves this README, the manufacturing starter kit, the shared
notebook, and the shared figure are reachable from the inventories.

## Promotion checklist

- Add a concrete station/cycle-time scenario.
- Add expected throughput, wait-time, and WIP outputs.
- Add a notebook cell or script that renders the starter-kit result table.
