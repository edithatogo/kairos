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
- Optional telemetry export from the Arrow track when throughput and wait-time outputs are promoted beyond placeholder status.

## Starter-kit inventory contract

This README is listed in `../starter-kits.yaml`. The inventory validator checks that the kit path, this README, and every linked example path exists before the kit is treated as public-discoverable.

## Adaptation checklist

1. Start from `../../des/factory_bottleneck/README.md`.
2. Replace placeholder assumptions with station, cycle-time, buffer, and shift-calendar inputs.
3. Record the scenario file and expected outputs next to the example before raising the maturity label.
4. Keep the model-zoo entry and starter-kit manifest in the same change.
