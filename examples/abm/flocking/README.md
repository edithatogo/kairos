# Flocking

Maturity: `reference`

This example is the model-zoo entry for agent-based behavior. It is intended to
teach how KairoECS examples describe agents, observations, and expected outputs
without implying a finalized ABM physics contract.

## Tutorial path

1. Read the assumptions and expected outputs below.
2. Compare the ABM documentation flow with the DES entries in
   `../../model-zoo/notebooks/model-zoo-tour.ipynb`.
3. Use `../../model-zoo/figures/model-zoo-flow.svg` when documenting how this
   entry connects to the public catalog.
4. Run the inventory validator before exposing new ABM links.

## Model assumptions

- Agents have positions and local-neighbor behavior.
- Cohesion and separation are reported as tutorial metrics.
- The current documentation is a reference contract for future executable
  fixtures, not evidence of a calibrated biological or physical model.

## Expected outputs

| Output | Meaning | Current evidence boundary |
|---|---|---|
| `agent_positions` | Per-step agent positions for replay or visualization. | Declared in inventory; no checked-in trajectory artifact yet. |
| `cohesion` | Summary metric for group alignment. | Expected tutorial metric; no benchmark threshold yet. |
| `separation` | Summary metric for collision avoidance spacing. | Expected tutorial metric; no benchmark threshold yet. |

## Validation commands

Run from the repository root:

```powershell
pwsh -NoProfile -File examples/model-zoo/validate-inventory.ps1
```

The command checks catalog wiring and tutorial assets. It does not execute an
ABM simulation.

## Promotion checklist

- Add a seeded scenario file.
- Add a checked trajectory or compact expected-output fixture.
- Add visualization output once Track 24 has a stable public path for examples.
