# Handoff: Track 23 Domain Starter Kits & Model Zoo

## Summary

Captured the discovery path for starter kits and model-zoo entries so later tracks can point to concrete example locations. This pass adds the first concrete starter-kit inventory record and a local validator that checks model-zoo and starter-kit links.

## Files changed

`docs/model-zoo/inventory.md`, `docs/starter-kits/README.md`, `examples/model-zoo/README.md`, `examples/model-zoo/validate-inventory.ps1`, `examples/starter-kits/starter-kits.yaml`, `examples/starter-kits/manufacturing/README.md`, `conductor/tracks/23-domain-starter-kits-model-zoo/test-matrix.md`, `conductor/tracks/23-domain-starter-kits-model-zoo/risk-register.md`, `conductor/tracks/23-domain-starter-kits-model-zoo/handoff.md`

## Contracts consumed

`docs/community/`, `examples/model-zoo/`, `conductor/package-catalog.md`, `conductor/delivery-readiness-checklist.md`, `conductor/workflow.md`

## Release gates affected

Starter-kit discoverability and model-zoo link integrity now sit on the public-docs path.

## Validation evidence

| Command | Result |
|---|---|
| `pwsh -NoProfile -File examples/model-zoo/validate-inventory.ps1` | Pass: validates 4 model-zoo entries and 1 starter-kit entry. |

## Risks and unresolved questions

The main risk is publishing an example index that points at kits which are not actually runnable yet. The new validator mitigates link and inventory drift, but runtime execution remains blocked until Track 03 stabilizes the DES API used by the placeholder `factory_bottleneck` example.
